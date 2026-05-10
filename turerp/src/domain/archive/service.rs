//! Archive service — business logic for data archiving

use crate::common::pagination::{PaginatedResult, PaginationParams};
use crate::domain::archive::model::{
    ArchiveJob, ArchiveJobStatus, ArchivePolicy, ArchiveRecord, CreateArchiveJob,
    CreateArchivePolicy, RestoreRequest, UpdateArchivePolicy,
};
use crate::domain::archive::repository::{
    BoxArchiveJobRepository, BoxArchivePolicyRepository, BoxArchiveRecordRepository,
};
use crate::error::ApiError;

/// Service for managing data archiving
#[derive(Clone)]
pub struct ArchiveService {
    policy_repo: BoxArchivePolicyRepository,
    job_repo: BoxArchiveJobRepository,
    record_repo: BoxArchiveRecordRepository,
}

impl ArchiveService {
    pub fn new(
        policy_repo: BoxArchivePolicyRepository,
        job_repo: BoxArchiveJobRepository,
        record_repo: BoxArchiveRecordRepository,
    ) -> Self {
        Self {
            policy_repo,
            job_repo,
            record_repo,
        }
    }

    // ---- Archive Policy Operations ----

    /// Create a new archive policy
    pub async fn create_policy(
        &self,
        create: CreateArchivePolicy,
        tenant_id: i64,
    ) -> Result<ArchivePolicy, ApiError> {
        if create.name.trim().is_empty() {
            return Err(ApiError::Validation(
                "Archive policy name is required".to_string(),
            ));
        }
        if create.table_name.trim().is_empty() {
            return Err(ApiError::Validation("Table name is required".to_string()));
        }
        if create.age_days <= 0 {
            return Err(ApiError::Validation(
                "Age threshold must be greater than 0 days".to_string(),
            ));
        }
        self.policy_repo.create(create, tenant_id).await
    }

    /// Get an archive policy by ID
    pub async fn get_policy(&self, id: i64, tenant_id: i64) -> Result<ArchivePolicy, ApiError> {
        self.policy_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Archive policy {} not found", id)))
    }

    /// List archive policies with pagination
    pub async fn list_policies(
        &self,
        tenant_id: i64,
        params: PaginationParams,
    ) -> Result<PaginatedResult<ArchivePolicy>, ApiError> {
        self.policy_repo.find_all(tenant_id, params).await
    }

    /// List active archive policies
    pub async fn list_active_policies(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<ArchivePolicy>, ApiError> {
        self.policy_repo.find_active(tenant_id).await
    }

    /// Update an archive policy
    pub async fn update_policy(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateArchivePolicy,
    ) -> Result<ArchivePolicy, ApiError> {
        if let Some(age_days) = update.age_days {
            if age_days <= 0 {
                return Err(ApiError::Validation(
                    "Age threshold must be greater than 0 days".to_string(),
                ));
            }
        }
        self.policy_repo.update(id, tenant_id, update).await
    }

    /// Delete an archive policy
    pub async fn delete_policy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.policy_repo.delete(id, tenant_id).await
    }

    // ---- Archive Job Operations ----

