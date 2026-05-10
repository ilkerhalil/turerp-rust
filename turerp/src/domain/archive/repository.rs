//! Archive repository traits and in-memory implementations

use async_trait::async_trait;
use chrono::Utc;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use crate::common::pagination::{PaginatedResult, PaginationParams};
use crate::domain::archive::model::{
    ArchiveJob, ArchiveJobStatus, ArchivePolicy, ArchiveRecord, CreateArchiveJob,
    CreateArchivePolicy, UpdateArchivePolicy,
};
use crate::error::ApiError;

// ---------------------------------------------------------------------------
// ArchivePolicyRepository
// ---------------------------------------------------------------------------

/// Repository trait for archive policy operations
#[async_trait]
pub trait ArchivePolicyRepository: Send + Sync {
    /// Create a new archive policy
    async fn create(
        &self,
        policy: CreateArchivePolicy,
        tenant_id: i64,
    ) -> Result<ArchivePolicy, ApiError>;

    /// Find an archive policy by ID
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<ArchivePolicy>, ApiError>;

    /// Find all archive policies for a tenant
    async fn find_all(
        &self,
        tenant_id: i64,
        params: PaginationParams,
    ) -> Result<PaginatedResult<ArchivePolicy>, ApiError>;

    /// Find active archive policies for a tenant
    async fn find_active(&self, tenant_id: i64) -> Result<Vec<ArchivePolicy>, ApiError>;

    /// Update an archive policy
    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateArchivePolicy,
    ) -> Result<ArchivePolicy, ApiError>;

    /// Delete an archive policy
    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
}

/// Type alias for boxed ArchivePolicyRepository
pub type BoxArchivePolicyRepository = Arc<dyn ArchivePolicyRepository>;

// ---------------------------------------------------------------------------
// InMemoryArchivePolicyRepository
// ---------------------------------------------------------------------------

struct PolicyInner {
    policies: HashMap<i64, ArchivePolicy>,
    next_id: AtomicI64,
}

/// In-memory archive policy repository for testing and development
pub struct InMemoryArchivePolicyRepository {
    inner: Mutex<PolicyInner>,
}

impl InMemoryArchivePolicyRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(PolicyInner {
                policies: HashMap::new(),
                next_id: AtomicI64::new(1),
            }),
        }
    }
}

impl Default for InMemoryArchivePolicyRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ArchivePolicyRepository for InMemoryArchivePolicyRepository {
    async fn create(
        &self,
        create: CreateArchivePolicy,
        tenant_id: i64,
    ) -> Result<ArchivePolicy, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_id.fetch_add(1, Ordering::SeqCst);
        let now = Utc::now();

        let policy = ArchivePolicy {
            id,
            tenant_id,
            name: create.name,
            table_name: create.table_name,
            age_days: create.age_days,
            conditions: create.conditions,
            is_active: create.is_active,
            created_at: now,
            updated_at: None,
        };

