use std::sync::Arc;
use std::io::{self, BufRead, Write};

use crate::config::Config;
use crate::domain::agent::AgentRepository;
use crate::domain::audit::AuditRepository;
use crate::domain::budget::BudgetRepository;
use crate::domain::execution::ExecutionPort;
use crate::domain::mcp::protocol::{
    JsonRpcRequest, JsonRpcResponse, ToolDef,
    METHOD_NOT_FOUND, INTERNAL_ERROR, INVALID_PARAMS,
};
use crate::domain::network::NetworkRepository;
use crate::interfaces::http::AppState;

pub struct BastionMcpServer {
    agents: Arc<dyn AgentRepository>,
    audit: Arc<dyn AuditRepository>,
    budget: Arc<dyn BudgetRepository>,
    executor: Arc<dyn ExecutionPort>,
    network: Arc<dyn NetworkRepository>,
    #[allow(dead_code)]
    config: Config,
}

impl BastionMcpServer {
    pub fn new(state: AppState) -> Self {
        Self {
            agents: state.agents,
            audit: state.audit,
            budget: state.budget,
            executor: state.executor,
            network: state.network,
            config: state.config,
        }
    }

    /// Public dispatch method (used by tests and the STDIO loop)
    pub async fn dispatch(&self, req: JsonRpcRequest) -> JsonRpcResponse {
        let id = req.id.clone();
        match req.method.as_str() {
            "initialize" => self.handle_initialize(id).await,
            "tools/list" => self.handle_tools_list(id).await,
            "tools/call" => self.handle_tools_call(id, req.params).await,
            _ => JsonRpcResponse::error(id, METHOD_NOT_FOUND, "Method not found"),
        }
    }

    /// Main STDIO loop — reads lines from stdin, writes JSON responses to stdout.
    pub async fn run(&self) -> anyhow::Result<()> {
        let stdin = io::stdin();
        let stdout = io::stdout();
        let mut out = stdout.lock();

        for line in stdin.lock().lines() {
            let line = match line {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("bastion-mcp: stdin error: {e}");
                    break;
                }
            };

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let response = match serde_json::from_str::<JsonRpcRequest>(trimmed) {
                Ok(req) => self.dispatch(req).await,
                Err(_) => JsonRpcResponse::error(None, -32700, "Parse error"),
            };

            let json = serde_json::to_string(&response)?;
            writeln!(out, "{}", json)?;
            out.flush()?;
        }

        Ok(())
    }

    // ───────────────────────────── handlers ──────────────────────────────

    async fn handle_initialize(&self, id: Option<serde_json::Value>) -> JsonRpcResponse {
        JsonRpcResponse::success(
            id,
            serde_json::json!({
                "protocolVersion": "2024-11-05",
                "serverInfo": {
                    "name": "bastion",
                    "version": "0.1.0"
                },
                "capabilities": {
                    "tools": {}
                }
            }),
        )
    }

    async fn handle_tools_list(&self, id: Option<serde_json::Value>) -> JsonRpcResponse {
        let tools: Vec<ToolDef> = vec![
            ToolDef {
                name: "bastion_exec".to_string(),
                description: "Execute a command inside the Bastion sandbox".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "command":    { "type": "string", "description": "Shell command to run" },
                        "profile":    { "type": "string", "description": "Sandbox profile" },
                        "agent_id":   { "type": "string", "description": "Agent ID for attribution" },
                        "timeout_ms": { "type": "integer", "description": "Timeout in milliseconds" }
                    },
                    "required": ["command"]
                }),
            },
            ToolDef {
                name: "bastion_list_agents".to_string(),
                description: "List all active agents registered in Bastion".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            ToolDef {
                name: "bastion_get_audit".to_string(),
                description: "Retrieve audit logs from Bastion".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "limit":  { "type": "integer", "description": "Max entries" },
                        "offset": { "type": "integer", "description": "Pagination offset" }
                    }
                }),
            },
            ToolDef {
                name: "bastion_get_budget".to_string(),
                description: "Get the current execution budget state".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            ToolDef {
                name: "bastion_network_status".to_string(),
                description: "Get the network proxy status and rules".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
            },
        ];

        JsonRpcResponse::success(id, serde_json::json!({ "tools": tools }))
    }

    async fn handle_tools_call(
        &self,
        id: Option<serde_json::Value>,
        params: serde_json::Value,
    ) -> JsonRpcResponse {
        let name = match params.get("name").and_then(|v| v.as_str()) {
            Some(n) => n.to_string(),
            None => {
                return JsonRpcResponse::error(id, INVALID_PARAMS, "Missing 'name' field");
            }
        };

        let args = params
            .get("arguments")
            .cloned()
            .unwrap_or(serde_json::Value::Object(Default::default()));

        let result = match name.as_str() {
            "bastion_exec" => self.tool_exec(args).await,
            "bastion_list_agents" => self.tool_list_agents().await,
            "bastion_get_audit" => self.tool_get_audit(args).await,
            "bastion_get_budget" => self.tool_get_budget().await,
            "bastion_network_status" => self.tool_network_status().await,
            unknown => {
                return JsonRpcResponse::error(
                    id,
                    METHOD_NOT_FOUND,
                    format!("Unknown tool: {unknown}"),
                );
            }
        };

        JsonRpcResponse::success(
            id,
            serde_json::json!({
                "content": [{ "type": "text", "text": serde_json::to_string(&result).unwrap_or_default() }]
            }),
        )
    }

    // ───────────────────────────── tool impls ────────────────────────────

    async fn tool_exec(&self, args: serde_json::Value) -> serde_json::Value {
        let command = match args.get("command").and_then(|v| v.as_str()) {
            Some(c) => c.to_string(),
            None => {
                return serde_json::json!({ "error": "Missing required parameter: command" });
            }
        };

        let profile = args
            .get("profile")
            .and_then(|v| v.as_str())
            .unwrap_or("default")
            .to_string();

        let timeout_ms = args
            .get("timeout_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(30_000);

        match self.executor.run(&command, &profile, timeout_ms).await {
            Ok(result) => serde_json::json!({
                "exit_code": result.exit_code,
                "stdout": result.stdout,
                "stderr": result.stderr,
            }),
            Err(e) => serde_json::json!({ "error": e.to_string() }),
        }
    }

    async fn tool_list_agents(&self) -> serde_json::Value {
        let agents = self.agents.list().await;
        serde_json::json!({ "agents": agents })
    }

    async fn tool_get_audit(&self, args: serde_json::Value) -> serde_json::Value {
        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(50) as usize;
        let offset = args
            .get("offset")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        let (logs, total) = self.audit.query(limit, offset).await;
        serde_json::json!({
            "logs": logs,
            "total": total,
            "limit": limit,
            "offset": offset,
        })
    }

    async fn tool_get_budget(&self) -> serde_json::Value {
        let budget = self.budget.get().await;
        serde_json::json!(budget)
    }

    async fn tool_network_status(&self) -> serde_json::Value {
        let status = self.network.get_status().await;
        let rules = self.network.get_rules().await;
        serde_json::json!({
            "proxy_enabled": status.proxy_enabled,
            "blocked_count": status.blocked_count,
            "allowed_count": status.allowed_count,
            "total_requests": status.total_requests,
            "rules": rules,
        })
    }
}

// Suppress unused import warning for INTERNAL_ERROR (used for completeness)
const _: i32 = INTERNAL_ERROR;
