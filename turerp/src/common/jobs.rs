//! Background job scheduler with in-memory and PostgreSQL backends
//!
//! Provides a `JobScheduler` trait for scheduling and executing background
//! tasks such as depreciation calculations, payroll runs, notifications,
//! and log archival. Supports retry with exponential backoff and cron
//! expression scheduling.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

/// Job priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum JobPriority {
    Low,
    #[default]
    Normal,
    High,
    Critical,
}

/// Job status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    /// Job is waiting to be executed
    Pending,
    /// Job is currently being executed
    Running,
    /// Job completed successfully
    Completed,
    /// Job failed after all retries
    Failed,
    /// Job was cancelled
    Cancelled,
    /// Job is scheduled for future execution
    Scheduled,
}

/// Job types supported by the scheduler
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum JobType {
    /// Calculate asset depreciation
    CalculateDepreciation { asset_id: i64, tenant_id: i64 },
    /// Run payroll for a period
    RunPayroll { tenant_id: i64, period: String },
    /// Send reminders for overdue invoices
    SendReminders { tenant_id: i64 },
    /// Archive old audit logs
    ArchiveLogs {
        tenant_id: i64,
        older_than_days: i32,
    },
    /// Generate reports
    GenerateReport {
        tenant_id: i64,
        report_type: String,
        params: String,
    },
    /// Custom job with arbitrary payload
    Custom { name: String, payload: String },
    /// Send a notification via email, SMS, or in-app
    SendNotification {
        notification_id: i64,
        tenant_id: i64,
    },
}

/// A scheduled job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: i64,
    pub job_type: JobType,
    pub status: JobStatus,
    pub priority: JobPriority,
    pub tenant_id: i64,
    pub attempts: u32,
    pub max_attempts: u32,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Create job request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateJob {
    pub job_type: JobType,
    pub priority: JobPriority,
    pub tenant_id: i64,
    pub max_attempts: u32,
    pub scheduled_at: Option<DateTime<Utc>>,
}

impl CreateJob {
    /// Create a new job with default settings
    pub fn new(job_type: JobType, tenant_id: i64) -> Self {
        Self {
            job_type,
            priority: JobPriority::Normal,
            tenant_id,
            max_attempts: 3,
            scheduled_at: None,
        }
    }

    /// Set job priority
    pub fn with_priority(mut self, priority: JobPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Schedule for future execution
    pub fn with_scheduled_at(mut self, scheduled_at: DateTime<Utc>) -> Self {
        self.scheduled_at = Some(scheduled_at);
        self
    }

    /// Set maximum retry attempts
    pub fn with_max_attempts(mut self, max_attempts: u32) -> Self {
        self.max_attempts = max_attempts;
        self
    }
}

/// Job scheduler trait
#[async_trait::async_trait]
pub trait JobScheduler: Send + Sync {
    /// Schedule a new job
    async fn schedule(&self, job: CreateJob) -> Result<Job, String>;

    /// Get a job by ID
    async fn get_job(&self, id: i64) -> Result<Option<Job>, String>;

    /// Get the next pending job (for worker processes)
    async fn next_pending(&self) -> Result<Option<Job>, String>;

    /// Mark a job as running
    async fn mark_running(&self, id: i64) -> Result<(), String>;

    /// Mark a job as completed
    async fn mark_completed(&self, id: i64) -> Result<(), String>;

    /// Mark a job as failed (with error message)
    async fn mark_failed(&self, id: i64, error: &str) -> Result<(), String>;

    /// Cancel a pending job
    async fn cancel(&self, id: i64) -> Result<(), String>;

    /// List jobs by status for a tenant
    async fn list_by_status(&self, tenant_id: i64, status: JobStatus) -> Result<Vec<Job>, String>;

    /// Retry a failed job
    async fn retry(&self, id: i64) -> Result<(), String>;

    /// Clean up old completed/failed jobs
    async fn cleanup(&self, older_than: Duration) -> Result<u64, String>;
}

/// In-memory job scheduler for development
pub struct InMemoryJobScheduler {
    jobs: parking_lot::RwLock<Vec<Job>>,
    next_id: parking_lot::RwLock<i64>,
}

impl InMemoryJobScheduler {
    pub fn new() -> Self {
        Self {
            jobs: parking_lot::RwLock::new(Vec::new()),
            next_id: parking_lot::RwLock::new(1),
        }
    }