        inner.policies.insert(id, policy.clone());
        Ok(policy)
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<ArchivePolicy>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .policies
            .get(&id)
            .filter(|p| p.tenant_id == tenant_id)
            .cloned())
    }

    async fn find_all(
        &self,
        tenant_id: i64,
        params: PaginationParams,
    ) -> Result<PaginatedResult<ArchivePolicy>, ApiError> {
        let inner = self.inner.lock();
        let mut items: Vec<ArchivePolicy> = inner
            .policies
            .values()
            .filter(|p| p.tenant_id == tenant_id)
            .cloned()
            .collect();

        items.sort_by_key(|a| a.id);
        let total = items.len() as u64;
        let start = (params.page.saturating_sub(1)) * params.per_page;
        let paginated: Vec<ArchivePolicy> = items
            .into_iter()
            .skip(start as usize)
            .take(params.per_page as usize)
            .collect();
        Ok(PaginatedResult::new(
            paginated,
            params.page,
            params.per_page,
            total,
        ))
    }

    async fn find_active(&self, tenant_id: i64) -> Result<Vec<ArchivePolicy>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .policies
            .values()
            .filter(|p| p.tenant_id == tenant_id && p.is_active)
            .cloned()
            .collect())
    }

    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateArchivePolicy,
    ) -> Result<ArchivePolicy, ApiError> {
        let mut inner = self.inner.lock();

        let policy = inner
            .policies
            .get_mut(&id)
            .filter(|p| p.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Archive policy {} not found", id)))?;

        if let Some(name) = update.name {
            policy.name = name;
        }
        if let Some(table_name) = update.table_name {
            policy.table_name = table_name;
        }
        if let Some(age_days) = update.age_days {
            policy.age_days = age_days;
        }
        if let Some(conditions) = update.conditions {
            policy.conditions = Some(conditions);
        }
        if let Some(is_active) = update.is_active {
            policy.is_active = is_active;
        }
        policy.updated_at = Some(Utc::now());

        Ok(policy.clone())
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();

        let len_before = inner.policies.len();
        inner
            .policies
            .retain(|_, p| !(p.id == id && p.tenant_id == tenant_id));

        if inner.policies.len() == len_before {
            return Err(ApiError::NotFound(format!(
                "Archive policy {} not found",
                id
            )));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// ArchiveJobRepository
// ---------------------------------------------------------------------------

/// Repository trait for archive job operations
#[async_trait]
pub trait ArchiveJobRepository: Send + Sync {
    /// Create a new archive job
    async fn create(&self, job: CreateArchiveJob, tenant_id: i64) -> Result<ArchiveJob, ApiError>;

    /// Find an archive job by ID
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<ArchiveJob>, ApiError>;

    /// Find all archive jobs for a tenant
    async fn find_all(
        &self,
        tenant_id: i64,
        params: PaginationParams,
    ) -> Result<PaginatedResult<ArchiveJob>, ApiError>;

    /// Find jobs by policy ID
    async fn find_by_policy(
        &self,
        policy_id: i64,
        tenant_id: i64,
        params: PaginationParams,
    ) -> Result<PaginatedResult<ArchiveJob>, ApiError>;

    /// Update archive job status
    async fn update_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: ArchiveJobStatus,
        records_archived: i64,
        records_failed: i64,
        error_message: Option<String>,
    ) -> Result<ArchiveJob, ApiError>;

    /// Start an archive job
    async fn start_job(&self, id: i64, tenant_id: i64) -> Result<ArchiveJob, ApiError>;

    /// Delete an archive job
    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
}

/// Type alias for boxed ArchiveJobRepository
pub type BoxArchiveJobRepository = Arc<dyn ArchiveJobRepository>;

// ---------------------------------------------------------------------------
// InMemoryArchiveJobRepository
// ---------------------------------------------------------------------------

struct JobInner {
    jobs: HashMap<i64, ArchiveJob>,
    next_id: AtomicI64,
}

/// In-memory archive job repository for testing and development
pub struct InMemoryArchiveJobRepository {
    inner: Mutex<JobInner>,
}

impl InMemoryArchiveJobRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(JobInner {
                jobs: HashMap::new(),
                next_id: AtomicI64::new(1),
            }),
        }
    }
}

impl Default for InMemoryArchiveJobRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ArchiveJobRepository for InMemoryArchiveJobRepository {
    async fn create(
        &self,
        create: CreateArchiveJob,
        tenant_id: i64,
    ) -> Result<ArchiveJob, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_id.fetch_add(1, Ordering::SeqCst);
        let now = Utc::now();

        let job = ArchiveJob {
            id,
            tenant_id,
            policy_id: create.policy_id,
            status: ArchiveJobStatus::Pending,
            started_at: None,
            completed_at: None,
            records_archived: 0,
            records_failed: 0,
            error_message: None,
            created_at: now,
        };

