//! Background job model types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::impl_soft_deletable;

/// Job priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
pub enum JobPriority {
    Low,
    #[default]
    Normal,
    High,
    Critical,
}

impl std::fmt::Display for JobPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobPriority::Low => write!(f, "low"),
            JobPriority::Normal => write!(f, "normal"),
            JobPriority::High => write!(f, "high"),
            JobPriority::Critical => write!(f, "critical"),
        }
    }
}

impl From<String> for JobPriority {
    fn from(s: String) -> Self {
        match s.as_str() {
            "low" => JobPriority::Low,
            "high" => JobPriority::High,
            "critical" => JobPriority::Critical,
            _ => JobPriority::Normal,
        }
    }
}

/// Job status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
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

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobStatus::Pending => write!(f, "pending"),
            JobStatus::Running => write!(f, "running"),
            JobStatus::Completed => write!(f, "completed"),
            JobStatus::Failed => write!(f, "failed"),
            JobStatus::Cancelled => write!(f, "cancelled"),
            JobStatus::Scheduled => write!(f, "scheduled"),
        }
    }
}

impl From<String> for JobStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "running" => JobStatus::Running,
            "completed" => JobStatus::Completed,
            "failed" => JobStatus::Failed,
            "cancelled" => JobStatus::Cancelled,
            "scheduled" => JobStatus::Scheduled,
            _ => JobStatus::Pending,
        }
    }
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

impl JobType {
    /// Returns the discriminant name for DB storage
    pub fn type_name(&self) -> &'static str {
        match self {
            JobType::CalculateDepreciation { .. } => "calculate_depreciation",
            JobType::RunPayroll { .. } => "run_payroll",
            JobType::SendReminders { .. } => "send_reminders",
            JobType::ArchiveLogs { .. } => "archive_logs",
            JobType::GenerateReport { .. } => "generate_report",
            JobType::Custom { .. } => "custom",
            JobType::SendNotification { .. } => "send_notification",
        }
    }
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
    pub updated_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

impl_soft_deletable!(Job);

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

/// A recurring cron job schedule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSchedule {
    pub id: i64,
    pub job_type: JobType,
    pub cron_expression: String,
    pub priority: JobPriority,
    pub tenant_id: i64,
    pub max_attempts: u32,
    pub is_active: bool,
    pub next_run_at: Option<DateTime<Utc>>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

impl_soft_deletable!(JobSchedule);

/// Create a recurring job schedule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateJobSchedule {
    pub job_type: JobType,
    pub cron_expression: String,
    pub priority: JobPriority,
    pub tenant_id: i64,
    pub max_attempts: u32,
}

/// Job counts by status for dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobCounts {
    pub pending: i64,
    pub running: i64,
    pub completed: i64,
    pub failed: i64,
    pub cancelled: i64,
    pub scheduled: i64,
}

impl Default for JobCounts {
    fn default() -> Self {
        Self {
            pending: 0,
            running: 0,
            completed: 0,
            failed: 0,
            cancelled: 0,
            scheduled: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_priority_display() {
        assert_eq!(JobPriority::Low.to_string(), "low");
        assert_eq!(JobPriority::Normal.to_string(), "normal");
        assert_eq!(JobPriority::High.to_string(), "high");
        assert_eq!(JobPriority::Critical.to_string(), "critical");
    }

    #[test]
    fn test_job_status_display() {
        assert_eq!(JobStatus::Pending.to_string(), "pending");
        assert_eq!(JobStatus::Running.to_string(), "running");
        assert_eq!(JobStatus::Completed.to_string(), "completed");
    }

    #[test]
    fn test_job_type_name() {
        let jt = JobType::CalculateDepreciation {
            asset_id: 1,
            tenant_id: 2,
        };
        assert_eq!(jt.type_name(), "calculate_depreciation");
    }

    #[test]
    fn test_create_job_builder() {
        let cj = CreateJob::new(
            JobType::SendReminders { tenant_id: 1 },
            1,
        )
        .with_priority(JobPriority::High)
        .with_max_attempts(5);
        assert_eq!(cj.priority, JobPriority::High);
        assert_eq!(cj.max_attempts, 5);
    }
}
