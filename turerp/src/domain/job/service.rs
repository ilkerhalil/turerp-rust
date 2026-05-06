//! Job service wrapping a repository and running background tasks

use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use cron::Schedule;

use crate::common::jobs::{CreateJob as CommonCreateJob, Job as CommonJob, JobScheduler};
use crate::domain::job::model::{
    CreateJob, CreateJobSchedule, Job, JobCounts, JobPriority, JobSchedule, JobStatus, JobType,
};
use crate::domain::job::repository::JobRepository;
use crate::error::ApiError;

/// Job service that implements `JobScheduler` and manages background tasks
pub struct JobService {
    repo: Arc<dyn JobRepository>,
    shutdown: parking_lot::Mutex<Option<tokio::sync::mpsc::Sender<()>>>,
}

impl JobService {
    /// Create a new job service from a repository
    pub fn new(repo: Arc<dyn JobRepository>) -> Self {
        Self {
            repo,
            shutdown: parking_lot::Mutex::new(None),
        }
    }

    /// Start background tasks: cron evaluation and stalled job recovery
    pub async fn start_background_tasks(&self,
    ) {
        let repo = self.repo.clone();
        let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(1);
        *self.shutdown.lock() = Some(tx);

        tokio::spawn(async move {
            let mut cron_interval = tokio::time::interval(Duration::from_secs(60));
            let mut heartbeat_interval = tokio::time::interval(Duration::from_secs(300));
            // First tick fires immediately; skip it for cron to avoid startup storm
            cron_interval.tick().await;

            loop {
                tokio::select! {
                    _ = cron_interval.tick() => {
                        if let Err(e) = Self::evaluate_schedules(&*repo).await {
                            tracing::warn!("Cron schedule evaluation failed: {}", e);
                        }
                    }
                    _ = heartbeat_interval.tick() => {
                        if let Err(e) = repo.reset_stalled(Duration::from_secs(1800)).await {
                            tracing::warn!("Stalled job recovery failed: {}", e);
                        }
                    }
                    _ = rx.recv() => {
                        tracing::info!("Job service background tasks shutting down");
                        break;
                    }
                }
            }
        });
    }

    /// Shut down background tasks
    pub async fn shutdown(&self,
    ) {
        if let Some(tx) = self.shutdown.lock().take() {
            let _ = tx.send(()).await;
        }
    }

    /// Evaluate due cron schedules and enqueue jobs
    async fn evaluate_schedules(
        repo: &dyn JobRepository,
    ) -> Result<(), ApiError> {
        let schedules = repo.list_due_schedules().await?;
        let now = Utc::now();

        for schedule in schedules {
            let cron_expr = &schedule.cron_expression;
            let schedule_parsed = match Schedule::from_str(cron_expr) {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!(
                        "Invalid cron expression '{}': {}. Disabling schedule {}.",
                        cron_expr,
                        e,
                        schedule.id
                    );
                    repo.toggle_schedule(schedule.id, false).await.ok();
                    continue;
                }
            };

            // Create job from schedule
            let job = CreateJob::new(schedule.job_type.clone(), schedule.tenant_id)
                .with_priority(schedule.priority)
                .with_max_attempts(schedule.max_attempts);

            if let Err(e) = repo.create(job).await {
                tracing::warn!("Failed to create scheduled job: {}", e);
                continue;
            }

            // Compute next run
            let next_run = schedule_parsed
                .upcoming(chrono::Utc)
                .next()
                .unwrap_or(now + chrono::Duration::try_hours(24).unwrap_or(chrono::Duration::max_duration()));

            if let Err(e) = repo
                .update_schedule_next_run(schedule.id, next_run, now)
                .await
            {
                tracing::warn!("Failed to update schedule next_run: {}", e);
            }
        }

