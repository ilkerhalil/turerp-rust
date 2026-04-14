//! Audit log repository
//!
//! Trait defining the repository interface for audit logs and an in-memory
//! implementation for development/testing.

use async_trait::async_trait;
use parking_lot::Mutex;
use std::sync::Arc;

use crate::common::pagination::PaginatedResult;
use crate::domain::audit::model::*;
use crate::error::ApiError;

/// Repository trait for AuditLog operations
#[async_trait]
pub trait AuditLogRepository: Send + Sync {
    /// Create a new audit log entry
    async fn create(&self, log: CreateAuditLog) -> Result<AuditLog, ApiError>;

    /// Find audit logs for a tenant with filtering and pagination
    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        query: AuditLogQueryParams,
    ) -> Result<PaginatedResult<AuditLog>, ApiError>;

    /// Create multiple audit log entries in batch (for channel-based writes)
    async fn create_batch(&self, logs: Vec<CreateAuditLog>) -> Result<(), ApiError>;
}

/// Type alias for a boxed, thread-safe audit log repository
pub type BoxAuditLogRepository = Arc<dyn AuditLogRepository>;

/// Inner state for InMemoryAuditLogRepository
struct InMemoryAuditLogInner {
    logs: std::collections::HashMap<i64, AuditLog>,
    next_id: i64,
}

/// In-memory audit log repository for development/testing
pub struct InMemoryAuditLogRepository {
    inner: Mutex<InMemoryAuditLogInner>,
}

impl InMemoryAuditLogRepository {
    /// Create a new in-memory audit log repository
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryAuditLogInner {
                logs: std::collections::HashMap::new(),
                next_id: 1,
            }),
        }
    }
}

impl Default for InMemoryAuditLogRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AuditLogRepository for InMemoryAuditLogRepository {
    async fn create(&self, log: CreateAuditLog) -> Result<AuditLog, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_id;
        inner.next_id += 1;

        let audit_log = AuditLog {
            id,
            tenant_id: log.tenant_id,
            user_id: log.user_id,
            username: log.username,
            action: log.action,
            path: log.path,
            status_code: log.status_code,
            request_id: log.request_id,
            ip_address: log.ip_address,
            user_agent: log.user_agent,
            created_at: log.created_at,
        };