    fn allocate_id(&self) -> i64 {
        let mut id = self.next_id.write();
        let job_id = *id;
        *id += 1;
        job_id
    }
}

impl Default for InMemoryJobScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl JobScheduler for InMemoryJobScheduler {
    async fn schedule(&self, create: CreateJob) -> Result<Job, String> {
        let id = self.allocate_id();
        let is_future = create.scheduled_at.is_some_and(|s| s > Utc::now());
        let job = Job {
            id,
            job_type: create.job_type,
            status: if is_future {
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
        };
        self.jobs.write().push(job.clone());
        Ok(job)
    }

    async fn get_job(&self, id: i64) -> Result<Option<Job>, String> {
        Ok(self.jobs.read().iter().find(|j| j.id == id).cloned())
    }

    async fn next_pending(&self) -> Result<Option<Job>, String> {
        let jobs = self.jobs.read();
        Ok(jobs
            .iter()
            .filter(|j| {
                j.status == JobStatus::Pending && j.scheduled_at.is_none_or(|s| s <= Utc::now())
            })
            .max_by(|a, b| {
                // Higher priority first, then earlier creation
                let pa = match a.priority {
                    JobPriority::Critical => 4,
                    JobPriority::High => 3,
                    JobPriority::Normal => 2,
                    JobPriority::Low => 1,
                };
                let pb = match b.priority {
                    JobPriority::Critical => 4,
                    JobPriority::High => 3,
                    JobPriority::Normal => 2,
                    JobPriority::Low => 1,
                };
                pa.cmp(&pb).then_with(|| b.created_at.cmp(&a.created_at))
            })
            .cloned())
    }

    async fn mark_running(&self, id: i64) -> Result<(), String> {
        let mut jobs = self.jobs.write();
        let job = jobs
            .iter_mut()
            .find(|j| j.id == id)
            .ok_or_else(|| format!("Job {} not found", id))?;
        job.status = JobStatus::Running;
        job.started_at = Some(Utc::now());
        job.attempts += 1;
        Ok(())
    }

    async fn mark_completed(&self, id: i64) -> Result<(), String> {
        let mut jobs = self.jobs.write();
        let job = jobs
            .iter_mut()
            .find(|j| j.id == id)
            .ok_or_else(|| format!("Job {} not found", id))?;
        job.status = JobStatus::Completed;
        job.completed_at = Some(Utc::now());
        Ok(())
    }

    async fn mark_failed(&self, id: i64, error: &str) -> Result<(), String> {
        let mut jobs = self.jobs.write();
        let job = jobs
            .iter_mut()
            .find(|j| j.id == id)
            .ok_or_else(|| format!("Job {} not found", id))?;
        job.last_error = Some(error.to_string());
        if job.attempts >= job.max_attempts {
            job.status = JobStatus::Failed;
            job.completed_at = Some(Utc::now());
        } else {
            // Retry: back to pending with exponential backoff
            job.status = JobStatus::Pending;
            job.scheduled_at =
                Some(Utc::now() + chrono::Duration::seconds(2_i64.pow(job.attempts)));
        }
        Ok(())
    }

    async fn cancel(&self, id: i64) -> Result<(), String> {
        let mut jobs = self.jobs.write();
        let job = jobs
            .iter_mut()
            .find(|j| j.id == id)
            .ok_or_else(|| format!("Job {} not found", id))?;
        if job.status != JobStatus::Pending && job.status != JobStatus::Scheduled {
            return Err("Can only cancel pending or scheduled jobs".to_string());
        }
        job.status = JobStatus::Cancelled;
        job.completed_at = Some(Utc::now());
        Ok(())
    }

    async fn list_by_status(&self, tenant_id: i64, status: JobStatus) -> Result<Vec<Job>, String> {
        Ok(self
            .jobs
            .read()
            .iter()
            .filter(|j| j.tenant_id == tenant_id && j.status == status)
            .cloned()
            .collect())
    }

    async fn retry(&self, id: i64) -> Result<(), String> {
        let mut jobs = self.jobs.write();
        let job = jobs
            .iter_mut()
            .find(|j| j.id == id)
            .ok_or_else(|| format!("Job {} not found", id))?;
        if job.status != JobStatus::Failed {
            return Err("Can only retry failed jobs".to_string());
        }
        job.status = JobStatus::Pending;
        job.attempts = 0;
        job.last_error = None;
        job.scheduled_at = None;
        job.started_at = None;
        job.completed_at = None;
        Ok(())
    }

    async fn cleanup(&self, older_than: Duration) -> Result<u64, String> {
        let cutoff =
            Utc::now() - chrono::Duration::from_std(older_than).unwrap_or(chrono::Duration::MAX);
        let mut jobs = self.jobs.write();
        let before = jobs.len();
        jobs.retain(|j| {
            !(j.status == JobStatus::Completed
                || j.status == JobStatus::Failed
                || j.status == JobStatus::Cancelled)
                || j.completed_at.is_none_or(|c| c > cutoff)
        });
        Ok((before - jobs.len()) as u64)
    }
}

/// Type alias for boxed job scheduler
pub type BoxJobScheduler = Arc<dyn JobScheduler>;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_schedule_job() {
        let scheduler = InMemoryJobScheduler::new();
        let job = scheduler
            .schedule(CreateJob::new(JobType::SendReminders { tenant_id: 1 }, 1))
            .await
            .unwrap();
        assert_eq!(job.status, JobStatus::Pending);
        assert_eq!(job.tenant_id, 1);
    }