    /// Create and start a new archive job
    pub async fn create_job(
        &self,
        create: CreateArchiveJob,
        tenant_id: i64,
    ) -> Result<ArchiveJob, ApiError> {
        // Verify the policy exists
        self.policy_repo
            .find_by_id(create.policy_id, tenant_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!("Archive policy {} not found", create.policy_id))
            })?;

        let job = self.job_repo.create(create, tenant_id).await?;
        self.job_repo.start_job(job.id, tenant_id).await
    }

    /// Get an archive job by ID
    pub async fn get_job(&self, id: i64, tenant_id: i64) -> Result<ArchiveJob, ApiError> {
        self.job_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Archive job {} not found", id)))
    }

    /// List archive jobs with pagination
    pub async fn list_jobs(
        &self,
        tenant_id: i64,
        params: PaginationParams,
    ) -> Result<PaginatedResult<ArchiveJob>, ApiError> {
        self.job_repo.find_all(tenant_id, params).await
    }

    /// List archive jobs for a specific policy
    pub async fn list_jobs_by_policy(
        &self,
        policy_id: i64,
        tenant_id: i64,
        params: PaginationParams,
    ) -> Result<PaginatedResult<ArchiveJob>, ApiError> {
        self.job_repo
            .find_by_policy(policy_id, tenant_id, params)
            .await
    }

    /// Complete an archive job with results
    pub async fn complete_job(
        &self,
        id: i64,
        tenant_id: i64,
        records_archived: i64,
        records_failed: i64,
    ) -> Result<ArchiveJob, ApiError> {
        self.job_repo
            .update_status(
                id,
                tenant_id,
                ArchiveJobStatus::Completed,
                records_archived,
                records_failed,
                None,
            )
            .await
    }

    /// Fail an archive job with an error message
    pub async fn fail_job(
        &self,
        id: i64,
        tenant_id: i64,
        error_message: String,
    ) -> Result<ArchiveJob, ApiError> {
        self.job_repo
            .update_status(
                id,
                tenant_id,
                ArchiveJobStatus::Failed,
                0,
                0,
                Some(error_message),
            )
            .await
    }

    /// Delete an archive job
    pub async fn delete_job(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.job_repo.delete(id, tenant_id).await
    }

    // ---- Archive Record Operations ----

    /// Create an archive record
    pub async fn create_record(
        &self,
        tenant_id: i64,
        source_table: String,
        source_id: i64,
        archived_data: serde_json::Value,
        archive_job_id: i64,
    ) -> Result<ArchiveRecord, ApiError> {
        self.record_repo
            .create(
                tenant_id,
                source_table,
                source_id,
                archived_data,
                archive_job_id,
            )
            .await
    }

    /// Get an archive record by ID
    pub async fn get_record(&self, id: i64, tenant_id: i64) -> Result<ArchiveRecord, ApiError> {
        self.record_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Archive record {} not found", id)))
    }

    /// List archive records with optional filters
    pub async fn list_records(
        &self,
        tenant_id: i64,
        source_table: Option<String>,
        source_id: Option<i64>,
        params: PaginationParams,
    ) -> Result<PaginatedResult<ArchiveRecord>, ApiError> {
        self.record_repo
            .find_all(tenant_id, source_table, source_id, params)
            .await
    }

    /// Restore archived records
    pub async fn restore_records(
        &self,
        request: RestoreRequest,
        tenant_id: i64,
    ) -> Result<(Vec<ArchiveRecord>, Vec<(i64, String)>), ApiError> {
        let mut restored = Vec::new();
        let mut failed = Vec::new();
        for id in request.record_ids {
            match self.record_repo.restore(id, tenant_id).await {
                Ok(record) => restored.push(record),
                Err(e) => {
                    tracing::warn!("Failed to restore archive record {}: {}", id, e);
                    failed.push((id, e.to_string()));
                }
            }
        }
        Ok((restored, failed))
    }

    /// Permanently delete an archive record
    pub async fn delete_record(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.record_repo.delete(id, tenant_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::archive::repository::{
        InMemoryArchiveJobRepository, InMemoryArchivePolicyRepository,
        InMemoryArchiveRecordRepository,
    };
    use std::sync::Arc;

    fn make_service() -> ArchiveService {
        let policy_repo = Arc::new(InMemoryArchivePolicyRepository::new());
        let job_repo = Arc::new(InMemoryArchiveJobRepository::new());
        let record_repo = Arc::new(InMemoryArchiveRecordRepository::new());
        ArchiveService::new(policy_repo, job_repo, record_repo)
    }

    #[tokio::test]
    async fn test_create_and_get_policy() {
        let svc = make_service();

        let create = CreateArchivePolicy {
            name: "Old Invoices".to_string(),
            table_name: "invoices".to_string(),
            age_days: 365,
            conditions: None,
            is_active: true,
        };

        let policy = svc.create_policy(create, 1).await.unwrap();
        assert_eq!(policy.id, 1);
        assert_eq!(policy.table_name, "invoices");

        let found = svc.get_policy(policy.id, 1).await.unwrap();
        assert_eq!(found.id, policy.id);
    }

    #[tokio::test]
    async fn test_create_policy_validation() {
        let svc = make_service();

        let empty_name = CreateArchivePolicy {
            name: "".to_string(),
            table_name: "invoices".to_string(),
            age_days: 365,
            conditions: None,
            is_active: true,
        };
        assert!(svc.create_policy(empty_name, 1).await.is_err());

        let empty_table = CreateArchivePolicy {
            name: "Test".to_string(),
            table_name: "".to_string(),
            age_days: 365,
            conditions: None,
            is_active: true,
        };
        assert!(svc.create_policy(empty_table, 1).await.is_err());

        let zero_age = CreateArchivePolicy {
            name: "Test".to_string(),
            table_name: "invoices".to_string(),
            age_days: 0,
            conditions: None,
            is_active: true,
        };
        assert!(svc.create_policy(zero_age, 1).await.is_err());
    }

    #[tokio::test]
    async fn test_update_policy() {
        let svc = make_service();

        let create = CreateArchivePolicy {
            name: "Old Invoices".to_string(),
            table_name: "invoices".to_string(),
            age_days: 365,
            conditions: None,
            is_active: true,
        };
        let policy = svc.create_policy(create, 1).await.unwrap();

        let update = UpdateArchivePolicy {
            name: Some("Very Old Invoices".to_string()),
            table_name: None,
            age_days: Some(730),
            conditions: None,
            is_active: None,
        };
        let updated = svc.update_policy(policy.id, 1, update).await.unwrap();
        assert_eq!(updated.name, "Very Old Invoices");
        assert_eq!(updated.age_days, 730);
    }

    #[tokio::test]
    async fn test_archive_job_lifecycle() {
        let svc = make_service();

        // Create policy first
        let create_policy = CreateArchivePolicy {
            name: "Old Invoices".to_string(),
            table_name: "invoices".to_string(),
            age_days: 365,
            conditions: None,
            is_active: true,
        };
        let policy = svc.create_policy(create_policy, 1).await.unwrap();

        // Create and start job
        let create_job = CreateArchiveJob {
            policy_id: policy.id,
        };
        let job = svc.create_job(create_job, 1).await.unwrap();
        assert_eq!(job.status, ArchiveJobStatus::Running);
        assert!(job.started_at.is_some());

        // Complete job
        let completed = svc.complete_job(job.id, 1, 100, 0).await.unwrap();
        assert_eq!(completed.status, ArchiveJobStatus::Completed);
        assert_eq!(completed.records_archived, 100);
        assert!(completed.completed_at.is_some());
    }

    #[tokio::test]
    async fn test_fail_job() {
        let svc = make_service();

        let create_policy = CreateArchivePolicy {
            name: "Old Invoices".to_string(),
            table_name: "invoices".to_string(),
            age_days: 365,
            conditions: None,
            is_active: true,
        };
        let policy = svc.create_policy(create_policy, 1).await.unwrap();

        let create_job = CreateArchiveJob {
            policy_id: policy.id,
        };
        let job = svc.create_job(create_job, 1).await.unwrap();

        let failed = svc
            .fail_job(job.id, 1, "Database connection lost".to_string())
            .await
            .unwrap();
        assert_eq!(failed.status, ArchiveJobStatus::Failed);
        assert_eq!(
            failed.error_message,
            Some("Database connection lost".to_string())
        );
    }

    #[tokio::test]
    async fn test_create_and_restore_record() {
        let svc = make_service();

        let record = svc
            .create_record(
                1,
                "invoices".to_string(),
                42,
                serde_json::json!({"amount": 1000}),
                1,
            )
            .await
            .unwrap();
        assert_eq!(record.source_id, 42);
        assert!(record.restored_at.is_none());

        let request = RestoreRequest {
            record_ids: vec![record.id],
        };
        let (restored, failed) = svc.restore_records(request, 1).await.unwrap();
        assert_eq!(restored.len(), 1);
        assert!(restored[0].restored_at.is_some());
        assert!(failed.is_empty());
    }

    #[tokio::test]
    async fn test_list_records() {
        let svc = make_service();

        svc.create_record(1, "invoices".to_string(), 1, serde_json::json!({}), 1)
            .await
            .unwrap();
        svc.create_record(1, "invoices".to_string(), 2, serde_json::json!({}), 1)
            .await
            .unwrap();
        svc.create_record(1, "sales_orders".to_string(), 1, serde_json::json!({}), 1)
            .await
            .unwrap();

        let params = PaginationParams::default();
        let all = svc.list_records(1, None, None, params).await.unwrap();
        assert_eq!(all.items.len(), 3);

        let params = PaginationParams::default();
        let invoices = svc
            .list_records(1, Some("invoices".to_string()), None, params)
            .await
            .unwrap();
        assert_eq!(invoices.items.len(), 2);

        let params = PaginationParams::default();
        let by_id = svc.list_records(1, None, Some(1), params).await.unwrap();
        assert_eq!(by_id.items.len(), 2);
    }

    #[tokio::test]
    async fn test_job_requires_existing_policy() {
        let svc = make_service();

        let create_job = CreateArchiveJob { policy_id: 999 };
        let result = svc.create_job(create_job, 1).await;
        assert!(result.is_err());
    }
}
