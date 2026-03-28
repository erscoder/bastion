use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub commands_executed: u32,
    pub status: String,
}
