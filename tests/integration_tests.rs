//! HTTP integration tests for Bastion API handlers (Sprint 2 + Sprint 3).
//!
//! Uses axum-test to exercise the full request/response pipeline including
//! auth middleware, routing, and handler logic — without binding a real port.

use axum_test::TestServer;
use bastion::{create_app, create_state, Config};
use bastion::domain::agent::Agent;
use chrono::Utc;
use serde_json::{json, Value};
use std::path::PathBuf;

/// Basic auth header for default credentials (bastion:bastion).
/// base64("bastion:bastion") = "YmFzdGlvbjpiYXN0aW9u"
const AUTH: &str = "Basic YmFzdGlvbjpiYXN0aW9u";

/// Bad credentials header.
const BAD_AUTH: &str = "Basic d3Jvbmc6Y3JlZHM="; // wrong:creds

fn test_config() -> Config {
    let sandbox = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sandbox");
    Config {
        sandbox_dir: sandbox.clone(),
        profiles_dir: sandbox,
        ..Config::default()
    }
}

fn make_server() -> TestServer {
    let state = create_state(test_config());
    let app = create_app(state);
    TestServer::new(app).expect("Failed to create test server")
}

// ============================================================================
// Auth tests
// ============================================================================

#[tokio::test]
async fn test_auth_required() {
    let server = make_server();
    let res = server.get("/api/health").await;
    assert_eq!(res.status_code(), 401, "Expected 401 without credentials");
}

#[tokio::test]
async fn test_auth_bad_credentials() {
    let server = make_server();
    let res = server
        .get("/api/health")
        .add_header("authorization", BAD_AUTH)
        .await;
    assert_eq!(res.status_code(), 401, "Expected 401 with wrong credentials");
}

#[tokio::test]
async fn test_auth_valid() {
    let server = make_server();
    let res = server
        .get("/api/health")
        .add_header("authorization", AUTH)
        .await;
    assert_eq!(res.status_code(), 200, "Expected 200 with valid credentials");
}

// ============================================================================
// Health endpoint
// ============================================================================

#[tokio::test]
async fn test_health_response() {
    let server = make_server();
    let res = server
        .get("/api/health")
        .add_header("authorization", AUTH)
        .await;
    res.assert_status_ok();
    let body: Value = res.json();
    assert_eq!(body["status"], "healthy");
    assert_eq!(body["version"], "0.1.0");
    assert!(
        body["uptime_seconds"].is_number(),
        "uptime_seconds should be a number"
    );
}

// ============================================================================
// Agents endpoint
// ============================================================================

#[tokio::test]
async fn test_agents_empty() {
    let server = make_server();
    let res = server
        .get("/api/agents")
        .add_header("authorization", AUTH)
        .await;
    res.assert_status_ok();
    let body: Value = res.json();
    assert_eq!(body["agents"], json!([]));
}

#[tokio::test]
async fn test_agents_with_data() {
    // Pre-seed the state with an agent and verify it shows up.
    let state = create_state(test_config());
    state.agents.add(Agent {
        id: "test-agent-1".to_string(),
        name: "Test Agent".to_string(),
        created_at: Utc::now(),
        commands_executed: 0,
        status: "active".to_string(),
    }).await;
    let app = create_app(state);
    let server = TestServer::new(app).unwrap();
    let res = server
        .get("/api/agents")
        .add_header("authorization", AUTH)
        .await;
    res.assert_status_ok();
    let body: Value = res.json();
    let agents = body["agents"].as_array().unwrap();
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0]["id"], "test-agent-1");
    assert_eq!(agents[0]["name"], "Test Agent");
}

// ============================================================================
// Exec endpoint
// ============================================================================

#[tokio::test]
async fn test_exec_echo() {
    let server = make_server();
    let res = server
        .post("/api/exec")
        .add_header("authorization", AUTH)
        .json(&json!({"command": "echo hi"}))
        .await;
    res.assert_status_ok();
    let body: Value = res.json();
    assert_eq!(body["exit_code"], 0);
    assert_eq!(body["stdout"], "hi\n");
    assert!(body["duration_ms"].is_number());
}

