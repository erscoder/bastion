use std::sync::Arc;
use tokio::sync::RwLock;
use crate::config::Config;
use crate::domain::agent::AgentRepository;
use crate::domain::audit::AuditRepository;
use crate::domain::budget::BudgetRepository;
use crate::domain::execution::ExecutionPort;
use crate::domain::network::NetworkRepository;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxProfile {
    pub name: String,
    pub description: String,
}

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub agents: Arc<dyn AgentRepository>,
    pub audit: Arc<dyn AuditRepository>,
    pub budget: Arc<dyn BudgetRepository>,
    pub executor: Arc<dyn ExecutionPort>,
    pub network: Arc<dyn NetworkRepository>,
    pub sandbox_profiles: Arc<RwLock<Vec<SandboxProfile>>>,
    pub start_time: std::time::Instant,
}
