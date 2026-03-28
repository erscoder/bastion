use axum::{extract::State, Json};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::{error, info};
use super::state::AppState;
use super::validation::{validate_command, validate_domain, validate_profile, validate_timeout};
use crate::domain::audit::AuditLog;
use crate::domain::network::{NetworkRule, RuleType};
use crate::infrastructure::sandbox::executor::SandboxExecutor;

// ============================================================================
// API Response Types
// ============================================================================

#[derive(Serialize)]
pub struct HealthChecks {
    pub sandbox: String,
    pub budget: String,
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub checks: HealthChecks,
}

#[derive(Serialize)]
pub struct MetricsResponse {
    pub uptime_seconds: u64,
    pub total_commands: u32,
    pub active_agents: usize,
    pub commands_remaining: u32,
    pub network_rules: usize,
    pub audit_log_entries: usize,
}

#[derive(Serialize)]
pub struct AgentsResponse {
    pub agents: Vec<crate::domain::agent::Agent>,
}

#[derive(Deserialize)]
pub struct ExecRequest {
    pub command: String,
    #[serde(default = "default_profile_name")]
    pub profile: String,
    #[serde(default)]
    pub agent_id: Option<String>,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

fn default_profile_name() -> String {
    "default".to_string()
}

fn default_timeout() -> u64 {
    30000
}

#[derive(Serialize)]
pub struct ExecResponse {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    #[serde(default)]
    pub agent_id: Option<String>,
}

#[derive(Serialize)]
pub struct AuditResponse {
    pub logs: Vec<AuditLog>,
    pub total: usize,
}

#[derive(Serialize)]
pub struct SandboxResponse {
    pub profiles: Vec<String>,
    pub active: String,
}

#[derive(Serialize)]
pub struct ProfileResponse {
    pub name: String,
    pub description: String,
    pub filesystem: FilesystemConfig,
    pub network: NetworkConfig,
}

#[derive(Serialize)]
pub struct FilesystemConfig {
    pub allowed_read: Vec<String>,
    pub allowed_write: Vec<String>,
    pub denied: Vec<String>,
}

#[derive(Serialize)]
pub struct NetworkConfig {
    pub proxy_enabled: bool,
    pub allowed_domains: Vec<String>,
    pub blocked_domains: Vec<String>,
}

#[derive(Serialize)]
pub struct PolicyReloadResponse {
    pub status: String,
    pub profiles_loaded: usize,
}

#[derive(Serialize)]
pub struct BudgetResponse {
    pub agents: usize,
    pub total_commands: u32,
    pub rate_limit: u32,
    pub commands_remaining: u32,
}

#[derive(Serialize)]
pub struct AgentDeleteResponse {
    pub status: String,
    pub agent_id: String,
}

// Network DTOs
#[derive(Deserialize)]
pub struct NetworkRuleRequest {
    pub domain: String,
}

#[derive(Serialize)]
pub struct NetworkStatusResponse {
    pub proxy_enabled: bool,
    pub blocked_domains: Vec<String>,
    pub allowed_domains: Vec<String>,
    pub total_requests: usize,
}

#[derive(Serialize)]
pub struct NetworkLogEntry {
    pub timestamp: String,
    pub domain: String,
    pub rule_applied: Option<String>,
    pub allowed: bool,
}

#[derive(Serialize)]
pub struct NetworkLogsResponse {
    pub logs: Vec<NetworkLogEntry>,
    pub total: usize,
}

#[derive(Serialize)]
pub struct NetworkRuleResponse {
    pub status: String,
    pub domain: String,
    pub rule_type: String,
}

// Budget extended DTOs
#[derive(Serialize)]
pub struct BudgetResetResponse {
    pub status: String,
    pub commands_remaining: u32,
}

#[derive(Serialize)]
pub struct AgentBudgetResponse {
    pub agent_id: String,
    pub commands_executed: u32,
    pub commands_remaining: u32,
}

// ============================================================================
// Handlers
// ============================================================================

pub async fn health_handler(State(state): State<AppState>) -> Json<HealthResponse> {
    let uptime = state.start_time.elapsed().as_secs();
    let budget = state.budget.get().await;

    let sandbox_check = if SandboxExecutor::sandbox_exec_available() {
        "ok".to_string()
    } else {
        "degraded".to_string()
    };

    let budget_check = if budget.commands_remaining > 0 {
        "ok".to_string()
    } else {
        "exhausted".to_string()
    };

    Json(HealthResponse {
        status: "healthy".to_string(),
        version: "0.1.0".to_string(),
        uptime_seconds: uptime,
        checks: HealthChecks {
            sandbox: sandbox_check,
            budget: budget_check,
        },
    })
}

pub async fn metrics_handler(State(state): State<AppState>) -> Json<MetricsResponse> {
    let uptime = state.start_time.elapsed().as_secs();
    let budget = state.budget.get().await;
    let agents = state.agents.list().await;
    let rules = state.network.get_rules().await;
    let (_, audit_total) = state.audit.query(0, 0).await;

    Json(MetricsResponse {
        uptime_seconds: uptime,
        total_commands: budget.total_commands,
        active_agents: agents.len(),
        commands_remaining: budget.commands_remaining,
        network_rules: rules.len(),
        audit_log_entries: audit_total,
    })
}

pub async fn agents_handler(State(state): State<AppState>) -> Json<AgentsResponse> {
    let agents = state.agents.list().await;
    Json(AgentsResponse { agents })
}

pub async fn exec_handler(
    State(state): State<AppState>,
    Json(payload): Json<ExecRequest>,
) -> Result<Json<ExecResponse>, (axum::http::StatusCode, String)> {
    // Input validation
    validate_command(&payload.command).map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e))?;
    validate_timeout(payload.timeout_ms).map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e))?;
    validate_profile(&payload.profile).map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e))?;

    // Rate limiting check
    let budget = state.budget.get().await;
    if budget.commands_remaining == 0 {
        return Err((
            axum::http::StatusCode::TOO_MANY_REQUESTS,
            "Rate limit exceeded: no commands remaining".to_string(),
        ));
    }

    info!("Executing command: {} (profile: {})", payload.command, payload.profile);

    let start = std::time::Instant::now();

    let result = state
        .executor
        .run(&payload.command, &payload.profile, payload.timeout_ms)
        .await
        .map_err(|e| {
            error!("Failed to execute command: {}", e);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?;

    let duration_ms = start.elapsed().as_millis() as u64;

    let agent_id_str = payload.agent_id.clone().unwrap_or_else(|| "unknown".to_string());
    state.budget.increment_for_agent(&agent_id_str).await;

    state.audit.append(AuditLog {
        timestamp: Utc::now(),
        agent_id: agent_id_str,
        command: payload.command.clone(),
        exit_code: result.exit_code,
        duration_ms,
        profile: payload.profile,
    }).await;

    Ok(Json(ExecResponse {
        exit_code: result.exit_code,
        stdout: result.stdout,
        stderr: result.stderr,
        duration_ms,
        agent_id: payload.agent_id,
    }))
}

pub async fn audit_handler(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<AuditResponse> {
    let limit = params
        .get("limit")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(100);
    let offset = params
        .get("offset")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);

    let (logs, total) = state.audit.query(limit, offset).await;
    Json(AuditResponse { logs, total })
}

pub async fn sandbox_handler(State(state): State<AppState>) -> Json<SandboxResponse> {
    let profiles = state.sandbox_profiles.read().await;
    let profile_names: Vec<String> = profiles.iter().map(|p| p.name.clone()).collect();
    Json(SandboxResponse {
        profiles: profile_names,
        active: state.config.default_profile.clone(),
    })
}

pub async fn profile_handler(State(_state): State<AppState>) -> Json<ProfileResponse> {
    Json(ProfileResponse {
        name: "default".to_string(),
        description: "Balanced security for general use".to_string(),
        filesystem: FilesystemConfig {
            allowed_read: vec!["/tmp".to_string(), "/var/folders".to_string()],
            allowed_write: vec!["/tmp".to_string()],
            denied: vec!["/System".to_string(), "/Users/*/Library".to_string()],
        },
        network: NetworkConfig {
            proxy_enabled: true,
            allowed_domains: vec!["*.openai.com".to_string(), "*.anthropic.com".to_string()],
            blocked_domains: vec![],
        },
    })
}

pub async fn policy_reload_handler(State(state): State<AppState>) -> Json<PolicyReloadResponse> {
    info!("Reloading sandbox policies");
    let profiles = state.sandbox_profiles.read().await;
    Json(PolicyReloadResponse {
        status: "reloaded".to_string(),
        profiles_loaded: profiles.len(),
    })
}

pub async fn budget_handler(State(state): State<AppState>) -> Json<BudgetResponse> {
    let budget = state.budget.get().await;
    let agents = state.agents.list().await;
    Json(BudgetResponse {
        agents: agents.len(),
        total_commands: budget.total_commands,
        rate_limit: state.config.max_commands_per_hour,
        commands_remaining: budget.commands_remaining,
    })
}

pub async fn delete_agent_handler(
    State(state): State<AppState>,
    axum::extract::Path(agent_id): axum::extract::Path<String>,
) -> Json<AgentDeleteResponse> {
    info!("Killing agent: {}", agent_id);
    state.agents.delete(&agent_id).await;
    Json(AgentDeleteResponse {
        status: "killed".to_string(),
        agent_id,
    })
}

pub async fn reset_agent_handler(
    State(state): State<AppState>,
    axum::extract::Path(agent_id): axum::extract::Path<String>,
) -> Json<AgentDeleteResponse> {
    info!("Resetting agent: {}", agent_id);
    state.agents.reset(&agent_id).await;
    Json(AgentDeleteResponse {
        status: "reset".to_string(),
        agent_id,
    })
}

// ============================================================================
// Network handlers
// ============================================================================

pub async fn network_status_handler(State(state): State<AppState>) -> Json<NetworkStatusResponse> {
    let rules = state.network.get_rules().await;
    let status = state.network.get_status().await;

    let blocked_domains: Vec<String> = rules
        .iter()
        .filter(|r| r.rule_type == RuleType::Block)
        .map(|r| r.domain.clone())
        .collect();

    let allowed_domains: Vec<String> = rules
        .iter()
        .filter(|r| r.rule_type == RuleType::Allow)
        .map(|r| r.domain.clone())
        .collect();

    Json(NetworkStatusResponse {
        proxy_enabled: status.proxy_enabled,
        blocked_domains,
        allowed_domains,
        total_requests: status.total_requests,
    })
}

pub async fn network_block_handler(
    State(state): State<AppState>,
    Json(payload): Json<NetworkRuleRequest>,
) -> Result<Json<NetworkRuleResponse>, (axum::http::StatusCode, String)> {
    validate_domain(&payload.domain).map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e))?;

    info!("Blocking domain: {}", payload.domain);
    let rule = NetworkRule {
        domain: payload.domain.clone(),
        rule_type: RuleType::Block,
        created_at: Utc::now(),
    };
    state.network.add_rule(rule).await;
    Ok(Json(NetworkRuleResponse {
        status: "blocked".to_string(),
        domain: payload.domain,
        rule_type: "block".to_string(),
    }))
}