#[tokio::test]
async fn test_exec_nonzero_exit() {
    let server = make_server();
    let res = server
        .post("/api/exec")
        .add_header("authorization", AUTH)
        .json(&json!({"command": "exit 2"}))
        .await;
    res.assert_status_ok();
    let body: Value = res.json();
    assert_eq!(body["exit_code"], 2);
}

#[tokio::test]
async fn test_exec_with_agent_id() {
    let server = make_server();
    let res = server
        .post("/api/exec")
        .add_header("authorization", AUTH)
        .json(&json!({"command": "echo test", "agent_id": "agent-abc"}))
        .await;
    res.assert_status_ok();
    let body: Value = res.json();
    assert_eq!(body["exit_code"], 0);
    assert_eq!(body["agent_id"], "agent-abc");
}

#[tokio::test]
async fn test_exec_stderr_capture() {
    let server = make_server();
    let res = server
        .post("/api/exec")
        .add_header("authorization", AUTH)
        .json(&json!({"command": "echo err >&2"}))
        .await;
    res.assert_status_ok();
    let body: Value = res.json();
    assert_eq!(body["exit_code"], 0);
    assert_eq!(body["stderr"], "err\n");
}

#[tokio::test]
async fn test_exec_timeout() {
    let server = make_server();
    let res = server
        .post("/api/exec")
        .add_header("authorization", AUTH)
        .json(&json!({"command": "sleep 10", "timeout_ms": 100}))
        .await;
    res.assert_status_ok();
    let body: Value = res.json();
    assert_eq!(body["exit_code"], -1);
    let stderr = body["stderr"].as_str().unwrap_or("");
    assert!(
        stderr.to_lowercase().contains("timeout"),
        "Expected timeout in stderr, got: {}",
        stderr
    );
}

// ============================================================================
// Audit endpoint
// ============================================================================

#[tokio::test]
async fn test_audit_empty() {
    let server = make_server();
    let res = server
        .get("/api/audit")
        .add_header("authorization", AUTH)
        .await;
    res.assert_status_ok();
    let body: Value = res.json();
    assert_eq!(body["logs"], json!([]));
    assert_eq!(body["total"], 0);
}

#[tokio::test]
async fn test_audit_after_exec() {
    let server = make_server();

    // Execute a command first
    server
        .post("/api/exec")
        .add_header("authorization", AUTH)
        .json(&json!({"command": "echo audit_test", "agent_id": "agent-x"}))
        .await
        .assert_status_ok();

    // Now check audit logs
    let res = server
        .get("/api/audit")
        .add_header("authorization", AUTH)
        .await;
    res.assert_status_ok();
    let body: Value = res.json();
    assert_eq!(body["total"], 1);
    let logs = body["logs"].as_array().unwrap();
    assert_eq!(logs[0]["command"], "echo audit_test");
    assert_eq!(logs[0]["agent_id"], "agent-x");
    assert_eq!(logs[0]["exit_code"], 0);
}

#[tokio::test]
async fn test_audit_pagination() {
    let server = make_server();

    // Execute multiple commands
    for i in 0..5 {
        server
            .post("/api/exec")
            .add_header("authorization", AUTH)
            .json(&json!({"command": format!("echo cmd{}", i)}))
            .await
            .assert_status_ok();
    }

    // Test limit
    let res = server
        .get("/api/audit?limit=2")
        .add_header("authorization", AUTH)
        .await;
    res.assert_status_ok();
    let body: Value = res.json();
    assert_eq!(body["total"], 5);
    assert_eq!(body["logs"].as_array().unwrap().len(), 2);

    // Test offset
    let res2 = server
        .get("/api/audit?limit=10&offset=3")
        .add_header("authorization", AUTH)
        .await;
    res2.assert_status_ok();
    let body2: Value = res2.json();
    assert_eq!(body2["logs"].as_array().unwrap().len(), 2);
}

// ============================================================================
// Sandbox endpoint
// ============================================================================

