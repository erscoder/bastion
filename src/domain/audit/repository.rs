use async_trait::async_trait;
use super::entity::AuditLog;

#[async_trait]
pub trait AuditRepository: Send + Sync {
    async fn append(&self, log: AuditLog);
    async fn query(&self, limit: usize, offset: usize) -> (Vec<AuditLog>, usize);
}
