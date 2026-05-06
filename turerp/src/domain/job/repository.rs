//! Job repository trait and in-memory implementation

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};

use crate::domain::job::model::{
    CreateJob, CreateJobSchedule, Job, JobCounts, JobPriority, JobSchedule, JobStatus, JobType,
};
use crate::error::ApiError;

/// Job repository trait for persistent storage
#[async_trait::async_trait]
pub trait JobRepository: Send + Sync {
    /// Create a new job
    async fn create(&self, job: CreateJob) -> Result<Job, ApiError>;

    /// Find a job by ID
    async fn find_by_id(&self, id: i64) -> Result<Option<Job>, ApiError>;

    /// Find the next pending job (highest priority, oldest first)
    async fn find_next_pending(&self) -> Result<Option<Job>, ApiError>;

    /// Mark a job as running and increment attempts
    async fn mark_running(&self, id: i64) -> Result<(), ApiError>;

    /// Mark a job as completed
    async fn mark_completed(&self, id: i64) -> Result<(), ApiError>;

    /// Mark a job as failed (with retry logic)
    async fn mark_failed(&self, id: i64, error: &str) -> Result<(), ApiError>;

    /// Cancel a pending or scheduled job
    async fn cancel(&self, id: i64) -> Result<(), ApiError>;

    /// List jobs by status for a tenant
    async fn list_by_status(
        &self,
        tenant_id: i64,
        status: JobStatus,
    ) -> Result<Vec<Job>, ApiError>;

    /// Retry a failed job
    async fn retry(&self, id: i64) -> Result<(), ApiError>;

    /// Clean up old completed/failed/cancelled jobs
    async fn cleanup(&self, older_than: Duration) -> Result<u64, ApiError>;

    // Cron schedule methods

    /// Create a recurring job schedule
    async fn create_schedule(
        &self,
        schedule: CreateJobSchedule,
    ) -> Result<JobSchedule, ApiError>;

    /// List recurring schedules for a tenant
    async fn list_schedules(&self,
        tenant_id: i64,
    ) -> Result<Vec<JobSchedule>, ApiError>;

    /// Update next_run_at and last_run_at for a schedule
    async fn update_schedule_next_run(
        &self,
        id: i64,
        next_run: DateTime<Utc>,
        last_run: DateTime<Utc>,
    ) -> Result<(), ApiError>;

    /// Enable or disable a schedule
    async fn toggle_schedule(&self,
        id: i64,
        active: bool,
    ) -> Result<(), ApiError>;

    /// List schedules that are due to run
    async fn list_due_schedules(&self) -> Result<Vec<JobSchedule>, ApiError>;

    // Dashboard methods

    /// Count jobs by status for a tenant
    async fn count_by_status(
        &self,
        tenant_id: i64,
    ) -> Result<JobCounts, ApiError>;

    /// List recent jobs for a tenant
    async fn list_recent(
        &self,
        tenant_id: i64,
        limit: i64,
    ) -> Result<Vec<Job>, ApiError>;

