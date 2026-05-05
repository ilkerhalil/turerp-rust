//! Database read replica routing
//!
//! Provides a `DbRouter` that routes read queries to replicas and write
//! queries to the master. Supports configurable read-after-write consistency
//! and health checking for replicas.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Database role
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DbRole {
    Master,
    Replica,
}

/// Replica health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplicaHealth {
    Healthy,
    Unhealthy,
    Unknown,
}

/// Replica node info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicaNode {
    pub id: String,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub health: ReplicaHealth,
    pub last_health_check: Option<DateTime<Utc>>,
    pub replication_lag_ms: Option<i64>,
    pub weight: u32,
}

/// Query type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryType {
    Read,
    Write,
}

/// Routing decision
#[derive(Debug, Clone)]
pub struct RoutingDecision {
    pub target: String,
    pub role: DbRole,
    pub reason: String,
}

/// Read-after-write consistency mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReadAfterWriteMode {
    /// Always route reads to master after a write within the same session
    Session,
    /// Route reads to replicas unless stale data is unacceptable
    Eventual,
    /// Always route reads to master (no replication lag risk)
    Strict,
}

/// Database router trait
#[async_trait::async_trait]
pub trait DbRouter: Send + Sync {
    /// Route a query to the appropriate database
    async fn route(&self, query_type: QueryType, session_id: Option<&str>) -> RoutingDecision;

    /// Mark a session as having performed a write (for read-after-write consistency)
    async fn mark_session_write(&self, session_id: &str);

    /// Clear session write marker
    async fn clear_session(&self, session_id: &str);

    /// Get replica health status
    async fn replica_health(&self) -> Vec<ReplicaNode>;

    /// Get router stats
    async fn stats(&self) -> RouterStats;
}

/// Type alias for boxed db router
pub type BoxDbRouter = Arc<dyn DbRouter>;

/// Router statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterStats {
    pub total_reads: u64,
    pub total_writes: u64,
    pub reads_to_master: u64,
    pub reads_to_replica: u64,
    pub active_replicas: u32,
    pub total_replicas: u32,
}

/// In-memory database router
pub struct InMemoryDbRouter {
    master: String,
    replicas: parking_lot::RwLock<Vec<ReplicaNode>>,
    session_writes: parking_lot::RwLock<std::collections::HashSet<String>>,
    read_after_write: ReadAfterWriteMode,
    stats: parking_lot::RwLock<RouterStats>,
    round_robin: parking_lot::RwLock<usize>,
}

impl InMemoryDbRouter {
    pub fn new(master: &str, read_after_write: ReadAfterWriteMode) -> Self {
        Self {
            master: master.to_string(),
            replicas: parking_lot::RwLock::new(Vec::new()),
            session_writes: parking_lot::RwLock::new(std::collections::HashSet::new()),
            read_after_write,
            stats: parking_lot::RwLock::new(RouterStats {
                total_reads: 0,
                total_writes: 0,
                reads_to_master: 0,
                reads_to_replica: 0,
                active_replicas: 0,
                total_replicas: 0,
            }),
            round_robin: parking_lot::RwLock::new(0),
        }
    }

    pub fn add_replica(&self, replica: ReplicaNode) {
        let mut replicas = self.replicas.write();
        let mut stats = self.stats.write();
        replicas.push(replica);
        stats.total_replicas = replicas.len() as u32;
        stats.active_replicas = replicas
            .iter()
            .filter(|r| r.health == ReplicaHealth::Healthy)
            .count() as u32;
    }

    pub fn set_replica_health(&self, replica_id: &str, health: ReplicaHealth) {
        let mut replicas = self.replicas.write();
        let mut stats = self.stats.write();
        if let Some(r) = replicas.iter_mut().find(|r| r.id == replica_id) {
            r.health = health;
            r.last_health_check = Some(Utc::now());
        }
        stats.active_replicas = replicas
            .iter()
            .filter(|r| r.health == ReplicaHealth::Healthy)
            .count() as u32;
        stats.total_replicas = replicas.len() as u32;
    }

    fn pick_replica(&self) -> Option<String> {
        let replicas = self.replicas.read();
        let healthy: Vec<&ReplicaNode> = replicas
            .iter()
            .filter(|r| r.health == ReplicaHealth::Healthy)
            .collect();
        if healthy.is_empty() {
            return None;
        }
        let mut idx = self.round_robin.write();
        *idx = (*idx + 1) % healthy.len();
        Some(healthy[*idx].id.clone())
    }

    fn should_route_to_master(&self, session_id: Option<&str>) -> bool {
        match self.read_after_write {
            ReadAfterWriteMode::Strict => true,
            ReadAfterWriteMode::Session => {
                if let Some(sid) = session_id {
                    self.session_writes.read().contains(sid)
                } else {
                    false
                }
            }
            ReadAfterWriteMode::Eventual => false,
        }
    }
}

impl Default for InMemoryDbRouter {
    fn default() -> Self {
        Self::new("localhost:5432/turerp", ReadAfterWriteMode::Eventual)
    }
}

