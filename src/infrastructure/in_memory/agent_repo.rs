use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::RwLock;
use crate::domain::agent::{Agent, AgentRepository};

pub struct InMemoryAgentRepo {
    data: Arc<RwLock<Vec<Agent>>>,
}

impl InMemoryAgentRepo {
    pub fn new() -> Self {
        Self { data: Arc::new(RwLock::new(vec![])) }
    }
}

#[async_trait]
impl AgentRepository for InMemoryAgentRepo {
    async fn list(&self) -> Vec<Agent> {
        self.data.read().await.clone()
    }

    async fn add(&self, agent: Agent) {
        self.data.write().await.push(agent);
    }

    async fn delete(&self, id: &str) -> bool {
        let mut data = self.data.write().await;
        let before = data.len();
        data.retain(|a| a.id != id);
        data.len() < before
    }

    async fn reset(&self, id: &str) -> bool {
        let mut data = self.data.write().await;
        if let Some(agent) = data.iter_mut().find(|a| a.id == id) {
            agent.commands_executed = 0;
            true
        } else {
            false
        }
    }
}
