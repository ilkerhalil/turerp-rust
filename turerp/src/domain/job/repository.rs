//! Job repository trait and in-memory implementation

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};

use crate::common::SoftDeletable;
#[cfg(test)]
use crate::domain::job::model::JobType;
use crate::domain::job::model::{
    CreateJob, CreateJobSchedule, Job, JobCounts, JobPriority, JobSchedule, JobStatus,
};
use crate::error::ApiError;

/// Job repository trait for persistent storage
///
/// All admin-API-exposed methods take `tenant_id` to prevent cross-tenant
/// access. Background-worker methods (`find_next_pending`, `list_due_schedules`,
/// `reset_stalled`, `cleanup`) intentionally stay tenant-agnostic because they
/// are invoked by system processes, not by tenant users.
#[async_trait::async_trait]
pub trait JobRepository: Send + Sync {
    /// Create a new job
    async fn create(&self, job: CreateJob) -> Result<Job, ApiError>;

    /// Find a job by ID, scoped to the given tenant
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Job>, ApiError>;

    /// Find the next pending job (highest priority, oldest first)
    ///
    /// System-level: invoked by background worker, not by tenant users.
    async fn find_next_pending(&self) -> Result<Option<Job>, ApiError>;

    /// Find the next pending job, scoped to the given tenant.
    ///
    /// Tenant-scoped variant of [`JobRepository::find_next_pending`] used by
    /// the admin API so a tenant admin cannot dequeue (and the Postgres impl
    /// cannot `FOR UPDATE SKIP LOCKED`-claim) another tenant's pending job.
    async fn find_next_pending_for_tenant(&self, tenant_id: i64) -> Result<Option<Job>, ApiError>;

