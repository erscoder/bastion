use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::RwLock;
use crate::domain::network::{NetworkLog, NetworkRepository, NetworkRule, NetworkStatus, RuleType};

pub struct InMemoryNetworkRepo {
    rules: Arc<RwLock<Vec<NetworkRule>>>,
    logs: Arc<RwLock<Vec<NetworkLog>>>,
}

impl InMemoryNetworkRepo {
    pub fn new() -> Self {
        Self {
            rules: Arc::new(RwLock::new(Vec::new())),
            logs: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

#[async_trait]
impl NetworkRepository for InMemoryNetworkRepo {
    async fn add_rule(&self, rule: NetworkRule) {
        let mut rules = self.rules.write().await;
        // Remove existing rule for same domain if present
        rules.retain(|r| r.domain != rule.domain);
        rules.push(rule);
    }

    async fn remove_rule(&self, domain: &str) {
        let mut rules = self.rules.write().await;
        rules.retain(|r| r.domain != domain);
    }

    async fn get_rules(&self) -> Vec<NetworkRule> {
        self.rules.read().await.clone()
    }

    async fn log_request(&self, log: NetworkLog) {
        let mut logs = self.logs.write().await;
        logs.push(log);
    }

    async fn get_logs(&self, limit: usize) -> Vec<NetworkLog> {
        let logs = self.logs.read().await;
        let start = if logs.len() > limit { logs.len() - limit } else { 0 };
        logs[start..].to_vec()
    }

    async fn get_status(&self) -> NetworkStatus {
        let rules = self.rules.read().await;
        let logs = self.logs.read().await;
        let blocked_count = rules.iter().filter(|r| r.rule_type == RuleType::Block).count();
        let allowed_count = rules.iter().filter(|r| r.rule_type == RuleType::Allow).count();
        NetworkStatus {
            proxy_enabled: true,
            blocked_count,
            allowed_count,
            total_requests: logs.len(),
        }
    }
}

impl Default for InMemoryNetworkRepo {
    fn default() -> Self {
        Self::new()
    }
}