    #[tokio::test]
    async fn test_job_lifecycle() {
        let scheduler = InMemoryJobScheduler::new();
        let job = scheduler
            .schedule(CreateJob::new(
                JobType::ArchiveLogs {
                    tenant_id: 1,
                    older_than_days: 30,
                },
                1,
            ))
            .await
            .unwrap();
        let id = job.id;

        // Get next pending
        let pending = scheduler.next_pending().await.unwrap().unwrap();
        assert_eq!(pending.id, id);

        // Mark running
        scheduler.mark_running(id).await.unwrap();
        let running = scheduler.get_job(id).await.unwrap().unwrap();
        assert_eq!(running.status, JobStatus::Running);
        assert_eq!(running.attempts, 1);

        // Mark completed
        scheduler.mark_completed(id).await.unwrap();
        let completed = scheduler.get_job(id).await.unwrap().unwrap();
        assert_eq!(completed.status, JobStatus::Completed);
        assert!(completed.completed_at.is_some());
    }

    #[tokio::test]
    async fn test_job_failure_retry() {
        let scheduler = InMemoryJobScheduler::new();
        let job = scheduler
            .schedule(
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
        let id = job.id;

        scheduler.mark_running(id).await.unwrap();
        scheduler.mark_failed(id, "Database error").await.unwrap();

        // Should be back to pending (retry)
        let retry_job = scheduler.get_job(id).await.unwrap().unwrap();
        assert_eq!(retry_job.status, JobStatus::Pending);
        assert!(retry_job.scheduled_at.is_some()); // Scheduled for later
    }

    #[tokio::test]
    async fn test_job_failure_max_retries() {
        let scheduler = InMemoryJobScheduler::new();
        let job = scheduler
            .schedule(
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
        let id = job.id;

        scheduler.mark_running(id).await.unwrap();
        scheduler.mark_failed(id, "Fatal error").await.unwrap();

        let failed = scheduler.get_job(id).await.unwrap().unwrap();
        assert_eq!(failed.status, JobStatus::Failed);
    }

    #[tokio::test]
    async fn test_cancel_job() {
        let scheduler = InMemoryJobScheduler::new();
        let job = scheduler
            .schedule(CreateJob::new(JobType::SendReminders { tenant_id: 1 }, 1))
            .await
            .unwrap();

        scheduler.cancel(job.id).await.unwrap();
        let cancelled = scheduler.get_job(job.id).await.unwrap().unwrap();
        assert_eq!(cancelled.status, JobStatus::Cancelled);
    }

    #[tokio::test]
    async fn test_priority_ordering() {
        let scheduler = InMemoryJobScheduler::new();

        scheduler
            .schedule(
                CreateJob::new(JobType::SendReminders { tenant_id: 1 }, 1)
                    .with_priority(JobPriority::Low),
            )
            .await
            .unwrap();
        scheduler
            .schedule(
                CreateJob::new(JobType::SendReminders { tenant_id: 1 }, 1)
                    .with_priority(JobPriority::Critical),
            )
            .await
            .unwrap();
        scheduler
            .schedule(
                CreateJob::new(JobType::SendReminders { tenant_id: 1 }, 1)
                    .with_priority(JobPriority::Normal),
            )
            .await
            .unwrap();

        let next = scheduler.next_pending().await.unwrap().unwrap();
        assert_eq!(next.priority, JobPriority::Critical);
    }

    #[tokio::test]
    async fn test_list_by_status() {
        let scheduler = InMemoryJobScheduler::new();

        scheduler
            .schedule(CreateJob::new(JobType::SendReminders { tenant_id: 1 }, 1))
            .await
            .unwrap();
        scheduler
            .schedule(CreateJob::new(JobType::SendReminders { tenant_id: 1 }, 1))
            .await
            .unwrap();

        let pending = scheduler
            .list_by_status(1, JobStatus::Pending)
            .await
            .unwrap();
        assert_eq!(pending.len(), 2);

        let completed = scheduler
            .list_by_status(1, JobStatus::Completed)
            .await
            .unwrap();
        assert!(completed.is_empty());
    }

    #[tokio::test]
    async fn test_cleanup() {
        let scheduler = InMemoryJobScheduler::new();

        let job = scheduler
            .schedule(CreateJob::new(JobType::SendReminders { tenant_id: 1 }, 1))
            .await
            .unwrap();

        scheduler.mark_running(job.id).await.unwrap();
        scheduler.mark_completed(job.id).await.unwrap();

        let cleaned = scheduler.cleanup(Duration::from_secs(0)).await.unwrap();
        assert_eq!(cleaned, 1);
    }

    #[tokio::test]
    async fn test_retry_failed_job() {
        let scheduler = InMemoryJobScheduler::new();
        let job = scheduler
            .schedule(
                CreateJob::new(JobType::SendReminders { tenant_id: 1 }, 1).with_max_attempts(1),
            )
            .await
            .unwrap();

        scheduler.mark_running(job.id).await.unwrap();
        scheduler.mark_failed(job.id, "error").await.unwrap();

        scheduler.retry(job.id).await.unwrap();
        let retried = scheduler.get_job(job.id).await.unwrap().unwrap();
        assert_eq!(retried.status, JobStatus::Pending);
        assert_eq!(retried.attempts, 0);
    }

    #[tokio::test]
    async fn test_schedule_all_job_types() {
        let scheduler = InMemoryJobScheduler::new();

        let types = vec![
            JobType::CalculateDepreciation {
                asset_id: 1,
                tenant_id: 1,
            },
            JobType::RunPayroll {
                tenant_id: 1,
                period: "2024-01".to_string(),
            },
            JobType::SendReminders { tenant_id: 1 },
            JobType::ArchiveLogs {
                tenant_id: 1,
                older_than_days: 30,
            },
            JobType::GenerateReport {
                tenant_id: 1,
                report_type: "balance_sheet".to_string(),
                params: "{}".to_string(),
            },
            JobType::SendNotification {
                notification_id: 1,
                tenant_id: 1,
            },
            JobType::Custom {
                name: "test".to_string(),
                payload: "data".to_string(),
            },
        ];

        for (i, job_type) in types.into_iter().enumerate() {
            let job = scheduler
                .schedule(CreateJob::new(job_type, 1))
                .await
                .unwrap();
            assert_eq!(job.id, (i + 1) as i64);
            assert_eq!(job.status, JobStatus::Pending);
        }
    }

    #[tokio::test]
    async fn test_scheduled_job_not_picked_up_early() {
        let scheduler = InMemoryJobScheduler::new();
        let future = Utc::now() + chrono::Duration::seconds(3600);
        let job = scheduler
            .schedule(
                CreateJob::new(JobType::SendReminders { tenant_id: 1 }, 1)
                    .with_scheduled_at(future),
            )
            .await
            .unwrap();

        assert_eq!(job.status, JobStatus::Scheduled);

        let next = scheduler.next_pending().await.unwrap();
        assert!(next.is_none(), "Future job should not be picked up");
    }

    #[tokio::test]
    async fn test_scheduled_job_picked_up_after_time() {
        let scheduler = InMemoryJobScheduler::new();
        let past = Utc::now() - chrono::Duration::seconds(1);
        let job = scheduler
            .schedule(
                CreateJob::new(JobType::SendReminders { tenant_id: 1 }, 1).with_scheduled_at(past),
            )
            .await
            .unwrap();

        let next = scheduler.next_pending().await.unwrap();
        assert_eq!(next.unwrap().id, job.id);
    }

    #[tokio::test]
    async fn test_exponential_backoff_timing() {
        let scheduler = InMemoryJobScheduler::new();
        let job = scheduler
            .schedule(
                CreateJob::new(JobType::SendReminders { tenant_id: 1 }, 1).with_max_attempts(5),
            )
            .await
            .unwrap();
        let id = job.id;

        // First failure: backoff = 2^1 = 2 seconds
        scheduler.mark_running(id).await.unwrap();
        scheduler.mark_failed(id, "err1").await.unwrap();
        let j1 = scheduler.get_job(id).await.unwrap().unwrap();
        assert_eq!(j1.attempts, 1);
        let delay1 = j1.scheduled_at.unwrap() - Utc::now();
        assert!(delay1.num_seconds() >= 1 && delay1.num_seconds() <= 3);

        // Second failure: backoff = 2^2 = 4 seconds
        scheduler.mark_running(id).await.unwrap();
        scheduler.mark_failed(id, "err2").await.unwrap();
        let j2 = scheduler.get_job(id).await.unwrap().unwrap();
        assert_eq!(j2.attempts, 2);
        let delay2 = j2.scheduled_at.unwrap() - Utc::now();
        assert!(delay2.num_seconds() >= 3 && delay2.num_seconds() <= 5);

        // Third failure: backoff = 2^3 = 8 seconds
        scheduler.mark_running(id).await.unwrap();
        scheduler.mark_failed(id, "err3").await.unwrap();
        let j3 = scheduler.get_job(id).await.unwrap().unwrap();
        assert_eq!(j3.attempts, 3);
        let delay3 = j3.scheduled_at.unwrap() - Utc::now();
        assert!(delay3.num_seconds() >= 7 && delay3.num_seconds() <= 9);
    }

    #[tokio::test]
    async fn test_cancel_running_job_fails() {
        let scheduler = InMemoryJobScheduler::new();
        let job = scheduler
            .schedule(CreateJob::new(JobType::SendReminders { tenant_id: 1 }, 1))
            .await
            .unwrap();

        scheduler.mark_running(job.id).await.unwrap();
        let result = scheduler.cancel(job.id).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Can only cancel"));
    }

    #[tokio::test]
    async fn test_retry_non_failed_job_fails() {
        let scheduler = InMemoryJobScheduler::new();
        let job = scheduler
            .schedule(CreateJob::new(JobType::SendReminders { tenant_id: 1 }, 1))
            .await
            .unwrap();

        let result = scheduler.retry(job.id).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Can only retry failed jobs"));
    }

    #[tokio::test]
    async fn test_cleanup_preserves_incomplete_jobs() {
        let scheduler = InMemoryJobScheduler::new();

        // Pending job
        scheduler
            .schedule(CreateJob::new(JobType::SendReminders { tenant_id: 1 }, 1))
            .await
            .unwrap();

        // Running job
        let running = scheduler
            .schedule(CreateJob::new(JobType::SendReminders { tenant_id: 1 }, 1))
            .await
            .unwrap();
        scheduler.mark_running(running.id).await.unwrap();

        // Completed job
        let completed = scheduler
            .schedule(CreateJob::new(JobType::SendReminders { tenant_id: 1 }, 1))
            .await
            .unwrap();
        scheduler.mark_running(completed.id).await.unwrap();
        scheduler.mark_completed(completed.id).await.unwrap();

        // Cleanup with 0 duration should remove only completed
        let cleaned = scheduler.cleanup(Duration::from_secs(0)).await.unwrap();
        assert_eq!(cleaned, 1);

        let all = scheduler
            .list_by_status(1, JobStatus::Pending)
            .await
            .unwrap();
        assert_eq!(all.len(), 1);

        let running_list = scheduler
            .list_by_status(1, JobStatus::Running)
            .await
            .unwrap();
        assert_eq!(running_list.len(), 1);
    }

    #[tokio::test]
    async fn test_cleanup_respects_cutoff() {
        let scheduler = InMemoryJobScheduler::new();

        // Old completed job
        let old = scheduler
            .schedule(CreateJob::new(JobType::SendReminders { tenant_id: 1 }, 1))
            .await
            .unwrap();
        scheduler.mark_running(old.id).await.unwrap();
        scheduler.mark_completed(old.id).await.unwrap();

        // Immediate cleanup with huge duration should keep everything
        let cleaned = scheduler
            .cleanup(Duration::from_secs(86400 * 365))
            .await
            .unwrap();
        assert_eq!(cleaned, 0);
    }

    #[tokio::test]
    async fn test_priority_tie_breaker() {
        let scheduler = InMemoryJobScheduler::new();

        let job1 = scheduler
            .schedule(CreateJob::new(JobType::SendReminders { tenant_id: 1 }, 1))
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
        let job2 = scheduler
            .schedule(CreateJob::new(JobType::SendReminders { tenant_id: 1 }, 1))
            .await
            .unwrap();

        let next = scheduler.next_pending().await.unwrap().unwrap();
        assert_eq!(next.id, job1.id, "Earlier created job should win tie");
    }

    #[tokio::test]
    async fn test_tenant_isolation() {
        let scheduler = InMemoryJobScheduler::new();

        scheduler
            .schedule(CreateJob::new(JobType::SendReminders { tenant_id: 1 }, 1))
            .await
            .unwrap();
        scheduler
            .schedule(CreateJob::new(JobType::SendReminders { tenant_id: 2 }, 2))
            .await
            .unwrap();
        scheduler
            .schedule(CreateJob::new(JobType::SendReminders { tenant_id: 1 }, 1))
            .await
            .unwrap();

        let tenant1 = scheduler
            .list_by_status(1, JobStatus::Pending)
            .await
            .unwrap();
        assert_eq!(tenant1.len(), 2);

        let tenant2 = scheduler
            .list_by_status(2, JobStatus::Pending)
            .await
            .unwrap();
        assert_eq!(tenant2.len(), 1);
    }

    #[tokio::test]
    async fn test_get_job_not_found() {
        let scheduler = InMemoryJobScheduler::new();
        let result = scheduler.get_job(999).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_mark_running_not_found() {
        let scheduler = InMemoryJobScheduler::new();
        let result = scheduler.mark_running(999).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[tokio::test]
    async fn test_mark_completed_not_found() {
        let scheduler = InMemoryJobScheduler::new();
        let result = scheduler.mark_completed(999).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mark_failed_not_found() {
        let scheduler = InMemoryJobScheduler::new();
        let result = scheduler.mark_failed(999, "error").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_retry_not_found() {
        let scheduler = InMemoryJobScheduler::new();
        let result = scheduler.retry(999).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[tokio::test]
    async fn test_cancel_not_found() {
        let scheduler = InMemoryJobScheduler::new();
        let result = scheduler.cancel(999).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[tokio::test]
    async fn test_job_status_transitions() {
        let scheduler = InMemoryJobScheduler::new();
        let job = scheduler
            .schedule(
                CreateJob::new(JobType::SendReminders { tenant_id: 1 }, 1).with_max_attempts(1),
            )
            .await
            .unwrap();
        let id = job.id;

        // Pending -> Running
        scheduler.mark_running(id).await.unwrap();
        let j = scheduler.get_job(id).await.unwrap().unwrap();
        assert_eq!(j.status, JobStatus::Running);
        assert_eq!(j.attempts, 1);
        assert!(j.started_at.is_some());

        // Running -> Failed (max attempts reached)
        scheduler.mark_failed(id, "boom").await.unwrap();
        let j = scheduler.get_job(id).await.unwrap().unwrap();
        assert_eq!(j.status, JobStatus::Failed);
        assert_eq!(j.last_error, Some("boom".to_string()));
        assert!(j.completed_at.is_some());
    }

    #[tokio::test]
    async fn test_multiple_retry_cycles() {
        let scheduler = InMemoryJobScheduler::new();
        let job = scheduler
            .schedule(
                CreateJob::new(JobType::SendReminders { tenant_id: 1 }, 1).with_max_attempts(3),
            )
            .await
            .unwrap();
        let id = job.id;

        // Attempt 1 fails -> retry
        scheduler.mark_running(id).await.unwrap();
        scheduler.mark_failed(id, "a").await.unwrap();
        let j = scheduler.get_job(id).await.unwrap().unwrap();
        assert_eq!(j.status, JobStatus::Pending);
        assert_eq!(j.attempts, 1);

        // Attempt 2 fails -> retry
        scheduler.mark_running(id).await.unwrap();
        scheduler.mark_failed(id, "b").await.unwrap();
        let j = scheduler.get_job(id).await.unwrap().unwrap();
        assert_eq!(j.status, JobStatus::Pending);
        assert_eq!(j.attempts, 2);

        // Attempt 3 fails -> final failure
        scheduler.mark_running(id).await.unwrap();
        scheduler.mark_failed(id, "c").await.unwrap();
        let j = scheduler.get_job(id).await.unwrap().unwrap();
        assert_eq!(j.status, JobStatus::Failed);
        assert_eq!(j.attempts, 3);
        assert_eq!(j.last_error, Some("c".to_string()));
    }
}
