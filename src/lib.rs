//! Bastion library — DDD architecture with Dependency Injection.

pub mod config;
pub mod domain;
pub mod infrastructure;
pub mod interfaces;

pub use config::Config;
pub use interfaces::http::{create_app, AppState};
pub use interfaces::http::state::SandboxProfile;
pub use infrastructure::sandbox::SandboxExecutor;
pub use infrastructure::mcp::BastionMcpServer;
pub use domain::execution::ExecutionPort;
pub use domain::budget::BudgetRepository;

use std::sync::Arc;
use tokio::sync::RwLock;
use infrastructure::in_memory::{InMemoryAgentRepo, InMemoryAuditRepo, InMemoryBudgetRepo, InMemoryNetworkRepo};

pub fn create_state(config: Config) -> AppState {
    AppState {
        executor: Arc::new(SandboxExecutor::new(config.profiles_dir.clone())),
        agents: Arc::new(InMemoryAgentRepo::new()),
        audit: Arc::new(InMemoryAuditRepo::new()),
        budget: Arc::new(InMemoryBudgetRepo::new()),
        network: Arc::new(InMemoryNetworkRepo::new()),
        sandbox_profiles: Arc::new(RwLock::new(vec![
            SandboxProfile {
                name: "default".to_string(),
                description: "Balanced security for general use".to_string(),
            },
            SandboxProfile {
                name: "strict".to_string(),
                description: "Maximum isolation".to_string(),
            },
            SandboxProfile {
                name: "permissive".to_string(),
                description: "Development mode".to_string(),
            },
        ])),
        start_time: std::time::Instant::now(),
        config,
    }
}