        Ok(())
    }

    /// Get dashboard counts for a tenant
    pub async fn dashboard(&self,
        tenant_id: i64,
    ) -> Result<JobCounts, ApiError> {
        self.repo.count_by_status(tenant_id).await
    }

    /// Get recent jobs for a tenant
    pub async fn recent_jobs(
        &self,
        tenant_id: i64,
        limit: i64,
    ) -> Result<Vec<Job>, ApiError> {
        self.repo.list_recent(tenant_id, limit).await
    }

    /// Create a recurring schedule
    pub async fn create_schedule(
        &self,
        schedule: CreateJobSchedule,
    ) -> Result<JobSchedule, ApiError> {
        // Validate cron expression
        Schedule::from_str(&schedule.cron_expression).map_err(|e| {
            ApiError::Validation(format!("Invalid cron expression: {}", e))
        })?;

        self.repo.create_schedule(schedule).await
    }

    /// List schedules for a tenant
    pub async fn list_schedules(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<JobSchedule>, ApiError> {
        self.repo.list_schedules(tenant_id).await
    }

    /// Toggle a schedule on/off
    pub async fn toggle_schedule(
        &self,
        id: i64,
        active: bool,
    ) -> Result<(), ApiError> {
        self.repo.toggle_schedule(id, active).await
    }

    /// List jobs by status for a tenant (used by API)
    pub async fn list_by_status(
        &self,
        tenant_id: i64,
        status: JobStatus,
    ) -> Result<Vec<Job>, ApiError> {
        self.repo.list_by_status(tenant_id, status).await
    }

    // Helper to map ApiError to String for JobScheduler trait compatibility
    fn map_err(e: ApiError) -> String {
        e.to_string()
    }
}

#[async_trait::async_trait]
impl JobScheduler for JobService {
    async fn schedule(&self,
        job: CommonCreateJob,
    ) -> Result<CommonJob, String> {
        let create = CreateJob {
            job_type: job.job_type,
            priority: job.priority,
            tenant_id: job.tenant_id,
            max_attempts: job.max_attempts,
            scheduled_at: job.scheduled_at,
        };
        let j = self.repo.create(create).await.map_err(Self::map_err)?;
        Ok(j.into())
    }

    async fn get_job(&self,
        id: i64,
    ) -> Result<Option<CommonJob>, String> {
        let j = self.repo.find_by_id(id).await.map_err(Self::map_err)?;
        Ok(j.map(Into::into))
    }

    async fn next_pending(&self,
    ) -> Result<Option<CommonJob>, String> {
        let j = self
            .repo
            .find_next_pending()
            .await
            .map_err(Self::map_err)?;
        Ok(j.map(Into::into))
    }

    async fn mark_running(&self,
        id: i64,
    ) -> Result<(), String> {
        self.repo.mark_running(id).await.map_err(Self::map_err)
    }

    async fn mark_completed(&self,
        id: i64,
    ) -> Result<(), String> {
        self.repo.mark_completed(id).await.map_err(Self::map_err)
    }

    async fn mark_failed(
        &self,
        id: i64,
        error: &str,
    ) -> Result<(), String> {
        self.repo.mark_failed(id, error).await.map_err(Self::map_err)
    }

    async fn cancel(&self,
        id: i64,
    ) -> Result<(), String> {
        self.repo.cancel(id).await.map_err(Self::map_err)
    }

    async fn list_by_status(
        &self,
        tenant_id: i64,
        status: crate::common::jobs::JobStatus,
    ) -> Result<Vec<CommonJob>, String> {
        let status: JobStatus = match status {
            crate::common::jobs::JobStatus::Pending => JobStatus::Pending,
            crate::common::jobs::JobStatus::Running => JobStatus::Running,
            crate::common::jobs::JobStatus::Completed => JobStatus::Completed,
            crate::common::jobs::JobStatus::Failed => JobStatus::Failed,
            crate::common::jobs::JobStatus::Cancelled => JobStatus::Cancelled,
            crate::common::jobs::JobStatus::Scheduled => JobStatus::Scheduled,
        };
        let jobs = self
            .repo
            .list_by_status(tenant_id, status)
            .await
            .map_err(Self::map_err)?;
        Ok(jobs.into_iter().map(Into::into).collect())
    }

