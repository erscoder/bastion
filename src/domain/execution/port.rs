use async_trait::async_trait;

#[derive(Debug)]
pub struct ExecResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

#[async_trait]
pub trait ExecutionPort: Send + Sync {
    async fn run(&self, command: &str, profile: &str, timeout_ms: u64) -> std::io::Result<ExecResult>;
    fn available(&self) -> bool;
}