    /// Reset stalled running jobs back to pending
    async fn reset_stalled(&self,
        timeout: Duration,
    ) -> Result<u64, ApiError>;
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
    async fn create(&self,
        create: CreateJob,
    ) -> Result<Job, ApiError> {
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
        };
        self.jobs.write().push(job.clone());
        Ok(job)
    }

    async fn find_by_id(&self,
        id: i64,
    ) -> Result<Option<Job>, ApiError> {
        Ok(self.jobs.read().iter().find(|j| j.id == id).cloned())
    }

    async fn find_next_pending(&self,
    ) -> Result<Option<Job>, ApiError> {
        let jobs = self.jobs.read();
        Ok(jobs
            .iter()
            .filter(|j| {
                j.status == JobStatus::Pending
                    && j.scheduled_at.map_or(true, |s| s <= Utc::now())
            })
            .max_by(|a, b| {
                let pa = Self::priority_value(a.priority);
                let pb = Self::priority_value(b.priority);
                pa.cmp(&pb).then_with(|| a.created_at.cmp(&b.created_at))
            })
            .cloned())
    }

    async fn mark_running(&self,
        id: i64,
    ) -> Result<(), ApiError> {
        let mut jobs = self.jobs.write();
        let job = jobs
            .iter_mut()
            .find(|j| j.id == id)
            .ok_or_else(|| ApiError::NotFound(format!("Job {} not found", id)))?;
        job.status = JobStatus::Running;
        job.started_at = Some(Utc::now());
        job.attempts += 1;
        job.updated_at = Some(Utc::now());
        Ok(())
    }

    async fn mark_completed(&self,
        id: i64,
    ) -> Result<(), ApiError> {
        let mut jobs = self.jobs.write();
        let job = jobs
            .iter_mut()
            .find(|j| j.id == id)
            .ok_or_else(|| ApiError::NotFound(format!("Job {} not found", id)))?;
        job.status = JobStatus::Completed;
        job.completed_at = Some(Utc::now());
        job.updated_at = Some(Utc::now());
        Ok(())
    }

    async fn mark_failed(&self,
        id: i64,
        error: &str,
    ) -> Result<(), ApiError> {
        let mut jobs = self.jobs.write();
        let job = jobs
            .iter_mut()
            .find(|j| j.id == id)
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

    async fn cancel(&self,
        id: i64,
    ) -> Result<(), ApiError> {
        let mut jobs = self.jobs.write();
        let job = jobs
            .iter_mut()
            .find(|j| j.id == id)
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
            .filter(|j| j.tenant_id == tenant_id && j.status == status)
            .cloned()
            .collect())
    }

    async fn retry(&self,
        id: i64,
    ) -> Result<(), ApiError> {
        let mut jobs = self.jobs.write();
        let job = jobs
            .iter_mut()
            .find(|j| j.id == id)
            .ok_or_else(|| ApiError::NotFound(format!("Job {} not found", id)))?;
        if job.status != JobStatus::Failed {
            return Err(ApiError::BadRequest("Can only retry failed jobs".to_string()));
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

    async fn cleanup(
        &self,
        older_than: Duration,
    ) -> Result<u64, ApiError> {
        let cutoff = Utc::now()
            - chrono::Duration::from_std(older_than)
                .unwrap_or(chrono::Duration::try_seconds(3600).unwrap_or(chrono::Duration::max_duration()));
        let mut jobs = self.jobs.write();
        let before = jobs.len();
        jobs.retain(|j| {
            !(j.status == JobStatus::Completed
                || j.status == JobStatus::Failed
                || j.status == JobStatus::Cancelled)
                || j.completed_at.map_or(true, |c| c > cutoff)
        });
        Ok((before - jobs.len()) as u64)
    }

    async fn create_schedule(
        &self,
        schedule: CreateJobSchedule,
    ) -> Result<JobSchedule, ApiError> {
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
        };
        self.schedules.write().push(s.clone());
        Ok(s)
    }

    async fn list_schedules(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<JobSchedule>, ApiError> {
        Ok(self
            .schedules
            .read()
            .iter()
            .filter(|s| s.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn update_schedule_next_run(
        &self,
        id: i64,
        next_run: DateTime<Utc>,
        last_run: DateTime<Utc>,
    ) -> Result<(), ApiError> {
        let mut schedules = self.schedules.write();
        let s = schedules
            .iter_mut()
            .find(|s| s.id == id)
            .ok_or_else(|| ApiError::NotFound(format!("Schedule {} not found", id)))?;
        s.next_run_at = Some(next_run);
        s.last_run_at = Some(last_run);
        s.updated_at = Some(Utc::now());
        Ok(())
    }

    async fn toggle_schedule(
        &self,
        id: i64,
        active: bool,
    ) -> Result<(), ApiError> {
        let mut schedules = self.schedules.write();
        let s = schedules
            .iter_mut()
            .find(|s| s.id == id)
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
            .filter(|s| {
                s.is_active
                    && s.next_run_at.map_or(true, |n| n <= now)
            })
            .cloned()
            .collect())
    }

    async fn count_by_status(
        &self,
        tenant_id: i64,
    ) -> Result<JobCounts, ApiError> {
        let jobs = self.jobs.read();
        let mut counts = JobCounts::default();
        for j in jobs.iter().filter(|j| j.tenant_id == tenant_id) {
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

    async fn list_recent(
        &self,
        tenant_id: i64,
        limit: i64,
    ) -> Result<Vec<Job>, ApiError> {
        let jobs = self.jobs.read();
        let mut filtered: Vec<Job> = jobs
            .iter()
            .filter(|j| j.tenant_id == tenant_id)
            .cloned()
            .collect();
        filtered.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        filtered.truncate(limit as usize);
        Ok(filtered)
    }

    async fn reset_stalled(
        &self,
        timeout: Duration,
    ) -> Result<u64, ApiError> {
        let cutoff = Utc::now()
            - chrono::Duration::from_std(timeout)
                .unwrap_or(chrono::Duration::try_seconds(300).unwrap_or(chrono::Duration::max_duration()));
        let mut jobs = self.jobs.write();
        let mut count = 0u64;
        for j in jobs.iter_mut().filter(|j| j.status == JobStatus::Running) {
            if j.started_at.map_or(false, |s| s < cutoff) {
                j.status = JobStatus::Pending;
                j.attempts += 1;
                j.started_at = None;
                j.updated_at = Some(Utc::now());
                count += 1;
            }
        }
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_repo_create_and_find() {
        let repo = InMemoryJobRepository::new();
        let job = repo
            .create(CreateJob::new(
                JobType::SendReminders { tenant_id: 1 },
                1,
            ))
            .await
            .unwrap();
        assert_eq!(job.status, JobStatus::Pending);

        let found = repo.find_by_id(job.id).await.unwrap();
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

        repo.mark_running(job.id).await.unwrap();
        repo.mark_failed(job.id, "db error").await.unwrap();

        let after = repo.find_by_id(job.id).await.unwrap().unwrap();
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

        repo.mark_running(job.id).await.unwrap();
        repo.mark_failed(job.id, "fatal").await.unwrap();

        let after = repo.find_by_id(job.id).await.unwrap().unwrap();
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
        repo.mark_running(job.id).await.unwrap();

        // Artificially backdate started_at
        {
            let mut jobs = repo.jobs.write();
            jobs[0].started_at = Some(Utc::now() - chrono::Duration::try_hours(1).unwrap());
        }

        let reset = repo.reset_stalled(Duration::from_secs(60)).await.unwrap();
        assert_eq!(reset, 1);

        let after = repo.find_by_id(job.id).await.unwrap().unwrap();
        assert_eq!(after.status, JobStatus::Pending);
        assert_eq!(after.attempts, 1);
    }

    #[tokio::test]
    async fn test_repo_schedule_crud() {
        let repo = InMemoryJobRepository::new();
        let s = repo
            .create_schedule(CreateJobSchedule {
                job_type: JobType::SendReminders { tenant_id: 1 },
                cron_expression: "0 0 * * *".to_string(),
                priority: JobPriority::Normal,
                tenant_id: 1,
                max_attempts: 3,
            })
            .await
            .unwrap();
        assert!(s.is_active);

        let list = repo.list_schedules(1).await.unwrap();
        assert_eq!(list.len(), 1);

        repo.toggle_schedule(s.id, false).await.unwrap();
        let after = repo.list_schedules(1).await.unwrap();
        assert!(!after[0].is_active);
    }
}
