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
                pa.cmp(&pb).then_with(|| a.created_at.cmp(&b.created_at))
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
}
