use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    pub total_commands: u32,
    pub commands_remaining: u32,
}

impl Default for Budget {
    fn default() -> Self {
        Self {
            total_commands: 0,
            commands_remaining: 100,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentBudget {
    pub agent_id: String,
    pub commands_executed: u32,
    pub commands_remaining: u32,
}
