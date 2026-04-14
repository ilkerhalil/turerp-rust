//! Audit log service
//!
//! Business logic for creating and querying audit log entries.

use crate::common::pagination::PaginatedResult;
use crate::domain::audit::model::{AuditLog, AuditLogQueryParams, CreateAuditLog};
use crate::domain::audit::repository::BoxAuditLogRepository;
use crate::error::ApiError;

/// Audit log service
#[derive(Clone)]
pub struct AuditService {
    repo: BoxAuditLogRepository,
}

impl AuditService {
    /// Create a new audit service
    pub fn new(repo: BoxAuditLogRepository) -> Self {
        Self { repo }
    }

    /// Create a single audit log entry
    pub async fn create_log(&self, log: CreateAuditLog) -> Result<AuditLog, ApiError> {
        self.repo.create(log).await
    }

    /// Create multiple audit log entries in batch
    pub async fn create_batch(&self, logs: Vec<CreateAuditLog>) -> Result<(), ApiError> {
        self.repo.create_batch(logs).await
    }

    /// Get audit logs for a tenant with filtering and pagination
    pub async fn get_logs(
        &self,
        tenant_id: i64,
        query: AuditLogQueryParams,
    ) -> Result<PaginatedResult<AuditLog>, ApiError> {
        // Validate pagination parameters
        let pagination = crate::common::pagination::PaginationParams {
            page: query.page,
            per_page: query.per_page,
        };
        pagination.validate().map_err(ApiError::Validation)?;

        self.repo.find_by_tenant_paginated(tenant_id, query).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::audit::repository::InMemoryAuditLogRepository;
    use chrono::Utc;
    use std::sync::Arc;

    fn sample_create_log(tenant_id: i64, user_id: i64) -> CreateAuditLog {
        CreateAuditLog {
            tenant_id,
            user_id,
            username: "testuser".to_string(),
            action: "GET".to_string(),
            path: "/api/v1/test".to_string(),
            status_code: 200,
            request_id: "req-001".to_string(),
            ip_address: Some("127.0.0.1".to_string()),
            user_agent: Some("test-agent".to_string()),
            created_at: Utc::now(),
        }
    }

    fn create_service() -> AuditService {
        let repo = Arc::new(InMemoryAuditLogRepository::new()) as BoxAuditLogRepository;
        AuditService::new(repo)
    }

    #[tokio::test]
    async fn test_create_log() {
        let service = create_service();
        let log = sample_create_log(1, 42);

        let result = service.create_log(log).await;
        assert!(result.is_ok());
        let created = result.unwrap();
        assert_eq!(created.id, 1);
        assert_eq!(created.tenant_id, 1);
        assert_eq!(created.user_id, 42);
    }

    #[tokio::test]
    async fn test_create_batch() {
        let service = create_service();
        let logs = vec![
            sample_create_log(1, 1),
            sample_create_log(1, 2),
            sample_create_log(1, 3),
        ];

        let result = service.create_batch(logs).await;
        assert!(result.is_ok());

        let query = AuditLogQueryParams::default();
        let found = service.get_logs(1, query).await.unwrap();
        assert_eq!(found.items.len(), 3);
    }

    #[tokio::test]
    async fn test_get_logs() {
        let service = create_service();

        service.create_log(sample_create_log(1, 1)).await.unwrap();
        service.create_log(sample_create_log(1, 2)).await.unwrap();
        service.create_log(sample_create_log(2, 1)).await.unwrap();

        let query = AuditLogQueryParams::default();
        let result = service.get_logs(1, query).await.unwrap();
        assert_eq!(result.items.len(), 2);
        assert_eq!(result.total, 2);
    }

    #[tokio::test]
    async fn test_get_logs_with_user_filter() {
        let service = create_service();

        service.create_log(sample_create_log(1, 1)).await.unwrap();
        service.create_log(sample_create_log(1, 2)).await.unwrap();

        let query = AuditLogQueryParams {
            user_id: Some(1),
            ..Default::default()
        };
        let result = service.get_logs(1, query).await.unwrap();
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].user_id, 1);
    }

    #[tokio::test]
    async fn test_get_logs_pagination_validation() {
        let service = create_service();

        // page = 0 should fail validation
        let query = AuditLogQueryParams {
            page: 0,
            per_page: 20,
            ..Default::default()
        };
        let result = service.get_logs(1, query).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ApiError::Validation(_)));

        // per_page = 0 should fail validation
        let query = AuditLogQueryParams {
            page: 1,
            per_page: 0,
            ..Default::default()
        };
        let result = service.get_logs(1, query).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ApiError::Validation(_)));

        // per_page > 100 should fail validation
        let query = AuditLogQueryParams {
            page: 1,
            per_page: 101,
            ..Default::default()
        };
        let result = service.get_logs(1, query).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ApiError::Validation(_)));
    }

    #[tokio::test]
    async fn test_get_logs_tenant_isolation() {
        let service = create_service();

        service.create_log(sample_create_log(1, 1)).await.unwrap();
        service.create_log(sample_create_log(2, 1)).await.unwrap();

        let query = AuditLogQueryParams::default();
        let result = service.get_logs(1, query).await.unwrap();
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].tenant_id, 1);
    }
}