        inner.logs.insert(id, audit_log.clone());
        Ok(audit_log)
    }

    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        query: AuditLogQueryParams,
    ) -> Result<PaginatedResult<AuditLog>, ApiError> {
        let inner = self.inner.lock();

        // Filter by tenant_id first, then apply optional filters
        let mut filtered: Vec<AuditLog> = inner
            .logs
            .values()
            .filter(|l| l.tenant_id == tenant_id)
            .cloned()
            .collect();

        // Apply optional user_id filter
        if let Some(user_id) = query.user_id {
            filtered.retain(|l| l.user_id == user_id);
        }

        // Apply optional path filter (substring match)
        if let Some(ref path) = query.path {
            let path_lower = path.to_lowercase();
            filtered.retain(|l| l.path.to_lowercase().contains(&path_lower));
        }

        // Apply optional from_date filter
        if let Some(from_date) = query.from_date {
            filtered.retain(|l| l.created_at >= from_date);
        }

        // Apply optional to_date filter
        if let Some(to_date) = query.to_date {
            filtered.retain(|l| l.created_at <= to_date);
        }

        // Sort by id for consistent ordering
        filtered.sort_by_key(|l| l.id);

        let total = filtered.len() as u64;
        let page = query.page.max(1);
        let per_page = query.per_page.max(1);

        let items: Vec<AuditLog> = filtered
            .into_iter()
            .skip(((page.saturating_sub(1)) * per_page) as usize)
            .take(per_page as usize)
            .collect();

        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn create_batch(&self, logs: Vec<CreateAuditLog>) -> Result<(), ApiError> {
        for log in logs {
            self.create(log).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn sample_create_log(tenant_id: i64, user_id: i64, action: &str, path: &str) -> CreateAuditLog {
        CreateAuditLog {
            tenant_id,
            user_id,
            username: "testuser".to_string(),
            action: action.to_string(),
            path: path.to_string(),
            status_code: 200,
            request_id: "req-test".to_string(),
            ip_address: Some("127.0.0.1".to_string()),
            user_agent: Some("test-agent".to_string()),
            created_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_create_audit_log() {
        let repo = InMemoryAuditLogRepository::new();
        let log = sample_create_log(1, 42, "GET", "/api/v1/users");

        let result = repo.create(log).await;
        assert!(result.is_ok());
        let created = result.unwrap();
        assert_eq!(created.id, 1);
        assert_eq!(created.tenant_id, 1);
        assert_eq!(created.user_id, 42);
        assert_eq!(created.action, "GET");
    }

    #[tokio::test]
    async fn test_find_by_tenant_paginated() {
        let repo = InMemoryAuditLogRepository::new();

        // Create logs for tenant 1
        repo.create(sample_create_log(1, 1, "GET", "/api/v1/users"))
            .await
            .unwrap();
        repo.create(sample_create_log(1, 2, "POST", "/api/v1/users"))
            .await
            .unwrap();
        // Create a log for tenant 2
        repo.create(sample_create_log(2, 1, "GET", "/api/v1/tenants"))
            .await
            .unwrap();

        let query = AuditLogQueryParams::default();
        let result = repo.find_by_tenant_paginated(1, query).await.unwrap();
        assert_eq!(result.items.len(), 2);
        assert_eq!(result.total, 2);
    }

    #[tokio::test]
    async fn test_find_by_tenant_with_user_id_filter() {
        let repo = InMemoryAuditLogRepository::new();

        repo.create(sample_create_log(1, 1, "GET", "/api/v1/users"))
            .await
            .unwrap();
        repo.create(sample_create_log(1, 2, "POST", "/api/v1/users"))
            .await
            .unwrap();

        let query = AuditLogQueryParams {
            user_id: Some(1),
            ..Default::default()
        };
        let result = repo.find_by_tenant_paginated(1, query).await.unwrap();
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].user_id, 1);
    }

    #[tokio::test]
    async fn test_find_by_tenant_with_path_filter() {
        let repo = InMemoryAuditLogRepository::new();

        repo.create(sample_create_log(1, 1, "GET", "/api/v1/users"))
            .await
            .unwrap();
        repo.create(sample_create_log(1, 1, "GET", "/api/v1/products"))
            .await
            .unwrap();

        let query = AuditLogQueryParams {
            path: Some("/users".to_string()),
            ..Default::default()
        };
        let result = repo.find_by_tenant_paginated(1, query).await.unwrap();
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].path, "/api/v1/users");
    }

    #[tokio::test]
    async fn test_find_by_tenant_with_date_filter() {
        let repo = InMemoryAuditLogRepository::new();

        let now = Utc::now();
        let past = now - chrono::Duration::hours(24);
        let future = now + chrono::Duration::hours(24);

        let log_past = CreateAuditLog {
            created_at: past,
            ..sample_create_log(1, 1, "GET", "/api/v1/old")
        };
        let log_now = CreateAuditLog {
            created_at: now,
            ..sample_create_log(1, 1, "GET", "/api/v1/current")
        };
        let log_future = CreateAuditLog {
            created_at: future,
            ..sample_create_log(1, 1, "GET", "/api/v1/future")
        };

        repo.create(log_past).await.unwrap();
        repo.create(log_now).await.unwrap();
        repo.create(log_future).await.unwrap();

        // Filter from 1 hour ago to 1 hour from now
        let query = AuditLogQueryParams {
            from_date: Some(now - chrono::Duration::hours(1)),
            to_date: Some(now + chrono::Duration::hours(1)),
            ..Default::default()
        };
        let result = repo.find_by_tenant_paginated(1, query).await.unwrap();
        assert_eq!(result.items.len(), 1);
    }

    #[tokio::test]
    async fn test_find_by_tenant_pagination() {
        let repo = InMemoryAuditLogRepository::new();

        // Create 5 logs
        for i in 0..5 {
            repo.create(sample_create_log(
                1,
                i + 1,
                "GET",
                &format!("/api/v1/resource/{}", i),
            ))
            .await
            .unwrap();
        }

        let query = AuditLogQueryParams {
            page: 1,
            per_page: 2,
            ..Default::default()
        };
        let result = repo.find_by_tenant_paginated(1, query).await.unwrap();
        assert_eq!(result.items.len(), 2);
        assert_eq!(result.total, 5);
        assert_eq!(result.total_pages, 3);
        assert!(result.has_next_page());
        assert!(!result.has_previous_page());
    }

    #[tokio::test]
    async fn test_create_batch() {
        let repo = InMemoryAuditLogRepository::new();

        let logs = vec![
            sample_create_log(1, 1, "GET", "/api/v1/a"),
            sample_create_log(1, 2, "POST", "/api/v1/b"),
            sample_create_log(1, 3, "DELETE", "/api/v1/c"),
        ];

        repo.create_batch(logs).await.unwrap();

        let query = AuditLogQueryParams::default();
        let result = repo.find_by_tenant_paginated(1, query).await.unwrap();
        assert_eq!(result.items.len(), 3);
        assert_eq!(result.total, 3);
    }
}
