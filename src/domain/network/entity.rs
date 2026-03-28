use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RuleType {
    Block,
    Allow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRule {
    pub domain: String,
    pub rule_type: RuleType,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkLog {
    pub timestamp: DateTime<Utc>,
    pub domain: String,
    pub rule_applied: Option<RuleType>,
    pub allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStatus {
    pub proxy_enabled: bool,
    pub blocked_count: usize,
    pub allowed_count: usize,
    pub total_requests: usize,
}
