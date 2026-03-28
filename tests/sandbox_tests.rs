//! Integration / unit tests for SandboxExecutor (Sprint 2)

use std::path::PathBuf;

// Re-use the executor from the main crate.
use bastion::SandboxExecutor;
use bastion::ExecutionPort;

fn executor() -> SandboxExecutor {
    // Use the real sandbox profiles directory.
    let profiles_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sandbox");
    SandboxExecutor::new(profiles_dir)
}

// ─────────────────────────────────────────────
// Test 1 — basic command succeeds
// ─────────────────────────────────────────────
#[tokio::test]
async fn test_echo_hello() {
    let exec = executor();
    let result = exec.run("echo hello", "default", 5000).await.unwrap();
    assert_eq!(result.exit_code, 0, "exit_code should be 0");
    assert_eq!(result.stdout, "hello\n", "stdout should be 'hello\\n'");
    assert!(result.stderr.is_empty(), "stderr should be empty");
}

// ─────────────────────────────────────────────
// Test 2 — timeout fires for long-running process
// ─────────────────────────────────────────────
#[tokio::test]
async fn test_timeout() {
    let exec = executor();
    let result = exec.run("sleep 10", "default", 100).await.unwrap();
    assert_eq!(result.exit_code, -1, "exit_code should be -1 on timeout");
    let stderr_lower = result.stderr.to_lowercase();
    assert!(
        stderr_lower.contains("timeout"),
        "stderr should contain 'timeout', got: '{}'",
        result.stderr
    );
}

// ─────────────────────────────────────────────
// Test 3 — fallback when profile does not exist
// ─────────────────────────────────────────────
#[tokio::test]
async fn test_fallback_missing_profile() {
    let exec = executor();
    // "nonexistent_profile" has no .sb file → fallback to sh -c
    let result = exec.run("echo fallback", "nonexistent_profile", 5000).await.unwrap();
    assert_eq!(result.exit_code, 0);
    assert_eq!(result.stdout, "fallback\n");
}

// ─────────────────────────────────────────────
// Test 4 — non-zero exit code is preserved
// ─────────────────────────────────────────────
#[tokio::test]
async fn test_nonzero_exit() {
    let exec = executor();
    let result = exec.run("exit 42", "default", 5000).await.unwrap();
    assert_eq!(result.exit_code, 42);
}

// ─────────────────────────────────────────────
// Test 5 — stderr is captured
// ─────────────────────────────────────────────
#[tokio::test]
async fn test_stderr_capture() {
    let exec = executor();
    let result = exec.run("echo err >&2", "default", 5000).await.unwrap();
    assert_eq!(result.exit_code, 0);
    assert_eq!(result.stderr, "err\n");
}
