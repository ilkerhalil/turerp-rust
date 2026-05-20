//! Archive domain models
//!
//! Provides types for data archiving policies, jobs, and archived records.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Status of an archive job
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, ToSchema)]
pub enum ArchiveJobStatus {
    #[default]
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for ArchiveJobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArchiveJobStatus::Pending => write!(f, "Pending"),
            ArchiveJobStatus::Running => write!(f, "Running"),
            ArchiveJobStatus::Completed => write!(f, "Completed"),
            ArchiveJobStatus::Failed => write!(f, "Failed"),
            ArchiveJobStatus::Cancelled => write!(f, "Cancelled"),
        }
    }
}

impl std::str::FromStr for ArchiveJobStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Pending" => Ok(ArchiveJobStatus::Pending),
            "Running" => Ok(ArchiveJobStatus::Running),
            "Completed" => Ok(ArchiveJobStatus::Completed),
            "Failed" => Ok(ArchiveJobStatus::Failed),
            "Cancelled" => Ok(ArchiveJobStatus::Cancelled),
            _ => Err(format!("Invalid archive job status: {}", s)),
        }
    }
}

/// An archive policy defining what data to archive
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ArchivePolicy {
    pub id: i64,
    pub tenant_id: i64,
    pub name: String,
    pub table_name: String,
    pub age_days: i32,
    pub conditions: Option<serde_json::Value>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// Response representation of an archive policy
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ArchivePolicyResponse {
    pub id: i64,
    pub tenant_id: i64,
    pub name: String,
    pub table_name: String,
    pub age_days: i32,
    pub conditions: Option<serde_json::Value>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<ArchivePolicy> for ArchivePolicyResponse {
    fn from(policy: ArchivePolicy) -> Self {
        Self {
            id: policy.id,
            tenant_id: policy.tenant_id,
            name: policy.name,
            table_name: policy.table_name,
            age_days: policy.age_days,
            conditions: policy.conditions,
            is_active: policy.is_active,
            created_at: policy.created_at,
            updated_at: policy.updated_at,
        }
    }
}

/// An archive job execution record
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ArchiveJob {
    pub id: i64,
    pub tenant_id: i64,
    pub policy_id: i64,
    pub status: ArchiveJobStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub records_archived: i64,
    pub records_failed: i64,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Response representation of an archive job
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ArchiveJobResponse {
    pub id: i64,
    pub tenant_id: i64,
    pub policy_id: i64,
    pub status: ArchiveJobStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub records_archived: i64,
    pub records_failed: i64,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<ArchiveJob> for ArchiveJobResponse {
    fn from(job: ArchiveJob) -> Self {
        Self {
            id: job.id,
            tenant_id: job.tenant_id,
            policy_id: job.policy_id,
            status: job.status,
            started_at: job.started_at,
            completed_at: job.completed_at,
            records_archived: job.records_archived,
            records_failed: job.records_failed,
            error_message: job.error_message,
            created_at: job.created_at,
        }
    }
}

/// A single archived record
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ArchiveRecord {
    pub id: i64,
    pub tenant_id: i64,
    pub source_table: String,
    pub source_id: i64,
    pub archived_data: serde_json::Value,
    pub archived_at: DateTime<Utc>,
    pub archive_job_id: i64,
    pub restored_at: Option<DateTime<Utc>>,
}

/// Response representation of an archived record
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ArchiveRecordResponse {
    pub id: i64,
    pub tenant_id: i64,
    pub source_table: String,
    pub source_id: i64,
    pub archived_data: serde_json::Value,
    pub archived_at: DateTime<Utc>,
    pub archive_job_id: i64,
    pub restored_at: Option<DateTime<Utc>>,
}

impl From<ArchiveRecord> for ArchiveRecordResponse {
    fn from(record: ArchiveRecord) -> Self {
        Self {
            id: record.id,
            tenant_id: record.tenant_id,
            source_table: record.source_table,
            source_id: record.source_id,
            archived_data: record.archived_data,
            archived_at: record.archived_at,
            archive_job_id: record.archive_job_id,
            restored_at: record.restored_at,
        }
    }
}

// ---- DTOs ----

/// Create a new archive policy
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateArchivePolicy {
    pub name: String,
    pub table_name: String,
    pub age_days: i32,
    pub conditions: Option<serde_json::Value>,
    #[serde(default = "default_active")]
    pub is_active: bool,
}

fn default_active() -> bool {
    true
}

/// Update an existing archive policy
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct UpdateArchivePolicy {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub table_name: Option<String>,
    #[serde(default)]
    pub age_days: Option<i32>,
    #[serde(default)]
    pub conditions: Option<serde_json::Value>,
    #[serde(default)]
    pub is_active: Option<bool>,
}

/// Create a new archive job
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateArchiveJob {
    pub policy_id: i64,
}

/// Request to restore archived records
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RestoreRequest {
    pub record_ids: Vec<i64>,
}

/// Failed item in a bulk restore operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BulkRestoreFailed {
    pub id: i64,
    pub reason: String,
}

/// Response for bulk restore operations
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BulkRestoreResponse<T> {
    pub restored: usize,
    pub items: Vec<T>,
    pub failed: Vec<BulkRestoreFailed>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_archive_job_status_display() {
        assert_eq!(ArchiveJobStatus::Pending.to_string(), "Pending");
        assert_eq!(ArchiveJobStatus::Running.to_string(), "Running");
        assert_eq!(ArchiveJobStatus::Completed.to_string(), "Completed");
        assert_eq!(ArchiveJobStatus::Failed.to_string(), "Failed");
        assert_eq!(ArchiveJobStatus::Cancelled.to_string(), "Cancelled");
    }

    #[test]
    fn test_archive_job_status_from_str() {
        assert_eq!(
            "Pending".parse::<ArchiveJobStatus>().unwrap(),
            ArchiveJobStatus::Pending
        );
        assert_eq!(
            "Running".parse::<ArchiveJobStatus>().unwrap(),
            ArchiveJobStatus::Running
        );
        assert_eq!(
            "Completed".parse::<ArchiveJobStatus>().unwrap(),
            ArchiveJobStatus::Completed
        );
        assert_eq!(
            "Failed".parse::<ArchiveJobStatus>().unwrap(),
            ArchiveJobStatus::Failed
        );
        assert_eq!(
            "Cancelled".parse::<ArchiveJobStatus>().unwrap(),
            ArchiveJobStatus::Cancelled
        );
        assert!("INVALID".parse::<ArchiveJobStatus>().is_err());
    }

    #[test]
    fn test_archive_policy_response_from_policy() {
        let policy = ArchivePolicy {
            id: 1,
            tenant_id: 100,
            name: "Old Invoices".to_string(),
            table_name: "invoices".to_string(),
            age_days: 365,
            conditions: None,
            is_active: true,
            created_at: Utc::now(),
            updated_at: None,
        };

        let resp = ArchivePolicyResponse::from(policy);
        assert_eq!(resp.id, 1);
        assert_eq!(resp.table_name, "invoices");
        assert_eq!(resp.age_days, 365);
    }

    #[test]
    fn test_archive_job_response_from_job() {
        let job = ArchiveJob {
            id: 1,
            tenant_id: 100,
            policy_id: 1,
            status: ArchiveJobStatus::Completed,
            started_at: Some(Utc::now()),
            completed_at: Some(Utc::now()),
            records_archived: 500,
            records_failed: 0,
            error_message: None,
            created_at: Utc::now(),
        };

        let resp = ArchiveJobResponse::from(job);
        assert_eq!(resp.records_archived, 500);
        assert_eq!(resp.status, ArchiveJobStatus::Completed);
    }

    #[test]
    fn test_archive_record_response_from_record() {
        let record = ArchiveRecord {
            id: 1,
            tenant_id: 100,
            source_table: "invoices".to_string(),
            source_id: 42,
            archived_data: serde_json::json!({"amount": 1000}),
            archived_at: Utc::now(),
            archive_job_id: 1,
            restored_at: None,
        };

        let resp = ArchiveRecordResponse::from(record);
        assert_eq!(resp.source_id, 42);
        assert!(resp.restored_at.is_none());
    }
}