#[async_trait::async_trait]
impl DbRouter for InMemoryDbRouter {
    async fn route(&self, query_type: QueryType, session_id: Option<&str>) -> RoutingDecision {
        let mut stats = self.stats.write();
        match query_type {
            QueryType::Write => {
                stats.total_writes += 1;
                if let Some(sid) = session_id {
                    self.session_writes.write().insert(sid.to_string());
                }
                RoutingDecision {
                    target: self.master.clone(),
                    role: DbRole::Master,
                    reason: "Write query routed to master".to_string(),
                }
            }
            QueryType::Read => {
                stats.total_reads += 1;
                if self.should_route_to_master(session_id) {
                    stats.reads_to_master += 1;
                    RoutingDecision {
                        target: self.master.clone(),
                        role: DbRole::Master,
                        reason: "Read routed to master (read-after-write consistency)".to_string(),
                    }
                } else if let Some(replica_id) = self.pick_replica() {
                    stats.reads_to_replica += 1;
                    RoutingDecision {
                        target: replica_id,
                        role: DbRole::Replica,
                        reason: "Read query routed to replica".to_string(),
                    }
                } else {
                    stats.reads_to_master += 1;
                    RoutingDecision {
                        target: self.master.clone(),
                        role: DbRole::Master,
                        reason: "No healthy replicas, read routed to master".to_string(),
                    }
                }
            }
        }
    }

    async fn mark_session_write(&self, session_id: &str) {
        self.session_writes.write().insert(session_id.to_string());
    }

    async fn clear_session(&self, session_id: &str) {
        self.session_writes.write().remove(session_id);
    }

    async fn replica_health(&self) -> Vec<ReplicaNode> {
        self.replicas.read().clone()
    }

    async fn stats(&self) -> RouterStats {
        self.stats.read().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_router() -> InMemoryDbRouter {
        let router = InMemoryDbRouter::new("master:5432/db", ReadAfterWriteMode::Session);
        router.add_replica(ReplicaNode {
            id: "replica-1".to_string(),
            host: "replica1".to_string(),
            port: 5432,
            database: "turerp".to_string(),
            health: ReplicaHealth::Healthy,
            last_health_check: None,
            replication_lag_ms: Some(10),
            weight: 1,
        });
        router.add_replica(ReplicaNode {
            id: "replica-2".to_string(),
            host: "replica2".to_string(),
            port: 5432,
            database: "turerp".to_string(),
            health: ReplicaHealth::Healthy,
            last_health_check: None,
            replication_lag_ms: Some(25),
            weight: 1,
        });
        router
    }

    #[tokio::test]
    async fn test_write_goes_to_master() {
        let router = create_router();
        let decision = router.route(QueryType::Write, None).await;
        assert_eq!(decision.role, DbRole::Master);
        assert_eq!(decision.target, "master:5432/db");
    }

    #[tokio::test]
    async fn test_read_goes_to_replica() {
        let router = create_router();
        let decision = router.route(QueryType::Read, None).await;
        assert_eq!(decision.role, DbRole::Replica);
    }

    #[tokio::test]
    async fn test_read_after_write_session() {
        let router = create_router();
        // Write first
        router.route(QueryType::Write, Some("sess-1")).await;
        // Then read should go to master (same session)
        let decision = router.route(QueryType::Read, Some("sess-1")).await;
        assert_eq!(decision.role, DbRole::Master);
    }

    #[tokio::test]
    async fn test_read_different_session() {
        let router = create_router();
        router.route(QueryType::Write, Some("sess-1")).await;
        let decision = router.route(QueryType::Read, Some("sess-2")).await;
        assert_eq!(decision.role, DbRole::Replica);
    }

    #[tokio::test]
    async fn test_clear_session() {
        let router = create_router();
        router.route(QueryType::Write, Some("sess-1")).await;
        router.clear_session("sess-1").await;
        let decision = router.route(QueryType::Read, Some("sess-1")).await;
        assert_eq!(decision.role, DbRole::Replica);
    }

    #[tokio::test]
    async fn test_no_healthy_replicas_fallback() {
        let router = InMemoryDbRouter::new("master:5432/db", ReadAfterWriteMode::Eventual);
        // No replicas added
        let decision = router.route(QueryType::Read, None).await;
        assert_eq!(decision.role, DbRole::Master);
    }

    #[tokio::test]
    async fn test_strict_mode() {
        let router = InMemoryDbRouter::new("master:5432/db", ReadAfterWriteMode::Strict);
        router.add_replica(ReplicaNode {
            id: "replica-1".to_string(),
            host: "replica1".to_string(),
            port: 5432,
            database: "turerp".to_string(),
            health: ReplicaHealth::Healthy,
            last_health_check: None,
            replication_lag_ms: None,
            weight: 1,
        });
        let decision = router.route(QueryType::Read, None).await;
        assert_eq!(decision.role, DbRole::Master);
    }

    #[tokio::test]
    async fn test_replica_health_tracking() {
        let router = create_router();
        router.set_replica_health("replica-1", ReplicaHealth::Unhealthy);

        let health = router.replica_health().await;
        assert_eq!(health.len(), 2);
        assert_eq!(health[0].health, ReplicaHealth::Unhealthy);
        assert_eq!(health[1].health, ReplicaHealth::Healthy);
    }

    #[tokio::test]
    async fn test_router_stats() {
        let router = create_router();
        router.route(QueryType::Write, None).await;
        router.route(QueryType::Read, None).await;
        router.route(QueryType::Read, None).await;

        let stats = router.stats().await;
        assert_eq!(stats.total_writes, 1);
        assert_eq!(stats.total_reads, 2);
        assert_eq!(stats.reads_to_replica, 2);
        assert_eq!(stats.active_replicas, 2);
    }
}
