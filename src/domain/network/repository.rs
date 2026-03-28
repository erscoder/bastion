use async_trait::async_trait;
use super::entity::{NetworkLog, NetworkRule, NetworkStatus};

#[async_trait]
pub trait NetworkRepository: Send + Sync {
    async fn add_rule(&self, rule: NetworkRule);
    async fn remove_rule(&self, domain: &str);
    async fn get_rules(&self) -> Vec<NetworkRule>;
    async fn log_request(&self, log: NetworkLog);
    async fn get_logs(&self, limit: usize) -> Vec<NetworkLog>;
    async fn get_status(&self) -> NetworkStatus;
}
