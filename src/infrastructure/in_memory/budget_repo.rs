use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::RwLock;
use crate::domain::budget::{AgentBudget, Budget, BudgetRepository};

pub struct InMemoryBudgetRepo {
    data: Arc<RwLock<Budget>>,
    per_agent: Arc<RwLock<HashMap<String, u32>>>,
}

impl InMemoryBudgetRepo {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(Budget::default())),
            per_agent: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl BudgetRepository for InMemoryBudgetRepo {
    async fn get(&self) -> Budget {
        self.data.read().await.clone()
    }

    async fn increment(&self) {
        let mut b = self.data.write().await;
        b.total_commands += 1;
        b.commands_remaining = b.commands_remaining.saturating_sub(1);
    }

    async fn increment_for_agent(&self, agent_id: &str) {
        // Increment the global budget
        let mut b = self.data.write().await;
        b.total_commands += 1;
        b.commands_remaining = b.commands_remaining.saturating_sub(1);
        drop(b);

        // Track per-agent executions
        let mut per = self.per_agent.write().await;
        let count = per.entry(agent_id.to_string()).or_insert(0);
        *count += 1;
    }

    async fn get_by_agent(&self, agent_id: &str) -> Option<AgentBudget> {
        let per = self.per_agent.read().await;
        let executed = *per.get(agent_id).unwrap_or(&0);
        let budget = self.data.read().await;
        // Per-agent remaining proportional, but simplified: share global remaining
        Some(AgentBudget {
            agent_id: agent_id.to_string(),
            commands_executed: executed,
            commands_remaining: budget.commands_remaining,
        })
    }

    async fn reset(&self) {
        let mut b = self.data.write().await;
        b.total_commands = 0;
        b.commands_remaining = 100;
        drop(b);
        let mut per = self.per_agent.write().await;
        per.clear();
    }

    async fn set_remaining(&self, n: u32) {
        let mut b = self.data.write().await;
        b.commands_remaining = n;
    }
}
