use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::RwLock;
use crate::domain::audit::{AuditLog, AuditRepository};

pub struct InMemoryAuditRepo {
    data: Arc<RwLock<Vec<AuditLog>>>,
}

impl InMemoryAuditRepo {
    pub fn new() -> Self {
        Self { data: Arc::new(RwLock::new(vec![])) }
    }
}

#[async_trait]
impl AuditRepository for InMemoryAuditRepo {
    async fn append(&self, log: AuditLog) {
        self.data.write().await.push(log);
    }

    async fn query(&self, limit: usize, offset: usize) -> (Vec<AuditLog>, usize) {
        let data = self.data.read().await;
        let total = data.len();
        let logs = data.iter().skip(offset).take(limit).cloned().collect();
        (logs, total)
    }
}