    async fn retry(&self,
        id: i64,
    ) -> Result<(), String> {
        self.repo.retry(id).await.map_err(Self::map_err)
    }

    async fn cleanup(
        &self,
        older_than: Duration,
    ) -> Result<u64, String> {
        self.repo.cleanup(older_than).await.map_err(Self::map_err)
    }
}

// Convert between domain::job::model::Job and common::jobs::Job
impl From<Job> for CommonJob {
    fn from(job: Job) -> Self {
        Self {
            id: job.id,
            job_type: job.job_type,
            status: match job.status {
                JobStatus::Pending => crate::common::jobs::JobStatus::Pending,
                JobStatus::Running => crate::common::jobs::JobStatus::Running,
                JobStatus::Completed => crate::common::jobs::JobStatus::Completed,
                JobStatus::Failed => crate::common::jobs::JobStatus::Failed,
                JobStatus::Cancelled => crate::common::jobs::JobStatus::Cancelled,
                JobStatus::Scheduled => crate::common::jobs::JobStatus::Scheduled,
            },
            priority: match job.priority {
                JobPriority::Low => crate::common::jobs::JobPriority::Low,
                JobPriority::Normal => crate::common::jobs::JobPriority::Normal,
                JobPriority::High => crate::common::jobs::JobPriority::High,
                JobPriority::Critical => crate::common::jobs::JobPriority::Critical,
            },
            tenant_id: job.tenant_id,
            attempts: job.attempts,
            max_attempts: job.max_attempts,
            scheduled_at: job.scheduled_at,
            started_at: job.started_at,
            completed_at: job.completed_at,
            last_error: job.last_error,
            created_at: job.created_at,
        }
    }
}

impl From<CommonJob> for Job {
    fn from(job: CommonJob) -> Self {
        Self {
            id: job.id,
            job_type: job.job_type,
            status: match job.status {
                crate::common::jobs::JobStatus::Pending => JobStatus::Pending,
                crate::common::jobs::JobStatus::Running => JobStatus::Running,
                crate::common::jobs::JobStatus::Completed => JobStatus::Completed,
                crate::common::jobs::JobStatus::Failed => JobStatus::Failed,
                crate::common::jobs::JobStatus::Cancelled => JobStatus::Cancelled,
                crate::common::jobs::JobStatus::Scheduled => JobStatus::Scheduled,
            },
            priority: match job.priority {
                crate::common::jobs::JobPriority::Low => JobPriority::Low,
                crate::common::jobs::JobPriority::Normal => JobPriority::Normal,
                crate::common::jobs::JobPriority::High => JobPriority::High,
                crate::common::jobs::JobPriority::Critical => JobPriority::Critical,
            },
            tenant_id: job.tenant_id,
            attempts: job.attempts,
            max_attempts: job.max_attempts,
            scheduled_at: job.scheduled_at,
            started_at: job.started_at,
            completed_at: job.completed_at,
            last_error: job.last_error,
            created_at: job.created_at,
            updated_at: None,
            deleted_at: None,
            deleted_by: None,
        }
    }
}

impl JobService {
    /// Soft delete a job
    pub async fn soft_delete_job(
        &self,
        id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.repo.soft_delete(id, deleted_by).await
    }

    /// Restore a soft-deleted job
    pub async fn restore_job(&self, id: i64) -> Result<(), ApiError> {
        self.repo.restore(id).await
    }