#[tokio::test]
async fn test_sandbox_profiles() {
    let server = make_server();
    let res = server
        .get("/api/sandbox")
        .add_header("authorization", AUTH)
        .await;
    res.assert_status_ok();
    let body: Value = res.json();
    let profiles = body["profiles"].as_array().unwrap();
    assert_eq!(profiles.len(), 3, "Expected 3 sandbox profiles");
    assert!(
        profiles.contains(&json!("default")),
        "Expected 'default' profile"
    );
    assert!(
        profiles.contains(&json!("strict")),
        "Expected 'strict' profile"
    );
    assert!(
        profiles.contains(&json!("permissive")),
        "Expected 'permissive' profile"
    );
    assert_eq!(body["active"], "default");
}

// ============================================================================
// Profile endpoint
// ============================================================================

#[tokio::test]
async fn test_profile_default() {
    let server = make_server();
    let res = server
        .get("/api/profile")
        .add_header("authorization", AUTH)
        .await;
    res.assert_status_ok();
    let body: Value = res.json();
    assert_eq!(body["name"], "default");
    assert!(body["description"].is_string());
    assert!(body["filesystem"].is_object());
    assert!(body["network"].is_object());
    assert!(
        body["filesystem"]["allowed_read"].is_array(),
        "filesystem.allowed_read should be array"
    );
    assert!(
        body["network"]["proxy_enabled"].is_boolean(),
        "network.proxy_enabled should be boolean"
    );
}

// ============================================================================
// Policy reload endpoint
// ============================================================================

#[tokio::test]
async fn test_policy_reload() {
    let server = make_server();
    let res = server
        .post("/api/policy/reload")
        .add_header("authorization", AUTH)
        .await;
    res.assert_status_ok();
    let body: Value = res.json();
    assert_eq!(body["status"], "reloaded");
    assert!(
        body["profiles_loaded"].is_number(),
        "profiles_loaded should be a number"
    );
    assert_eq!(body["profiles_loaded"], 3);
}

// ============================================================================
// Budget endpoint
// ============================================================================

#[tokio::test]
async fn test_budget_initial() {
    let server = make_server();
    let res = server
        .get("/api/budget")
        .add_header("authorization", AUTH)
        .await;
    res.assert_status_ok();
    let body: Value = res.json();
    assert_eq!(body["total_commands"], 0);
    assert_eq!(body["commands_remaining"], 100);
    assert_eq!(body["rate_limit"], 100);
    assert_eq!(body["agents"], 0);
}

#[tokio::test]
async fn test_budget_after_exec() {
    let server = make_server();

    // Execute two commands
    for _ in 0..2 {
        server
            .post("/api/exec")
            .add_header("authorization", AUTH)
            .json(&json!({"command": "echo x"}))
            .await
            .assert_status_ok();
    }

    let res = server
        .get("/api/budget")
        .add_header("authorization", AUTH)
        .await;
    res.assert_status_ok();
    let body: Value = res.json();
    assert_eq!(body["total_commands"], 2);
    assert_eq!(body["commands_remaining"], 98);
}

// ============================================================================
// Agent management (delete / reset)
// ============================================================================

#[tokio::test]
async fn test_delete_agent_nonexistent() {
    // Deleting a non-existent agent still returns "killed" (idempotent)
    let server = make_server();
    let res = server
        .delete("/api/agents/nonexistent-id")
        .add_header("authorization", AUTH)
        .await;
    res.assert_status_ok();
    let body: Value = res.json();
    assert_eq!(body["status"], "killed");
    assert_eq!(body["agent_id"], "nonexistent-id");
}

