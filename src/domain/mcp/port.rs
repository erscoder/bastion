use async_trait::async_trait;
use super::protocol::{JsonRpcRequest, JsonRpcResponse};

#[async_trait]
pub trait McpHandler: Send + Sync {
    async fn dispatch(&self, req: JsonRpcRequest) -> JsonRpcResponse;
}