    /// List deleted jobs for a tenant
    pub async fn deleted_jobs(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<Job>, ApiError> {
        self.repo.find_deleted(tenant_id).await
    }

    /// Permanently destroy a soft-deleted job
    pub async fn destroy_job(&self, id: i64) -> Result<(), ApiError> {
        self.repo.destroy(id).await
    }

    /// Soft delete a job schedule
    pub async fn soft_delete_schedule(
        &self,
        id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.repo.soft_delete_schedule(id, deleted_by).await
    }

    /// Restore a soft-deleted schedule
    pub async fn restore_schedule(&self, id: i64) -> Result<(), ApiError> {
        self.repo.restore_schedule(id).await
    }

    /// List deleted schedules for a tenant
    pub async fn deleted_schedules(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<JobSchedule>, ApiError> {
        self.repo.find_deleted_schedules(tenant_id).await
    }

    /// Permanently destroy a soft-deleted schedule
    pub async fn destroy_schedule(&self, id: i64) -> Result<(), ApiError> {
        self.repo.destroy_schedule(id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::job::repository::InMemoryJobRepository;

    #[tokio::test]
    async fn test_service_schedule_and_get() {
        let repo = Arc::new(InMemoryJobRepository::new());
        let svc = JobService::new(repo);

        let job = svc
            .schedule(CommonCreateJob::new(
                JobType::SendReminders { tenant_id: 1 },
                1,
            ))
            .await
            .unwrap();

        let found = svc.get_job(job.id).await.unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn test_service_mark_completed() {
        let repo = Arc::new(InMemoryJobRepository::new());
        let svc = JobService::new(repo);

        let job = svc
            .schedule(CommonCreateJob::new(
                JobType::ArchiveLogs {
                    tenant_id: 1,
                    older_than_days: 30,
                },
                1,
            ))
            .await
            .unwrap();

        svc.mark_running(job.id).await.unwrap();
        svc.mark_completed(job.id).await.unwrap();

        let found = svc.get_job(job.id).await.unwrap().unwrap();
        assert_eq!(
            found.status,
            crate::common::jobs::JobStatus::Completed
        );
    }

    #[tokio::test]
    async fn test_service_dashboard() {
        let repo = Arc::new(InMemoryJobRepository::new());
        let svc = JobService::new(repo);

        for _ in 0..2 {
            svc.schedule(CommonCreateJob::new(
                JobType::SendReminders { tenant_id: 1 },
                1,
            ))
            .await
            .unwrap();
        }

        let counts = svc.dashboard(1).await.unwrap();
        assert_eq!(counts.pending, 2);
    }

    #[tokio::test]
    async fn test_service_create_schedule_invalid_cron() {
        let repo = Arc::new(InMemoryJobRepository::new());
        let svc = JobService::new(repo);

        let result = svc
            .create_schedule(CreateJobSchedule {
                job_type: JobType::SendReminders { tenant_id: 1 },
                cron_expression: "invalid".to_string(),
                priority: JobPriority::Normal,
                tenant_id: 1,
                max_attempts: 3,
            })
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_service_create_schedule_valid_cron() {
        let repo = Arc::new(InMemoryJobRepository::new());
        let svc = JobService::new(repo);

        let schedule = svc
            .create_schedule(CreateJobSchedule {
                job_type: JobType::SendReminders { tenant_id: 1 },
                cron_expression: "0 0 * * *".to_string(),
                priority: JobPriority::Normal,
                tenant_id: 1,
                max_attempts: 3,
            })
            .await
            .unwrap();

        assert_eq!(schedule.cron_expression, "0 0 * * *");
    }

    #[tokio::test]
    async fn test_service_evaluate_schedules() {
        let repo = Arc::new(InMemoryJobRepository::new());
        let svc = JobService::new(repo.clone());

        // Create a schedule that is due (no next_run_at means it's due)
        let _ = svc
            .create_schedule(CreateJobSchedule {
                job_type: JobType::SendReminders { tenant_id: 1 },
                cron_expression: "0 0 * * *".to_string(),
                priority: JobPriority::Normal,
                tenant_id: 1,
                max_attempts: 3,
            })
            .await
            .unwrap();

        svc.start_background_tasks().await;

        // Give the background task a moment to run
        tokio::time::sleep(Duration::from_millis(100)).await;

        svc.shutdown().await;

        // After evaluation, a job should have been created
        let next = svc.next_pending().await.unwrap();
        assert!(next.is_some());
    }
}