#[tokio::test]
async fn test_delete_agent_removes_from_list() {
    let state = create_state(test_config());
    state.agents.add(Agent {
        id: "del-me".to_string(),
        name: "ToDelete".to_string(),
        created_at: Utc::now(),
        commands_executed: 0,
        status: "active".to_string(),
    }).await;
    let app = create_app(state);
    let server = TestServer::new(app).unwrap();

    // Verify agent exists
    let res = server
        .get("/api/agents")
        .add_header("authorization", AUTH)
        .await;
    let body: Value = res.json();
    assert_eq!(body["agents"].as_array().unwrap().len(), 1);

    // Delete it
    let del_res = server
        .delete("/api/agents/del-me")
        .add_header("authorization", AUTH)
        .await;
    del_res.assert_status_ok();
    assert_eq!(del_res.json::<Value>()["status"], "killed");

    // Verify it's gone
    let res2 = server
        .get("/api/agents")
        .add_header("authorization", AUTH)
        .await;
    let body2: Value = res2.json();
    assert_eq!(body2["agents"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_reset_agent() {
    let state = create_state(test_config());
    state.agents.add(Agent {
        id: "reset-me".to_string(),
        name: "ToReset".to_string(),
        created_at: Utc::now(),
        commands_executed: 42,
        status: "active".to_string(),
    }).await;
    let app = create_app(state);
    let server = TestServer::new(app).unwrap();

    let res = server
        .post("/api/agents/reset-me/reset")
        .add_header("authorization", AUTH)
        .await;
    res.assert_status_ok();
    let body: Value = res.json();
    assert_eq!(body["status"], "reset");
    assert_eq!(body["agent_id"], "reset-me");

    // Verify commands_executed was reset to 0
    let agents_res = server
        .get("/api/agents")
        .add_header("authorization", AUTH)
        .await;
    let agents_body: Value = agents_res.json();
    let agents = agents_body["agents"].as_array().unwrap();
    let agent = &agents[0];
    assert_eq!(agent["commands_executed"], 0);
}

#[tokio::test]
async fn test_reset_agent_nonexistent() {
    // Resetting a non-existent agent returns "reset" without error
    let server = make_server();
    let res = server
        .post("/api/agents/no-such-id/reset")
        .add_header("authorization", AUTH)
        .await;
    res.assert_status_ok();
    let body: Value = res.json();
    assert_eq!(body["status"], "reset");
    assert_eq!(body["agent_id"], "no-such-id");
}

// ============================================================================
// Sprint 3: Network Control tests
// ============================================================================

#[tokio::test]
async fn test_network_status_empty() {
    let server = make_server();
    let res = server
        .get("/api/network/status")
        .add_header("authorization", AUTH)
        .await;
    res.assert_status_ok();
    let body: Value = res.json();
    assert!(body["proxy_enabled"].as_bool().unwrap_or(false), "proxy_enabled should be true");
    assert_eq!(body["blocked_domains"], json!([]));
    assert_eq!(body["allowed_domains"], json!([]));
    assert_eq!(body["total_requests"], 0);
}

#[tokio::test]
async fn test_network_block_domain() {
    let server = make_server();
    let res = server
        .post("/api/network/block")
        .add_header("authorization", AUTH)
        .json(&json!({"domain": "evil.com"}))
        .await;
    res.assert_status_ok();
    let body: Value = res.json();
    assert_eq!(body["status"], "blocked");
    assert_eq!(body["domain"], "evil.com");
    assert_eq!(body["rule_type"], "block");

    // Verify it appears in status
    let status_res = server
        .get("/api/network/status")
        .add_header("authorization", AUTH)
        .await;
    status_res.assert_status_ok();
    let status: Value = status_res.json();
    let blocked = status["blocked_domains"].as_array().unwrap();
    assert!(blocked.contains(&json!("evil.com")), "evil.com should be in blocked_domains");
    assert_eq!(status["allowed_domains"], json!([]));
}

#[tokio::test]
async fn test_network_allow_domain() {
    let server = make_server();
    let res = server
        .post("/api/network/allow")
        .add_header("authorization", AUTH)
        .json(&json!({"domain": "trusted.com"}))
        .await;
    res.assert_status_ok();
    let body: Value = res.json();
    assert_eq!(body["status"], "allowed");
    assert_eq!(body["domain"], "trusted.com");
    assert_eq!(body["rule_type"], "allow");

    // Verify it appears in status
    let status_res = server
        .get("/api/network/status")
        .add_header("authorization", AUTH)
        .await;
    status_res.assert_status_ok();
    let status: Value = status_res.json();
    let allowed = status["allowed_domains"].as_array().unwrap();
    assert!(allowed.contains(&json!("trusted.com")), "trusted.com should be in allowed_domains");
    assert_eq!(status["blocked_domains"], json!([]));
}

#[tokio::test]
async fn test_network_block_and_allow_multiple() {
    let server = make_server();

    // Add a blocked domain
    server
        .post("/api/network/block")
        .add_header("authorization", AUTH)
        .json(&json!({"domain": "bad1.com"}))
        .await
        .assert_status_ok();

    server
        .post("/api/network/block")
        .add_header("authorization", AUTH)
        .json(&json!({"domain": "bad2.com"}))
        .await
        .assert_status_ok();

    // Add an allowed domain
    server
        .post("/api/network/allow")
        .add_header("authorization", AUTH)
        .json(&json!({"domain": "good.com"}))
        .await
        .assert_status_ok();

    let status_res = server
        .get("/api/network/status")
        .add_header("authorization", AUTH)
        .await;
    status_res.assert_status_ok();
    let status: Value = status_res.json();
    let blocked = status["blocked_domains"].as_array().unwrap();
    let allowed = status["allowed_domains"].as_array().unwrap();

    assert_eq!(blocked.len(), 2, "Should have 2 blocked domains");
    assert_eq!(allowed.len(), 1, "Should have 1 allowed domain");
}

#[tokio::test]
async fn test_network_rule_overwrite() {
    // Adding a rule for the same domain should overwrite
    let server = make_server();

    server
        .post("/api/network/block")
        .add_header("authorization", AUTH)
        .json(&json!({"domain": "example.com"}))
        .await
        .assert_status_ok();

    // Now allow the same domain — should replace block with allow
    server
        .post("/api/network/allow")
        .add_header("authorization", AUTH)
        .json(&json!({"domain": "example.com"}))
        .await
        .assert_status_ok();

    let status_res = server
        .get("/api/network/status")
        .add_header("authorization", AUTH)
        .await;
    status_res.assert_status_ok();
    let status: Value = status_res.json();
    let blocked = status["blocked_domains"].as_array().unwrap();
    let allowed = status["allowed_domains"].as_array().unwrap();

    assert_eq!(blocked.len(), 0, "Should have no blocked domains after overwrite");
    assert_eq!(allowed.len(), 1, "Should have 1 allowed domain");
    assert!(allowed.contains(&json!("example.com")));
}

#[tokio::test]
async fn test_network_logs_empty() {
    let server = make_server();
    let res = server
        .get("/api/network/logs")
        .add_header("authorization", AUTH)
        .await;
    res.assert_status_ok();
    let body: Value = res.json();
    assert_eq!(body["logs"], json!([]));
    assert_eq!(body["total"], 0);
}

// ============================================================================
// Sprint 3: Budget extended tests
// ============================================================================

#[tokio::test]
async fn test_budget_reset() {
    let server = make_server();

    // Execute some commands to deplete budget
    for _ in 0..3 {
        server
            .post("/api/exec")
            .add_header("authorization", AUTH)
            .json(&json!({"command": "echo x"}))
            .await
            .assert_status_ok();
    }

    // Verify budget was decremented
    let budget_res = server.get("/api/budget").add_header("authorization", AUTH).await;
    let budget: Value = budget_res.json();
    assert_eq!(budget["commands_remaining"], 97);

    // Reset budget
    let reset_res = server
        .post("/api/budget/reset")
        .add_header("authorization", AUTH)
        .await;
    reset_res.assert_status_ok();
    let reset_body: Value = reset_res.json();
    assert_eq!(reset_body["status"], "reset");
    assert_eq!(reset_body["commands_remaining"], 100, "After reset, remaining should be 100");

    // Verify budget is back to 100
    let budget_res2 = server.get("/api/budget").add_header("authorization", AUTH).await;
    let budget2: Value = budget_res2.json();
    assert_eq!(budget2["commands_remaining"], 100);
    assert_eq!(budget2["total_commands"], 0);
}

#[tokio::test]
async fn test_budget_per_agent() {
    let server = make_server();

    // Execute a command with a specific agent
    server
        .post("/api/exec")
        .add_header("authorization", AUTH)
        .json(&json!({"command": "echo hello", "agent_id": "test-agent-budget"}))
        .await
        .assert_status_ok();

    // Get per-agent budget
    let res = server
        .get("/api/budget/test-agent-budget")
        .add_header("authorization", AUTH)
        .await;
    res.assert_status_ok();
    let body: Value = res.json();
    assert_eq!(body["agent_id"], "test-agent-budget");
    assert_eq!(body["commands_executed"], 1);
    assert!(body["commands_remaining"].is_number());
}

#[tokio::test]
async fn test_budget_per_agent_tracks_multiple_commands() {
    let server = make_server();

    let agent_id = "multi-exec-agent";
    for _ in 0..4 {
        server
            .post("/api/exec")
            .add_header("authorization", AUTH)
            .json(&json!({"command": "echo hi", "agent_id": agent_id}))
            .await
            .assert_status_ok();
    }

    let res = server
        .get(&format!("/api/budget/{}", agent_id))
        .add_header("authorization", AUTH)
        .await;
    res.assert_status_ok();
    let body: Value = res.json();
    assert_eq!(body["agent_id"], agent_id);
    assert_eq!(body["commands_executed"], 4);
}

// ============================================================================
// Sprint 3: Rate limiting tests
// ============================================================================

#[tokio::test]
async fn test_rate_limit_exceeded() {
    // Create a state with budget exhausted
    let state = create_state(test_config());
    // Drain the budget to 0
    state.budget.set_remaining(0).await;

    let app = create_app(state);
    let server = TestServer::new(app).unwrap();

    // Next exec should return 429
    let res = server
        .post("/api/exec")
        .add_header("authorization", AUTH)
        .json(&json!({"command": "echo should_fail"}))
        .await;
    assert_eq!(res.status_code(), 429, "Expected 429 Too Many Requests when budget exhausted");
}

#[tokio::test]
async fn test_rate_limit_allows_after_reset() {
    // Create state, drain budget, reset, then exec should succeed
    let state = create_state(test_config());
    state.budget.set_remaining(0).await;

    let app = create_app(state.clone());
    let server = TestServer::new(app).unwrap();

    // Should be blocked
    let blocked_res = server
        .post("/api/exec")
        .add_header("authorization", AUTH)
        .json(&json!({"command": "echo x"}))
        .await;
    assert_eq!(blocked_res.status_code(), 429);

    // Reset budget
    let reset_res = server
        .post("/api/budget/reset")
        .add_header("authorization", AUTH)
        .await;
    reset_res.assert_status_ok();

    // Now should succeed
    let ok_res = server
        .post("/api/exec")
        .add_header("authorization", AUTH)
        .json(&json!({"command": "echo ok"}))
        .await;
    assert_eq!(ok_res.status_code(), 200, "Expected 200 after budget reset");
}

#[tokio::test]
async fn test_rate_limit_boundary() {
    // Budget of 1: first exec succeeds, second returns 429
    let state = create_state(test_config());
    state.budget.set_remaining(1).await;

    let app = create_app(state);
    let server = TestServer::new(app).unwrap();

    // First exec: should succeed
    let first = server
        .post("/api/exec")
        .add_header("authorization", AUTH)
        .json(&json!({"command": "echo boundary"}))
        .await;
    assert_eq!(first.status_code(), 200, "First exec should succeed");

    // Second exec: budget is now 0, should 429
    let second = server
        .post("/api/exec")
        .add_header("authorization", AUTH)
        .json(&json!({"command": "echo boundary2"}))
        .await;
    assert_eq!(second.status_code(), 429, "Second exec should be rate limited");
}

// ============================================================================
// Sprint 5: Input Validation tests
// ============================================================================

#[tokio::test]
async fn test_exec_empty_command() {
    let server = make_server();
    let res = server
        .post("/api/exec")
        .add_header("authorization", AUTH)
        .json(&json!({"command": ""}))
        .await;
    assert_eq!(res.status_code(), 400, "Expected 400 for empty command");
}

#[tokio::test]
async fn test_exec_timeout_too_large() {
    let server = make_server();
    let res = server
        .post("/api/exec")
        .add_header("authorization", AUTH)
        .json(&json!({"command": "echo x", "timeout_ms": 999_999u64}))
        .await;
    assert_eq!(res.status_code(), 400, "Expected 400 for timeout exceeding 300_000ms");
}

#[tokio::test]
async fn test_exec_invalid_profile() {
    let server = make_server();
    let res = server
        .post("/api/exec")
        .add_header("authorization", AUTH)
        .json(&json!({"command": "echo x", "profile": "../../etc"}))
        .await;
    assert_eq!(res.status_code(), 400, "Expected 400 for invalid profile name");
}

#[tokio::test]
async fn test_network_block_empty_domain() {
    let server = make_server();
    let res = server
        .post("/api/network/block")
        .add_header("authorization", AUTH)
        .json(&json!({"domain": ""}))
        .await;
    assert_eq!(res.status_code(), 400, "Expected 400 for empty domain");
}

#[tokio::test]
async fn test_network_allow_empty_domain() {
    let server = make_server();
    let res = server
        .post("/api/network/allow")
        .add_header("authorization", AUTH)
        .json(&json!({"domain": ""}))
        .await;
    assert_eq!(res.status_code(), 400, "Expected 400 for empty domain in allow");
}

// ============================================================================
// Sprint 5: Metrics endpoint tests
// ============================================================================

#[tokio::test]
async fn test_metrics_initial() {
    let server = make_server();
    let res = server
        .get("/api/metrics")
        .add_header("authorization", AUTH)
        .await;
    res.assert_status_ok();
    let body: Value = res.json();
    assert!(body["uptime_seconds"].is_number(), "uptime_seconds should be a number");
    assert_eq!(body["total_commands"], 0, "Initial total_commands should be 0");
    assert_eq!(body["active_agents"], 0, "Initial active_agents should be 0");
    assert_eq!(body["commands_remaining"], 100, "Initial commands_remaining should be 100");
    assert_eq!(body["network_rules"], 0, "Initial network_rules should be 0");
    assert_eq!(body["audit_log_entries"], 0, "Initial audit_log_entries should be 0");
}

#[tokio::test]
async fn test_metrics_after_exec() {
    let server = make_server();

    // Execute one command
    server
        .post("/api/exec")
        .add_header("authorization", AUTH)
        .json(&json!({"command": "echo metrics_test"}))
        .await
        .assert_status_ok();

    let res = server
        .get("/api/metrics")
        .add_header("authorization", AUTH)
        .await;
    res.assert_status_ok();
    let body: Value = res.json();
    assert_eq!(body["total_commands"], 1, "total_commands should be 1 after one exec");
    assert_eq!(body["commands_remaining"], 99, "commands_remaining should be 99 after one exec");
    assert_eq!(body["audit_log_entries"], 1, "audit_log_entries should be 1 after one exec");
}

// ============================================================================
// Sprint 5: Enhanced health check tests
// ============================================================================

#[tokio::test]
async fn test_health_has_checks() {
    let server = make_server();
    let res = server
        .get("/api/health")
        .add_header("authorization", AUTH)
        .await;
    res.assert_status_ok();
    let body: Value = res.json();
    assert!(body["checks"].is_object(), "health response should have 'checks' object");
    assert!(body["checks"]["sandbox"].is_string(), "checks.sandbox should be a string");
    assert!(body["checks"]["budget"].is_string(), "checks.budget should be a string");
    // Budget is full at startup, so budget check should be "ok"
    assert_eq!(body["checks"]["budget"], "ok", "checks.budget should be 'ok' when commands remain");
}

// ============================================================================
// Sprint 5: Request ID tracking tests
// ============================================================================

#[tokio::test]
async fn test_request_id_generated() {
    let server = make_server();
    let res = server
        .get("/api/health")
        .add_header("authorization", AUTH)
        .await;
    res.assert_status_ok();
    let request_id = res.headers().get("x-request-id");
    assert!(
        request_id.is_some(),
        "Response should contain X-Request-ID header"
    );
    let id_str = request_id.unwrap().to_str().unwrap();
    assert!(!id_str.is_empty(), "X-Request-ID should not be empty");
}

#[tokio::test]
async fn test_request_id_propagated() {
    let server = make_server();
    let custom_id = "my-custom-request-id-123";
    let res = server
        .get("/api/health")
        .add_header("authorization", AUTH)
        .add_header("x-request-id", custom_id)
        .await;
    res.assert_status_ok();
    let returned_id = res
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert_eq!(
        returned_id, custom_id,
        "X-Request-ID in response should match the one sent in request"
    );
}