        inner.jobs.insert(id, job.clone());
        Ok(job)
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<ArchiveJob>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .jobs
            .get(&id)
            .filter(|j| j.tenant_id == tenant_id)
            .cloned())
    }

    async fn find_all(
        &self,
        tenant_id: i64,
        params: PaginationParams,
    ) -> Result<PaginatedResult<ArchiveJob>, ApiError> {
        let inner = self.inner.lock();
        let mut items: Vec<ArchiveJob> = inner
            .jobs
            .values()
            .filter(|j| j.tenant_id == tenant_id)
            .cloned()
            .collect();

        items.sort_by_key(|a| a.id);
        let total = items.len() as u64;
        let start = (params.page.saturating_sub(1)) * params.per_page;
        let paginated: Vec<ArchiveJob> = items
            .into_iter()
            .skip(start as usize)
            .take(params.per_page as usize)
            .collect();
        Ok(PaginatedResult::new(
            paginated,
            params.page,
            params.per_page,
            total,
        ))
    }

    async fn find_by_policy(
        &self,
        policy_id: i64,
        tenant_id: i64,
        params: PaginationParams,
    ) -> Result<PaginatedResult<ArchiveJob>, ApiError> {
        let inner = self.inner.lock();
        let mut items: Vec<ArchiveJob> = inner
            .jobs
            .values()
            .filter(|j| j.tenant_id == tenant_id && j.policy_id == policy_id)
            .cloned()
            .collect();

        items.sort_by_key(|a| a.id);
        let total = items.len() as u64;
        let start = (params.page.saturating_sub(1)) * params.per_page;
        let paginated: Vec<ArchiveJob> = items
            .into_iter()
            .skip(start as usize)
            .take(params.per_page as usize)
            .collect();
        Ok(PaginatedResult::new(
            paginated,
            params.page,
            params.per_page,
            total,
        ))
    }

    async fn update_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: ArchiveJobStatus,
        records_archived: i64,
        records_failed: i64,
        error_message: Option<String>,
    ) -> Result<ArchiveJob, ApiError> {
        let mut inner = self.inner.lock();

        let job = inner
            .jobs
            .get_mut(&id)
            .filter(|j| j.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Archive job {} not found", id)))?;

        job.status = status;
        job.records_archived = records_archived;
        job.records_failed = records_failed;
        job.error_message = error_message;
        if job.status == ArchiveJobStatus::Completed || job.status == ArchiveJobStatus::Failed {
            job.completed_at = Some(Utc::now());
        }

        Ok(job.clone())
    }

    async fn start_job(&self, id: i64, tenant_id: i64) -> Result<ArchiveJob, ApiError> {
        let mut inner = self.inner.lock();

        let job = inner
            .jobs
            .get_mut(&id)
            .filter(|j| j.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Archive job {} not found", id)))?;

        if job.status != ArchiveJobStatus::Pending {
            return Err(ApiError::Conflict(format!(
                "Archive job {} is not pending",
                id
            )));
        }

        job.status = ArchiveJobStatus::Running;
        job.started_at = Some(Utc::now());

        Ok(job.clone())
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();

        let len_before = inner.jobs.len();
        inner
            .jobs
            .retain(|_, j| !(j.id == id && j.tenant_id == tenant_id));

        if inner.jobs.len() == len_before {
            return Err(ApiError::NotFound(format!("Archive job {} not found", id)));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// ArchiveRecordRepository
// ---------------------------------------------------------------------------

/// Repository trait for archive record operations
#[async_trait]
pub trait ArchiveRecordRepository: Send + Sync {
    /// Create an archive record
    async fn create(
        &self,
        tenant_id: i64,
        source_table: String,
        source_id: i64,
        archived_data: serde_json::Value,
        archive_job_id: i64,
    ) -> Result<ArchiveRecord, ApiError>;

    /// Find an archive record by ID
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<ArchiveRecord>, ApiError>;

    /// Find archive records with optional filters and pagination
    async fn find_all(
        &self,
        tenant_id: i64,
        source_table: Option<String>,
        source_id: Option<i64>,
        params: PaginationParams,
    ) -> Result<PaginatedResult<ArchiveRecord>, ApiError>;

    /// Restore an archived record
    async fn restore(&self, id: i64, tenant_id: i64) -> Result<ArchiveRecord, ApiError>;

    /// Delete an archive record permanently
    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
}

/// Type alias for boxed ArchiveRecordRepository
pub type BoxArchiveRecordRepository = Arc<dyn ArchiveRecordRepository>;

// ---------------------------------------------------------------------------
// InMemoryArchiveRecordRepository
// ---------------------------------------------------------------------------

struct RecordInner {
    records: HashMap<i64, ArchiveRecord>,
    next_id: AtomicI64,
}

/// In-memory archive record repository for testing and development
pub struct InMemoryArchiveRecordRepository {
    inner: Mutex<RecordInner>,
}

impl InMemoryArchiveRecordRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(RecordInner {
                records: HashMap::new(),
                next_id: AtomicI64::new(1),
            }),
        }
    }
}

impl Default for InMemoryArchiveRecordRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ArchiveRecordRepository for InMemoryArchiveRecordRepository {
    async fn create(
        &self,
        tenant_id: i64,
        source_table: String,
        source_id: i64,
        archived_data: serde_json::Value,
        archive_job_id: i64,
    ) -> Result<ArchiveRecord, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_id.fetch_add(1, Ordering::SeqCst);
        let now = Utc::now();

        let record = ArchiveRecord {
            id,
            tenant_id,
            source_table,
            source_id,
            archived_data,
            archived_at: now,
            archive_job_id,
            restored_at: None,
        };

        inner.records.insert(id, record.clone());
        Ok(record)
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<ArchiveRecord>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .records
            .get(&id)
            .filter(|r| r.tenant_id == tenant_id)
            .cloned())
    }

    async fn find_all(
        &self,
        tenant_id: i64,
        source_table: Option<String>,
        source_id: Option<i64>,
        params: PaginationParams,
    ) -> Result<PaginatedResult<ArchiveRecord>, ApiError> {
        let inner = self.inner.lock();
        let mut items: Vec<ArchiveRecord> = inner
            .records
            .values()
            .filter(|r| r.tenant_id == tenant_id)
            .filter(|r| match &source_table {
                Some(st) => r.source_table == *st,
                None => true,
            })
            .filter(|r| match source_id {
                Some(sid) => r.source_id == sid,
                None => true,
            })
            .cloned()
            .collect();

        items.sort_by_key(|a| a.id);
        let total = items.len() as u64;
        let start = (params.page.saturating_sub(1)) * params.per_page;
        let paginated: Vec<ArchiveRecord> = items
            .into_iter()
            .skip(start as usize)
            .take(params.per_page as usize)
            .collect();
        Ok(PaginatedResult::new(
            paginated,
            params.page,
            params.per_page,
            total,
        ))
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<ArchiveRecord, ApiError> {
        let mut inner = self.inner.lock();

        let record = inner
            .records
            .get_mut(&id)
            .filter(|r| r.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Archive record {} not found", id)))?;

        if record.restored_at.is_some() {
            return Err(ApiError::Conflict(format!(
                "Archive record {} is already restored",
                id
            )));
        }

        record.restored_at = Some(Utc::now());
        Ok(record.clone())
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();

        let len_before = inner.records.len();
        inner
            .records
            .retain(|_, r| !(r.id == id && r.tenant_id == tenant_id));

        if inner.records.len() == len_before {
            return Err(ApiError::NotFound(format!(
                "Archive record {} not found",
                id
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_archive_policy_crud() {
        let repo = InMemoryArchivePolicyRepository::new();

        let create = CreateArchivePolicy {
            name: "Old Invoices".to_string(),
            table_name: "invoices".to_string(),
            age_days: 365,
            conditions: None,
            is_active: true,
        };

        let policy = repo.create(create, 1).await.unwrap();
        assert_eq!(policy.id, 1);
        assert_eq!(policy.tenant_id, 1);
        assert_eq!(policy.table_name, "invoices");

        let found = repo.find_by_id(1, 1).await.unwrap().unwrap();
        assert_eq!(found.id, policy.id);

        let not_found = repo.find_by_id(1, 999).await.unwrap();
        assert!(not_found.is_none());

        let update = UpdateArchivePolicy {
            name: Some("Very Old Invoices".to_string()),
            table_name: None,
            age_days: Some(730),
            conditions: None,
            is_active: None,
        };
        let updated = repo.update(1, 1, update).await.unwrap();
        assert_eq!(updated.name, "Very Old Invoices");
        assert_eq!(updated.age_days, 730);

        repo.delete(1, 1).await.unwrap();
        let gone = repo.find_by_id(1, 1).await.unwrap();
        assert!(gone.is_none());
    }

    #[tokio::test]
    async fn test_archive_job_crud() {
        let repo = InMemoryArchiveJobRepository::new();

        let create = CreateArchiveJob { policy_id: 1 };
        let job = repo.create(create, 1).await.unwrap();
        assert_eq!(job.id, 1);
        assert_eq!(job.status, ArchiveJobStatus::Pending);

        let started = repo.start_job(1, 1).await.unwrap();
        assert_eq!(started.status, ArchiveJobStatus::Running);
        assert!(started.started_at.is_some());

        let completed = repo
            .update_status(1, 1, ArchiveJobStatus::Completed, 100, 0, None)
            .await
            .unwrap();
        assert_eq!(completed.status, ArchiveJobStatus::Completed);
        assert_eq!(completed.records_archived, 100);
        assert!(completed.completed_at.is_some());
    }

    #[tokio::test]
    async fn test_archive_record_crud() {
        let repo = InMemoryArchiveRecordRepository::new();

        let record = repo
            .create(
                1,
                "invoices".to_string(),
                42,
                serde_json::json!({"amount": 1000}),
                1,
            )
            .await
            .unwrap();
        assert_eq!(record.id, 1);
        assert_eq!(record.source_id, 42);

        let restored = repo.restore(1, 1).await.unwrap();
        assert!(restored.restored_at.is_some());

        let second_restore = repo.restore(1, 1).await;
        assert!(second_restore.is_err());

        repo.delete(1, 1).await.unwrap();
        let gone = repo.find_by_id(1, 1).await.unwrap();
        assert!(gone.is_none());
    }
}
