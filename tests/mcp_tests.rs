use bastion::{create_state, BastionMcpServer, Config};
use bastion::domain::mcp::protocol::{JsonRpcRequest, METHOD_NOT_FOUND};

fn make_server() -> BastionMcpServer {
    let sandbox = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sandbox");
    let config = Config {
        sandbox_dir: sandbox.clone(),
        profiles_dir: sandbox,
        ..Config::default()
    };
    let state = create_state(config);
    BastionMcpServer::new(state)
}

fn req(method: &str, params: serde_json::Value) -> JsonRpcRequest {
    JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(serde_json::json!(1)),
        method: method.to_string(),
        params,
    }
}

#[tokio::test]
async fn test_initialize_response() {
    let server = make_server();
    let resp = server.dispatch(req("initialize", serde_json::json!({}))).await;
    assert!(resp.error.is_none(), "Expected no error on initialize");
    let result = resp.result.expect("Expected result");
    assert!(result.get("serverInfo").is_some(), "serverInfo missing");
    assert!(result.get("capabilities").is_some(), "capabilities missing");
    assert_eq!(result["serverInfo"]["name"], "bastion");
}

#[tokio::test]
async fn test_tools_list() {
    let server = make_server();
    let resp = server.dispatch(req("tools/list", serde_json::json!({}))).await;
    assert!(resp.error.is_none(), "Expected no error");
    let tools = resp.result.unwrap()["tools"]
        .as_array()
        .expect("tools must be array")
        .clone();
    assert_eq!(tools.len(), 5, "Expected exactly 5 tools");

    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"bastion_exec"));
    assert!(names.contains(&"bastion_list_agents"));
    assert!(names.contains(&"bastion_get_audit"));
    assert!(names.contains(&"bastion_get_budget"));
    assert!(names.contains(&"bastion_network_status"));
}

#[tokio::test]
async fn test_tool_exec() {
    let server = make_server();
    let resp = server.dispatch(req("tools/call", serde_json::json!({
        "name": "bastion_exec",
        "arguments": { "command": "echo mcp" }
    }))).await;
    assert!(resp.error.is_none(), "Expected no error");
    let content_text = &resp.result.unwrap()["content"][0]["text"];
    let inner: serde_json::Value =
        serde_json::from_str(content_text.as_str().unwrap()).unwrap();
    assert_eq!(inner["exit_code"].as_i64().unwrap(), 0, "exit_code should be 0");
    assert!(inner["stdout"].as_str().unwrap().contains("mcp"), "stdout should contain 'mcp'");
}

#[tokio::test]
async fn test_tool_list_agents() {
    let server = make_server();
    let resp = server.dispatch(req("tools/call", serde_json::json!({
        "name": "bastion_list_agents",
        "arguments": {}
    }))).await;
    assert!(resp.error.is_none(), "Expected no error");
    let content_text = &resp.result.unwrap()["content"][0]["text"];
    let inner: serde_json::Value =
        serde_json::from_str(content_text.as_str().unwrap()).unwrap();
    assert_eq!(
        inner["agents"].as_array().unwrap().len(),
        0,
        "Expected empty agents list"
    );
}

#[tokio::test]
async fn test_tool_get_audit() {
    let server = make_server();
    let resp = server.dispatch(req("tools/call", serde_json::json!({
        "name": "bastion_get_audit",
        "arguments": {}
    }))).await;
    assert!(resp.error.is_none(), "Expected no error");
    let content_text = &resp.result.unwrap()["content"][0]["text"];
    let inner: serde_json::Value =
        serde_json::from_str(content_text.as_str().unwrap()).unwrap();
    assert_eq!(
        inner["logs"].as_array().unwrap().len(),
        0,
        "Expected empty logs"
    );
}

#[tokio::test]
async fn test_tool_get_budget() {
    let server = make_server();
    let resp = server.dispatch(req("tools/call", serde_json::json!({
        "name": "bastion_get_budget",
        "arguments": {}
    }))).await;
    assert!(resp.error.is_none(), "Expected no error");
    let content_text = &resp.result.unwrap()["content"][0]["text"];
    let inner: serde_json::Value =
        serde_json::from_str(content_text.as_str().unwrap()).unwrap();
    assert_eq!(
        inner["total_commands"].as_u64().unwrap(),
        0,
        "Expected total_commands=0"
    );
}

#[tokio::test]
async fn test_tool_network_status() {
    let server = make_server();
    let resp = server.dispatch(req("tools/call", serde_json::json!({
        "name": "bastion_network_status",
        "arguments": {}
    }))).await;
    assert!(resp.error.is_none(), "Expected no error");
    let content_text = &resp.result.unwrap()["content"][0]["text"];
    let inner: serde_json::Value =
        serde_json::from_str(content_text.as_str().unwrap()).unwrap();
    assert!(
        inner.get("proxy_enabled").is_some(),
        "proxy_enabled should be present"
    );
}

#[tokio::test]
async fn test_unknown_method() {
    let server = make_server();
    let resp = server.dispatch(req("foo", serde_json::json!({}))).await;
    assert!(resp.error.is_some(), "Expected error for unknown method");
    assert_eq!(
        resp.error.unwrap().code,
        METHOD_NOT_FOUND,
        "Expected METHOD_NOT_FOUND (-32601)"
    );
}

#[tokio::test]
async fn test_unknown_tool() {
    let server = make_server();
    let resp = server.dispatch(req("tools/call", serde_json::json!({
        "name": "nonexistent",
        "arguments": {}
    }))).await;
    assert!(resp.error.is_some(), "Expected error for unknown tool");
    assert_eq!(
        resp.error.unwrap().code,
        METHOD_NOT_FOUND,
        "Expected METHOD_NOT_FOUND for unknown tool"
    );
}

#[tokio::test]
async fn test_tools_call_missing_name() {
    let server = make_server();
    let resp = server.dispatch(req("tools/call", serde_json::json!({
        "arguments": {}
    }))).await;
    assert!(resp.error.is_some(), "Expected error when name is missing");
}
