use async_trait::async_trait;
use super::entity::Agent;

#[async_trait]
pub trait AgentRepository: Send + Sync {
    async fn list(&self) -> Vec<Agent>;
    async fn add(&self, agent: Agent);
    async fn delete(&self, id: &str) -> bool;
    async fn reset(&self, id: &str) -> bool;
}
