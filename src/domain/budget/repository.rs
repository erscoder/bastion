use async_trait::async_trait;
use super::entity::{AgentBudget, Budget};

#[async_trait]
pub trait BudgetRepository: Send + Sync {
    async fn get(&self) -> Budget;
    async fn increment(&self);
    async fn increment_for_agent(&self, agent_id: &str);
    async fn get_by_agent(&self, agent_id: &str) -> Option<AgentBudget>;
    async fn reset(&self);
    async fn set_remaining(&self, n: u32);
}