pub async fn network_allow_handler(
    State(state): State<AppState>,
    Json(payload): Json<NetworkRuleRequest>,
) -> Result<Json<NetworkRuleResponse>, (axum::http::StatusCode, String)> {
    validate_domain(&payload.domain).map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e))?;

    info!("Allowing domain: {}", payload.domain);
    let rule = NetworkRule {
        domain: payload.domain.clone(),
        rule_type: RuleType::Allow,
        created_at: Utc::now(),
    };
    state.network.add_rule(rule).await;
    Ok(Json(NetworkRuleResponse {
        status: "allowed".to_string(),
        domain: payload.domain,
        rule_type: "allow".to_string(),
    }))
}

pub async fn network_logs_handler(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<NetworkLogsResponse> {
    let limit = params
        .get("limit")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(100);

    let logs = state.network.get_logs(limit).await;
    let total = logs.len();

    let entries: Vec<NetworkLogEntry> = logs
        .into_iter()
        .map(|l| NetworkLogEntry {
            timestamp: l.timestamp.to_rfc3339(),
            domain: l.domain,
            rule_applied: l.rule_applied.map(|rt| match rt {
                RuleType::Block => "block".to_string(),
                RuleType::Allow => "allow".to_string(),
            }),
            allowed: l.allowed,
        })
        .collect();

    Json(NetworkLogsResponse {
        logs: entries,
        total,
    })
}

// ============================================================================
// Budget extended handlers
// ============================================================================

pub async fn budget_reset_handler(State(state): State<AppState>) -> Json<BudgetResetResponse> {
    info!("Resetting budget");
    state.budget.reset().await;
    let budget = state.budget.get().await;
    Json(BudgetResetResponse {
        status: "reset".to_string(),
        commands_remaining: budget.commands_remaining,
    })
}

pub async fn budget_agent_handler(
    State(state): State<AppState>,
    axum::extract::Path(agent_id): axum::extract::Path<String>,
) -> Result<Json<AgentBudgetResponse>, (axum::http::StatusCode, String)> {
    match state.budget.get_by_agent(&agent_id).await {
        Some(ab) => Ok(Json(AgentBudgetResponse {
            agent_id: ab.agent_id,
            commands_executed: ab.commands_executed,
            commands_remaining: ab.commands_remaining,
        })),
        None => Err((
            axum::http::StatusCode::NOT_FOUND,
            format!("Agent {} not found", agent_id),
        )),
    }
}