    /// Mark a job as running and increment attempts, scoped to tenant
    async fn mark_running(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;

    /// Mark a job as completed, scoped to tenant
    async fn mark_completed(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;

    /// Mark a job as failed (with retry logic), scoped to tenant
    async fn mark_failed(&self, id: i64, tenant_id: i64, error: &str) -> Result<(), ApiError>;

    /// Cancel a pending or scheduled job, scoped to tenant
    async fn cancel(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;

    /// List jobs by status for a tenant
    async fn list_by_status(&self, tenant_id: i64, status: JobStatus)
        -> Result<Vec<Job>, ApiError>;

    /// Retry a failed job, scoped to tenant
    async fn retry(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;

    /// Clean up old completed/failed/cancelled jobs
    ///
    /// System-level: invoked by background task, not by tenant users.
    async fn cleanup(&self, older_than: Duration) -> Result<u64, ApiError>;

    /// Clean up old completed/failed/cancelled jobs, scoped to the given
    /// tenant.
    ///
    /// Tenant-scoped variant of [`JobRepository::cleanup`] used by the admin
    /// API so a tenant admin can only purge its own terminal jobs.
    async fn cleanup_for_tenant(
        &self,
        tenant_id: i64,
        older_than: Duration,
    ) -> Result<u64, ApiError>;

    /// Soft delete a job, scoped to tenant
    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError>;

    /// Restore a soft-deleted job, scoped to tenant
    async fn restore(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;

    /// List deleted jobs for a tenant
    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Job>, ApiError>;

    /// Permanently destroy a soft-deleted job, scoped to tenant
    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;

    /// Soft delete a job schedule, scoped to tenant
    async fn soft_delete_schedule(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError>;

    /// Restore a soft-deleted job schedule, scoped to tenant
    async fn restore_schedule(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;

    /// List deleted schedules for a tenant
    async fn find_deleted_schedules(&self, tenant_id: i64) -> Result<Vec<JobSchedule>, ApiError>;

    /// Permanently destroy a soft-deleted schedule, scoped to tenant
    async fn destroy_schedule(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;

    // Cron schedule methods

    /// Create a recurring job schedule
    async fn create_schedule(&self, schedule: CreateJobSchedule) -> Result<JobSchedule, ApiError>;

    /// List recurring schedules for a tenant
    async fn list_schedules(&self, tenant_id: i64) -> Result<Vec<JobSchedule>, ApiError>;

    /// Update next_run_at and last_run_at for a schedule, scoped to tenant
    async fn update_schedule_next_run(
        &self,
        id: i64,
        tenant_id: i64,
        next_run: DateTime<Utc>,
        last_run: DateTime<Utc>,
    ) -> Result<(), ApiError>;

    /// Enable or disable a schedule, scoped to tenant
    async fn toggle_schedule(&self, id: i64, tenant_id: i64, active: bool) -> Result<(), ApiError>;

    /// List schedules that are due to run
    ///
    /// System-level: invoked by background cron task, not by tenant users.
    async fn list_due_schedules(&self) -> Result<Vec<JobSchedule>, ApiError>;

    // Dashboard methods

    /// Count jobs by status for a tenant
    async fn count_by_status(&self, tenant_id: i64) -> Result<JobCounts, ApiError>;

    /// List recent jobs for a tenant
    async fn list_recent(&self, tenant_id: i64, limit: i64) -> Result<Vec<Job>, ApiError>;

    /// Reset stalled running jobs back to pending
    ///
    /// System-level: invoked by background heartbeat task, not by tenant users.
    async fn reset_stalled(&self, timeout: Duration) -> Result<u64, ApiError>;
}

/// Type alias for boxed job repository
pub type BoxJobRepository = Arc<dyn JobRepository>;

/// In-memory job repository for development/testing
pub struct InMemoryJobRepository {
    jobs: parking_lot::RwLock<Vec<Job>>,
    schedules: parking_lot::RwLock<Vec<JobSchedule>>,
    next_id: parking_lot::RwLock<i64>,
    next_schedule_id: parking_lot::RwLock<i64>,
}

impl InMemoryJobRepository {
    /// Create a new empty repository
    pub fn new() -> Self {
        Self {
            jobs: parking_lot::RwLock::new(Vec::new()),
            schedules: parking_lot::RwLock::new(Vec::new()),
            next_id: parking_lot::RwLock::new(1),
            next_schedule_id: parking_lot::RwLock::new(1),
        }
    }

    fn allocate_id(&self) -> i64 {
        let mut id = self.next_id.write();
        let job_id = *id;
        *id += 1;
        job_id
    }

    fn allocate_schedule_id(&self) -> i64 {
        let mut id = self.next_schedule_id.write();
        let sid = *id;
        *id += 1;
        sid
    }

    fn priority_value(p: JobPriority) -> u8 {
        match p {
            JobPriority::Critical => 4,
            JobPriority::High => 3,
            JobPriority::Normal => 2,
            JobPriority::Low => 1,
        }
    }
}

impl Default for InMemoryJobRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl JobRepository for InMemoryJobRepository {
    async fn create(&self, create: CreateJob) -> Result<Job, ApiError> {
        let id = self.allocate_id();
        let job = Job {
            id,
            job_type: create.job_type,
            status: if create.scheduled_at.is_some() {
                JobStatus::Scheduled
            } else {
                JobStatus::Pending
            },
            priority: create.priority,
            tenant_id: create.tenant_id,
            attempts: 0,
            max_attempts: create.max_attempts,
            scheduled_at: create.scheduled_at,
            started_at: None,
            completed_at: None,
            last_error: None,
            created_at: Utc::now(),
            updated_at: None,
            deleted_at: None,
            deleted_by: None,
        };
        self.jobs.write().push(job.clone());
        Ok(job)
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Job>, ApiError> {
        Ok(self
            .jobs
            .read()
            .iter()
            .find(|j| j.id == id && j.tenant_id == tenant_id && !j.is_deleted())
            .cloned())
    }

    async fn find_next_pending(&self) -> Result<Option<Job>, ApiError> {
        let jobs = self.jobs.read();
        Ok(jobs
            .iter()
            .filter(|j| {
                !j.is_deleted()
                    && j.status == JobStatus::Pending
                    && j.scheduled_at.is_none_or(|s| s <= Utc::now())
            })
            .max_by(|a, b| {
                let pa = Self::priority_value(a.priority);
                let pb = Self::priority_value(b.priority);
                pa.cmp(&pb).then_with(|| a.created_at.cmp(&b.created_at))
            })
            .cloned())
    }

    async fn find_next_pending_for_tenant(&self, tenant_id: i64) -> Result<Option<Job>, ApiError> {
        let jobs = self.jobs.read();
        Ok(jobs
            .iter()
            .filter(|j| {
                j.tenant_id == tenant_id
                    && !j.is_deleted()
                    && j.status == JobStatus::Pending
                    && j.scheduled_at.is_none_or(|s| s <= Utc::now())
            })
            .max_by(|a, b| {
                let pa = Self::priority_value(a.priority);
                let pb = Self::priority_value(b.priority);
                pa.cmp(&pb).then_with(|| a.created_at.cmp(&b.created_at))
            })
            .cloned())
    }

    async fn mark_running(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut jobs = self.jobs.write();
        let job = jobs
            .iter_mut()
            .find(|j| j.id == id && j.tenant_id == tenant_id && !j.is_deleted())
            .ok_or_else(|| ApiError::NotFound(format!("Job {} not found", id)))?;
        job.status = JobStatus::Running;
        job.started_at = Some(Utc::now());
        job.attempts += 1;
        job.updated_at = Some(Utc::now());
        Ok(())
    }

    async fn mark_completed(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut jobs = self.jobs.write();
        let job = jobs
            .iter_mut()
            .find(|j| j.id == id && j.tenant_id == tenant_id && !j.is_deleted())
            .ok_or_else(|| ApiError::NotFound(format!("Job {} not found", id)))?;
        job.status = JobStatus::Completed;
        job.completed_at = Some(Utc::now());
        job.updated_at = Some(Utc::now());
        Ok(())
    }

    async fn mark_failed(&self, id: i64, tenant_id: i64, error: &str) -> Result<(), ApiError> {
        let mut jobs = self.jobs.write();
        let job = jobs
            .iter_mut()
            .find(|j| j.id == id && j.tenant_id == tenant_id && !j.is_deleted())
            .ok_or_else(|| ApiError::NotFound(format!("Job {} not found", id)))?;
        job.last_error = Some(error.to_string());
        if job.attempts >= job.max_attempts {
            job.status = JobStatus::Failed;
            job.completed_at = Some(Utc::now());
        } else {
            job.status = JobStatus::Pending;
            let backoff_secs = (2_i64.pow(job.attempts)).min(3600);
            job.scheduled_at = Some(Utc::now() + chrono::Duration::seconds(backoff_secs));
        }
        job.updated_at = Some(Utc::now());
        Ok(())
    }

    async fn cancel(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut jobs = self.jobs.write();
        let job = jobs
            .iter_mut()
            .find(|j| j.id == id && j.tenant_id == tenant_id && !j.is_deleted())
            .ok_or_else(|| ApiError::NotFound(format!("Job {} not found", id)))?;
        if job.status != JobStatus::Pending && job.status != JobStatus::Scheduled {
            return Err(ApiError::BadRequest(
                "Can only cancel pending or scheduled jobs".to_string(),
            ));
        }
        job.status = JobStatus::Cancelled;
        job.completed_at = Some(Utc::now());
        job.updated_at = Some(Utc::now());
        Ok(())
    }

    async fn list_by_status(
        &self,
        tenant_id: i64,
        status: JobStatus,
    ) -> Result<Vec<Job>, ApiError> {
        Ok(self
            .jobs
            .read()
            .iter()
            .filter(|j| j.tenant_id == tenant_id && j.status == status && !j.is_deleted())
            .cloned()
            .collect())
    }

    async fn retry(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut jobs = self.jobs.write();
        let job = jobs
            .iter_mut()
            .find(|j| j.id == id && j.tenant_id == tenant_id && !j.is_deleted())
            .ok_or_else(|| ApiError::NotFound(format!("Job {} not found", id)))?;
        if job.status != JobStatus::Failed {
            return Err(ApiError::BadRequest(
                "Can only retry failed jobs".to_string(),
            ));
        }
        job.status = JobStatus::Pending;
        job.attempts = 0;
        job.last_error = None;
        job.scheduled_at = None;
        job.started_at = None;
        job.completed_at = None;
        job.updated_at = Some(Utc::now());
        Ok(())
    }

    async fn cleanup(&self, older_than: Duration) -> Result<u64, ApiError> {
        let cutoff = Utc::now()
            - chrono::Duration::from_std(older_than)
                .unwrap_or(chrono::Duration::try_seconds(3600).unwrap_or(chrono::Duration::zero()));
        let mut jobs = self.jobs.write();
        let before = jobs.len();
        jobs.retain(|j| {
            j.is_deleted()
                || !(j.status == JobStatus::Completed
                    || j.status == JobStatus::Failed
                    || j.status == JobStatus::Cancelled)
                || j.completed_at.is_none_or(|c| c > cutoff)
        });
        Ok((before - jobs.len()) as u64)
    }

    async fn cleanup_for_tenant(
        &self,
        tenant_id: i64,
        older_than: Duration,
    ) -> Result<u64, ApiError> {
        let cutoff = Utc::now()
            - chrono::Duration::from_std(older_than)
                .unwrap_or(chrono::Duration::try_seconds(3600).unwrap_or(chrono::Duration::zero()));
        let mut jobs = self.jobs.write();
        let before = jobs.len();
        // Keep every job that is NOT a terminal job of this tenant completed
        // at/before the cutoff (deleted jobs are preserved, mirroring `cleanup`).
        jobs.retain(|j| {
            j.tenant_id != tenant_id
                || j.is_deleted()
                || !(j.status == JobStatus::Completed
                    || j.status == JobStatus::Failed
                    || j.status == JobStatus::Cancelled)
                || j.completed_at.is_none_or(|c| c > cutoff)
        });
        Ok((before - jobs.len()) as u64)
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let mut jobs = self.jobs.write();
        let job = jobs
            .iter_mut()
            .find(|j| j.id == id && j.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Job {} not found", id)))?;
        if job.is_deleted() {
            return Err(ApiError::Conflict(format!("Job {} is already deleted", id)));
        }
        job.mark_deleted(deleted_by);
        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut jobs = self.jobs.write();
        let job = jobs
            .iter_mut()
            .find(|j| j.id == id && j.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Job {} not found", id)))?;
        if !job.is_deleted() {
            return Err(ApiError::BadRequest(format!("Job {} is not deleted", id)));
        }
        job.restore();
        Ok(())
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Job>, ApiError> {
        Ok(self
            .jobs
            .read()
            .iter()
            .filter(|j| j.tenant_id == tenant_id && j.is_deleted())
            .cloned()
            .collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut jobs = self.jobs.write();
        let len_before = jobs.len();
        jobs.retain(|j| !(j.id == id && j.tenant_id == tenant_id && j.is_deleted()));
        if jobs.len() == len_before {
            return Err(ApiError::NotFound(format!("Deleted job {} not found", id)));
        }
        Ok(())
    }

    async fn create_schedule(&self, schedule: CreateJobSchedule) -> Result<JobSchedule, ApiError> {
        let id = self.allocate_schedule_id();
        let s = JobSchedule {
            id,
            job_type: schedule.job_type,
            cron_expression: schedule.cron_expression,
            priority: schedule.priority,
            tenant_id: schedule.tenant_id,
            max_attempts: schedule.max_attempts,
            is_active: true,
            next_run_at: None,
            last_run_at: None,
            created_at: Utc::now(),
            updated_at: None,
            deleted_at: None,
            deleted_by: None,
        };
        self.schedules.write().push(s.clone());
        Ok(s)
    }

    async fn list_schedules(&self, tenant_id: i64) -> Result<Vec<JobSchedule>, ApiError> {
        Ok(self
            .schedules
            .read()
            .iter()
            .filter(|s| s.tenant_id == tenant_id && !s.is_deleted())
            .cloned()
            .collect())
    }

    async fn update_schedule_next_run(
        &self,
        id: i64,
        tenant_id: i64,
        next_run: DateTime<Utc>,
        last_run: DateTime<Utc>,
    ) -> Result<(), ApiError> {
        let mut schedules = self.schedules.write();
        let s = schedules
            .iter_mut()
            .find(|s| s.id == id && s.tenant_id == tenant_id && !s.is_deleted())
            .ok_or_else(|| ApiError::NotFound(format!("Schedule {} not found", id)))?;
        s.next_run_at = Some(next_run);
        s.last_run_at = Some(last_run);
        s.updated_at = Some(Utc::now());
        Ok(())
    }

    async fn toggle_schedule(&self, id: i64, tenant_id: i64, active: bool) -> Result<(), ApiError> {
        let mut schedules = self.schedules.write();
        let s = schedules
            .iter_mut()
            .find(|s| s.id == id && s.tenant_id == tenant_id && !s.is_deleted())
            .ok_or_else(|| ApiError::NotFound(format!("Schedule {} not found", id)))?;
        s.is_active = active;
        s.updated_at = Some(Utc::now());
        Ok(())
    }

    async fn list_due_schedules(&self) -> Result<Vec<JobSchedule>, ApiError> {
        let now = Utc::now();
        Ok(self
            .schedules
            .read()
            .iter()
            .filter(|s| !s.is_deleted() && s.is_active && s.next_run_at.is_none_or(|n| n <= now))
            .cloned()
            .collect())
    }

    async fn count_by_status(&self, tenant_id: i64) -> Result<JobCounts, ApiError> {
        let jobs = self.jobs.read();
        let mut counts = JobCounts::default();
        for j in jobs
            .iter()
            .filter(|j| j.tenant_id == tenant_id && !j.is_deleted())
        {
            match j.status {
                JobStatus::Pending => counts.pending += 1,
                JobStatus::Running => counts.running += 1,
                JobStatus::Completed => counts.completed += 1,
                JobStatus::Failed => counts.failed += 1,
                JobStatus::Cancelled => counts.cancelled += 1,
                JobStatus::Scheduled => counts.scheduled += 1,
            }
        }
        Ok(counts)
    }

    async fn list_recent(&self, tenant_id: i64, limit: i64) -> Result<Vec<Job>, ApiError> {
        let jobs = self.jobs.read();
        let mut filtered: Vec<Job> = jobs
            .iter()
            .filter(|j| j.tenant_id == tenant_id && !j.is_deleted())
            .cloned()
            .collect();
        filtered.sort_by_key(|b| std::cmp::Reverse(b.created_at));
        filtered.truncate(limit as usize);
        Ok(filtered)
    }

    async fn reset_stalled(&self, timeout: Duration) -> Result<u64, ApiError> {
        let cutoff = Utc::now()
            - chrono::Duration::from_std(timeout)
                .unwrap_or(chrono::Duration::try_seconds(300).unwrap_or(chrono::Duration::zero()));
        let mut jobs = self.jobs.write();
        let mut count = 0u64;
        for j in jobs
            .iter_mut()
            .filter(|j| j.status == JobStatus::Running && !j.is_deleted())
        {
            if j.started_at.is_some_and(|s| s < cutoff) {
                j.status = JobStatus::Pending;
                j.attempts += 1;
                j.started_at = None;
                j.updated_at = Some(Utc::now());
                count += 1;
            }
        }
        Ok(count)
    }

    async fn soft_delete_schedule(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        let mut schedules = self.schedules.write();
        let s = schedules
            .iter_mut()
            .find(|s| s.id == id && s.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Schedule {} not found", id)))?;
        if s.is_deleted() {
            return Err(ApiError::Conflict(format!(
                "Schedule {} is already deleted",
                id
            )));
        }
        s.mark_deleted(deleted_by);
        Ok(())
    }

    async fn restore_schedule(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut schedules = self.schedules.write();
        let s = schedules
            .iter_mut()
            .find(|s| s.id == id && s.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Schedule {} not found", id)))?;
        if !s.is_deleted() {
            return Err(ApiError::BadRequest(format!(
                "Schedule {} is not deleted",
                id
            )));
        }
        s.restore();
        Ok(())
    }

    async fn find_deleted_schedules(&self, tenant_id: i64) -> Result<Vec<JobSchedule>, ApiError> {
        Ok(self
            .schedules
            .read()
            .iter()
            .filter(|s| s.tenant_id == tenant_id && s.is_deleted())
            .cloned()
            .collect())
    }

    async fn destroy_schedule(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut schedules = self.schedules.write();
        let len_before = schedules.len();
        schedules.retain(|s| !(s.id == id && s.tenant_id == tenant_id && s.is_deleted()));
        if schedules.len() == len_before {
            return Err(ApiError::NotFound(format!(
                "Deleted schedule {} not found",
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
    async fn test_repo_create_and_find() {
        let repo = InMemoryJobRepository::new();
        let job = repo
            .create(CreateJob::new(JobType::SendReminders { tenant_id: 1 }, 1))
            .await
            .unwrap();
        assert_eq!(job.status, JobStatus::Pending);

        let found = repo.find_by_id(job.id, 1).await.unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn test_repo_next_pending_priority() {
        let repo = InMemoryJobRepository::new();
        repo.create(
            CreateJob::new(JobType::SendReminders { tenant_id: 1 }, 1)
                .with_priority(JobPriority::Low),
        )
        .await
        .unwrap();
        repo.create(
            CreateJob::new(JobType::SendReminders { tenant_id: 1 }, 1)
                .with_priority(JobPriority::Critical),
        )
        .await
        .unwrap();

        let next = repo.find_next_pending().await.unwrap().unwrap();
        assert_eq!(next.priority, JobPriority::Critical);
    }

    #[tokio::test]
    async fn test_repo_mark_failed_retry() {
        let repo = InMemoryJobRepository::new();
        let job = repo
            .create(
                CreateJob::new(
                    JobType::CalculateDepreciation {
                        asset_id: 1,
                        tenant_id: 1,
                    },
                    1,
                )
                .with_max_attempts(3),
            )
            .await
            .unwrap();

        repo.mark_running(job.id, 1).await.unwrap();
        repo.mark_failed(job.id, 1, "db error").await.unwrap();

        let after = repo.find_by_id(job.id, 1).await.unwrap().unwrap();
        assert_eq!(after.status, JobStatus::Pending);
        assert_eq!(after.attempts, 1);
        assert!(after.scheduled_at.is_some());
    }

    #[tokio::test]
    async fn test_repo_mark_failed_max_retries() {
        let repo = InMemoryJobRepository::new();
        let job = repo
            .create(
                CreateJob::new(
                    JobType::RunPayroll {
                        tenant_id: 1,
                        period: "2024-01".to_string(),
                    },
                    1,
                )
                .with_max_attempts(1),
            )
            .await
            .unwrap();

        repo.mark_running(job.id, 1).await.unwrap();
        repo.mark_failed(job.id, 1, "fatal").await.unwrap();

        let after = repo.find_by_id(job.id, 1).await.unwrap().unwrap();
        assert_eq!(after.status, JobStatus::Failed);
    }

    #[tokio::test]
    async fn test_repo_counts() {
        let repo = InMemoryJobRepository::new();
        for _ in 0..3 {
            repo.create(CreateJob::new(JobType::SendReminders { tenant_id: 1 }, 1))
                .await
                .unwrap();
        }
        let counts = repo.count_by_status(1).await.unwrap();
        assert_eq!(counts.pending, 3);
    }

    #[tokio::test]
    async fn test_repo_reset_stalled() {
        let repo = InMemoryJobRepository::new();
        let job = repo
            .create(CreateJob::new(JobType::SendReminders { tenant_id: 1 }, 1))
            .await
            .unwrap();
        repo.mark_running(job.id, 1).await.unwrap();

        // Artificially backdate started_at
        {
            let mut jobs = repo.jobs.write();
            jobs[0].started_at = Some(Utc::now() - chrono::Duration::try_hours(1).unwrap());
        }

        let reset = repo.reset_stalled(Duration::from_secs(60)).await.unwrap();
        assert_eq!(reset, 1);

        let after = repo.find_by_id(job.id, 1).await.unwrap().unwrap();
        assert_eq!(after.status, JobStatus::Pending);
        assert_eq!(after.attempts, 2);
    }

    #[tokio::test]
    async fn test_repo_schedule_crud() {
        let repo = InMemoryJobRepository::new();
        let s = repo
            .create_schedule(CreateJobSchedule {
                job_type: JobType::SendReminders { tenant_id: 1 },
                cron_expression: "0 0 0 * * *".to_string(),
                priority: JobPriority::Normal,
                tenant_id: 1,
                max_attempts: 3,
            })
            .await
            .unwrap();
        assert!(s.is_active);

        let list = repo.list_schedules(1).await.unwrap();
        assert_eq!(list.len(), 1);

        repo.toggle_schedule(s.id, 1, false).await.unwrap();
        let after = repo.list_schedules(1).await.unwrap();
        assert!(!after[0].is_active);
    }

    // ---- Tenant isolation tests (security) ----

    #[tokio::test]
    async fn test_repo_find_by_id_blocks_other_tenant() {
        let repo = InMemoryJobRepository::new();
        let job = repo
            .create(CreateJob::new(JobType::SendReminders { tenant_id: 1 }, 1))
            .await
            .unwrap();
        // Tenant 2 must NOT be able to read tenant 1's job
        let result = repo.find_by_id(job.id, 2).await.unwrap();
        assert!(result.is_none(), "tenant 2 should not see tenant 1's job");
        // Tenant 1 can read it
        let result = repo.find_by_id(job.id, 1).await.unwrap();
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_repo_mark_running_blocks_other_tenant() {
        let repo = InMemoryJobRepository::new();
        let job = repo
            .create(CreateJob::new(JobType::SendReminders { tenant_id: 1 }, 1))
            .await
            .unwrap();
        let result = repo.mark_running(job.id, 2).await;
        assert!(
            result.is_err(),
            "tenant 2 must not mark tenant 1's job as running"
        );
        // Tenant 1 succeeds
        assert!(repo.mark_running(job.id, 1).await.is_ok());
    }

    #[tokio::test]
    async fn test_repo_cancel_blocks_other_tenant() {
        let repo = InMemoryJobRepository::new();
        let job = repo
            .create(CreateJob::new(JobType::SendReminders { tenant_id: 1 }, 1))
            .await
            .unwrap();
        let result = repo.cancel(job.id, 2).await;
        assert!(result.is_err(), "tenant 2 must not cancel tenant 1's job");
    }

    #[tokio::test]
    async fn test_repo_retry_blocks_other_tenant() {
        let repo = InMemoryJobRepository::new();
        let job = repo
            .create(CreateJob::new(JobType::SendReminders { tenant_id: 1 }, 1).with_max_attempts(1))
            .await
            .unwrap();
        repo.mark_running(job.id, 1).await.unwrap();
        repo.mark_failed(job.id, 1, "err").await.unwrap();
        let result = repo.retry(job.id, 2).await;
        assert!(result.is_err(), "tenant 2 must not retry tenant 1's job");
    }

    #[tokio::test]
    async fn test_repo_soft_delete_blocks_other_tenant() {
        let repo = InMemoryJobRepository::new();
        let job = repo
            .create(CreateJob::new(JobType::SendReminders { tenant_id: 1 }, 1))
            .await
            .unwrap();
        let result = repo.soft_delete(job.id, 2, 99).await;
        assert!(
            result.is_err(),
            "tenant 2 must not soft-delete tenant 1's job"
        );
    }

    #[tokio::test]
    async fn test_repo_restore_blocks_other_tenant() {
        let repo = InMemoryJobRepository::new();
        let job = repo
            .create(CreateJob::new(JobType::SendReminders { tenant_id: 1 }, 1))
            .await
            .unwrap();
        repo.soft_delete(job.id, 1, 1).await.unwrap();
        let result = repo.restore(job.id, 2).await;
        assert!(result.is_err(), "tenant 2 must not restore tenant 1's job");
    }

    #[tokio::test]
    async fn test_repo_destroy_blocks_other_tenant() {
        let repo = InMemoryJobRepository::new();
        let job = repo
            .create(CreateJob::new(JobType::SendReminders { tenant_id: 1 }, 1))
            .await
            .unwrap();
        repo.soft_delete(job.id, 1, 1).await.unwrap();
        let result = repo.destroy(job.id, 2).await;
        assert!(result.is_err(), "tenant 2 must not destroy tenant 1's job");
    }

    #[tokio::test]
    async fn test_repo_toggle_schedule_blocks_other_tenant() {
        let repo = InMemoryJobRepository::new();
        let s = repo
            .create_schedule(CreateJobSchedule {
                job_type: JobType::SendReminders { tenant_id: 1 },
                cron_expression: "0 0 0 * * *".to_string(),
                priority: JobPriority::Normal,
                tenant_id: 1,
                max_attempts: 3,
            })
            .await
            .unwrap();
        let result = repo.toggle_schedule(s.id, 2, false).await;
        assert!(
            result.is_err(),
            "tenant 2 must not toggle tenant 1's schedule"
        );
    }

    #[tokio::test]
    async fn test_repo_update_schedule_next_run_blocks_other_tenant() {
        let repo = InMemoryJobRepository::new();
        let s = repo
            .create_schedule(CreateJobSchedule {
                job_type: JobType::SendReminders { tenant_id: 1 },
                cron_expression: "0 0 0 * * *".to_string(),
                priority: JobPriority::Normal,
                tenant_id: 1,
                max_attempts: 3,
            })
            .await
            .unwrap();
        let now = Utc::now();
        let result = repo.update_schedule_next_run(s.id, 2, now, now).await;
        assert!(
            result.is_err(),
            "tenant 2 must not update tenant 1's schedule"
        );
    }

    // --- mark_completed / mark_failed tenant isolation ---

    #[tokio::test]
    async fn test_repo_mark_completed_blocks_other_tenant() {
        let repo = InMemoryJobRepository::new();
        let job = repo
            .create(CreateJob::new(JobType::SendReminders { tenant_id: 1 }, 1))
            .await
            .unwrap();
        repo.mark_running(job.id, 1).await.unwrap();

        // tenant 2 cannot mark tenant 1's job completed
        let result = repo.mark_completed(job.id, 2).await;
        assert!(
            result.is_err(),
            "tenant 2 must not mark tenant 1's job completed"
        );

        // tenant 1 can still mark it completed
        let result = repo.mark_completed(job.id, 1).await;
        assert!(result.is_ok(), "tenant 1 must be able to complete own job");
    }

    #[tokio::test]
    async fn test_repo_mark_failed_blocks_other_tenant() {
        let repo = InMemoryJobRepository::new();
        let job = repo
            .create(CreateJob::new(JobType::SendReminders { tenant_id: 1 }, 1))
            .await
            .unwrap();
        repo.mark_running(job.id, 1).await.unwrap();

        // tenant 2 cannot mark tenant 1's job failed
        let result = repo.mark_failed(job.id, 2, "injected error").await;
        assert!(
            result.is_err(),
            "tenant 2 must not mark tenant 1's job failed"
        );

        // tenant 1 can still mark it failed
        let result = repo.mark_failed(job.id, 1, "legit error").await;
        assert!(result.is_ok(), "tenant 1 must be able to fail own job");
    }

    // --- tenant-scoped admin API variants (cross-tenant leak fix) ---

    #[tokio::test]
    async fn test_repo_find_next_pending_for_tenant_blocks_other_tenant() {
        let repo = InMemoryJobRepository::new();
        // Tenant 1 owns the only pending job.
        repo.create(CreateJob::new(JobType::SendReminders { tenant_id: 1 }, 1))
            .await
            .unwrap();

        // Tenant 2 must NOT be able to dequeue tenant 1's pending job.
        let next = repo.find_next_pending_for_tenant(2).await.unwrap();
        assert!(
            next.is_none(),
            "tenant 2 must not see tenant 1's pending job"
        );

        // Tenant 1 can dequeue its own job.
        let next = repo.find_next_pending_for_tenant(1).await.unwrap();
        assert!(next.is_some(), "tenant 1 must see its own pending job");
        assert_eq!(next.unwrap().tenant_id, 1);
    }

    #[tokio::test]
    async fn test_repo_cleanup_for_tenant_blocks_other_tenant() {
        let repo = InMemoryJobRepository::new();
        // Terminal jobs for both tenants, completed in the past.
        for tenant in [1, 2] {
            let job = repo
                .create(
                    CreateJob::new(JobType::SendReminders { tenant_id: tenant }, tenant)
                        .with_max_attempts(1),
                )
                .await
                .unwrap();
            repo.mark_running(job.id, tenant).await.unwrap();
            repo.mark_completed(job.id, tenant).await.unwrap();
            // Backdate completion so it falls under any reasonable cutoff.
            {
                let mut jobs = repo.jobs.write();
                for j in jobs.iter_mut() {
                    if j.id == job.id {
                        j.completed_at = Some(Utc::now() - chrono::Duration::try_days(30).unwrap());
                    }
                }
            }
        }

        // Tenant 1 cleanup must NOT touch tenant 2's jobs.
        let removed = repo
            .cleanup_for_tenant(1, Duration::from_secs(60))
            .await
            .unwrap();
        assert_eq!(
            removed, 1,
            "tenant 1 cleanup should remove only its own job"
        );

        // Tenant 2's job must still exist.
        let guard = repo.jobs.read();
        let remaining: Vec<_> = guard.iter().filter(|j| j.tenant_id == 2).cloned().collect();
        drop(guard);
        assert_eq!(
            remaining.len(),
            1,
            "tenant 2's job must survive tenant 1 cleanup"
        );

        // Tenant 2 can now clean its own.
        let removed = repo
            .cleanup_for_tenant(2, Duration::from_secs(60))
            .await
            .unwrap();
        assert_eq!(removed, 1, "tenant 2 cleanup should remove its own job");
    }
}
