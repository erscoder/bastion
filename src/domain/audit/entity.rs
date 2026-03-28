use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub timestamp: DateTime<Utc>,
    pub agent_id: String,
    pub command: String,
    pub exit_code: i32,
    pub duration_ms: u64,
    pub profile: String,
}
