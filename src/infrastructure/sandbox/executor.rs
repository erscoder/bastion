use std::path::PathBuf;
use std::time::Duration;
use async_trait::async_trait;
use tracing::warn;
use crate::domain::execution::{ExecResult, ExecutionPort};

pub struct SandboxExecutor {
    pub profiles_dir: PathBuf,
}

impl SandboxExecutor {
    pub fn new(profiles_dir: PathBuf) -> Self {
        Self { profiles_dir }
    }

    pub fn sandbox_exec_available() -> bool {
        std::process::Command::new("which")
            .arg("sandbox-exec")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

#[async_trait]
impl ExecutionPort for SandboxExecutor {
    async fn run(&self, command: &str, profile: &str, timeout_ms: u64) -> std::io::Result<ExecResult> {
        let profile_path = self.profiles_dir.join(format!("{}.sb", profile));
        let use_sandbox = Self::sandbox_exec_available() && profile_path.exists();

        if !use_sandbox {
            if !Self::sandbox_exec_available() {
                warn!("sandbox-exec not found in PATH, falling back to sh -c");
            } else {
                warn!("Sandbox profile '{}' not found at {:?}, falling back to sh -c", profile, profile_path);
            }
        }

        let mut cmd = if use_sandbox {
            let mut c = tokio::process::Command::new("sandbox-exec");
            c.args(["-f", profile_path.to_str().unwrap_or(""), "sh", "-c", command]);
            c
        } else {
            let mut c = tokio::process::Command::new("sh");
            c.args(["-c", command]);
            c
        };

        let deadline = Duration::from_millis(timeout_ms);
        match tokio::time::timeout(deadline, cmd.output()).await {
            Ok(Ok(output)) => Ok(ExecResult {
                exit_code: output.status.code().unwrap_or(-1),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            }),
            Ok(Err(io_err)) => Err(io_err),
            Err(_elapsed) => Ok(ExecResult {
                exit_code: -1,
                stdout: String::new(),
                stderr: "Timeout".to_string(),
            }),
        }
    }

    fn available(&self) -> bool {
        Self::sandbox_exec_available()
    }
}
