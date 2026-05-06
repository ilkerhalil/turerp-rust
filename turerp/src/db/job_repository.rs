//! PostgreSQL-backed job scheduler

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use std::sync::Arc;
use std::time::Duration;

use crate::common::jobs::{CreateJob, Job, JobPriority, JobScheduler, JobStatus, JobType};
use crate::error::ApiError;

/// Database row for a job
#[derive(Debug, FromRow)]
struct JobRow {
    id: i64,
    job_type: String,
    payload: sqlx::types::Json<serde_json::Value>,
    status: String,
    priority: String,
    tenant_id: i64,
    attempts: i32,
    max_attempts: i32,
    scheduled_at: Option<DateTime<Utc>>,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    last_error: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
}

impl From<JobRow> for Job {
    fn from(row: JobRow) -> Self {
        let job_type = match row.job_type.as_str() {
            "CalculateDepreciation" => JobType::CalculateDepreciation {
                asset_id: row
                    .payload
                    .0
                    .get("asset_id")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0),
                tenant_id: row.tenant_id,
            },
            "RunPayroll" => JobType::RunPayroll {
                tenant_id: row.tenant_id,
                period: row
                    .payload
                    .0
                    .get("period")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
            },
            "SendReminders" => JobType::SendReminders {
                tenant_id: row.tenant_id,
            },
            "ArchiveLogs" => JobType::ArchiveLogs {
                tenant_id: row.tenant_id,
                older_than_days: row
                    .payload
                    .0
                    .get("older_than_days")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(30) as i32,
            },
            "GenerateReport" => JobType::GenerateReport {
                tenant_id: row.tenant_id,
                report_type: row
                    .payload
                    .0
                    .get("report_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                params: row
                    .payload
                    .0
                    .get("params")
                    .and_then(|v| v.as_str())
                    .unwrap_or("{}")
                    .to_string(),
            },
            "SendNotification" => JobType::SendNotification {
                notification_id: row
                    .payload
                    .0
                    .get("notification_id")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0),
                tenant_id: row.tenant_id,
            },
            _ => JobType::Custom {
                name: row.job_type,
                payload: row.payload.0.to_string(),
            },
        };

        Self {
            id: row.id,
            job_type,
            status: parse_status(&row.status),
            priority: parse_priority(&row.priority),
            tenant_id: row.tenant_id,
            attempts: row.attempts as u32,
            max_attempts: row.max_attempts as u32,
            scheduled_at: row.scheduled_at,
            started_at: row.started_at,
            completed_at: row.completed_at,
            last_error: row.last_error,
            created_at: row.created_at,
        }
    }
}

fn parse_status(s: &str) -> JobStatus {
    match s {
        "pending" => JobStatus::Pending,
        "running" => JobStatus::Running,
        "completed" => JobStatus::Completed,
        "failed" => JobStatus::Failed,
        "cancelled" => JobStatus::Cancelled,
        "scheduled" => JobStatus::Scheduled,
        _ => JobStatus::Pending,
    }
}

fn parse_priority(s: &str) -> JobPriority {
    match s {
        "low" => JobPriority::Low,
        "normal" => JobPriority::Normal,
        "high" => JobPriority::High,
        "critical" => JobPriority::Critical,
        _ => JobPriority::Normal,
    }
}

fn priority_str(p: JobPriority) -> &'static str {
    match p {
        JobPriority::Low => "low",
        JobPriority::Normal => "normal",
        JobPriority::High => "high",
        JobPriority::Critical => "critical",
    }
}

fn status_str(s: JobStatus) -> &'static str {
    match s {
        JobStatus::Pending => "pending",
        JobStatus::Running => "running",
        JobStatus::Completed => "completed",
        JobStatus::Failed => "failed",
        JobStatus::Cancelled => "cancelled",
        JobStatus::Scheduled => "scheduled",
    }
}

fn job_payload_json(job_type: &JobType) -> serde_json::Value {
    match job_type {
        JobType::CalculateDepreciation { asset_id, .. } => {
            serde_json::json!({"asset_id": asset_id})
        }
        JobType::RunPayroll { period, .. } => {
            serde_json::json!({"period": period})
        }
        JobType::ArchiveLogs {
            older_than_days, ..
        } => {
            serde_json::json!({"older_than_days": older_than_days})
        }
        JobType::GenerateReport {
            report_type,
            params,
            ..
        } => {
            serde_json::json!({"report_type": report_type, "params": params})
        }
        JobType::SendNotification {
            notification_id, ..
        } => {
            serde_json::json!({"notification_id": notification_id})
        }
        JobType::Custom { payload, .. } => serde_json::from_str(payload)
            .unwrap_or_else(|_| serde_json::json!({"payload": payload})),
        _ => serde_json::json!({}),
    }
}

fn job_type_name(job_type: &JobType) -> String {
    match job_type {
        JobType::CalculateDepreciation { .. } => "CalculateDepreciation".to_string(),
        JobType::RunPayroll { .. } => "RunPayroll".to_string(),
        JobType::SendReminders { .. } => "SendReminders".to_string(),
        JobType::ArchiveLogs { .. } => "ArchiveLogs".to_string(),
        JobType::GenerateReport { .. } => "GenerateReport".to_string(),
        JobType::SendNotification { .. } => "SendNotification".to_string(),
        JobType::Custom { name, .. } => name.clone(),
    }
}

/// PostgreSQL-backed job scheduler
pub struct PostgresJobScheduler {
    pool: Arc<PgPool>,
}

impl PostgresJobScheduler {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl JobScheduler for PostgresJobScheduler {
    async fn schedule(&self, create: CreateJob) -> Result<Job, String> {
        let type_name = job_type_name(&create.job_type);
        let payload = job_payload_json(&create.job_type);
        let priority = priority_str(create.priority);
        let status = if create.scheduled_at.is_some() {
            "scheduled"
        } else {
            "pending"
        };

        let row = sqlx::query_as::<_, JobRow>(
            r#"
            INSERT INTO jobs (job_type, payload, status, priority, tenant_id, max_attempts, scheduled_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#
        )
        .bind(&type_name)
        .bind(sqlx::types::Json(payload))
        .bind(status)
        .bind(priority)
        .bind(create.tenant_id)
        .bind(create.max_attempts as i32)
        .bind(create.scheduled_at)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| format!("Failed to schedule job: {}", e))?;

        Ok(Job::from(row))
    }

    async fn get_job(&self, id: i64) -> Result<Option<Job>, String> {
        let row = sqlx::query_as::<_, JobRow>("SELECT * FROM jobs WHERE id = $1")
            .bind(id)
            .fetch_optional(&*self.pool)
            .await
            .map_err(|e| format!("Failed to get job: {}", e))?;
        Ok(row.map(Job::from))
    }

    async fn next_pending(&self) -> Result<Option<Job>, String> {
        let row = sqlx::query_as::<_, JobRow>(
            r#"
            SELECT * FROM jobs
            WHERE status = 'pending'
            AND (scheduled_at IS NULL OR scheduled_at <= NOW())
            ORDER BY
                CASE priority
                    WHEN 'critical' THEN 4
                    WHEN 'high' THEN 3
                    WHEN 'normal' THEN 2
                    WHEN 'low' THEN 1
                END DESC,
                created_at ASC
            LIMIT 1
            "#,
        )
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| format!("Failed to get next pending job: {}", e))?;
        Ok(row.map(Job::from))
    }

    async fn mark_running(&self, id: i64) -> Result<(), String> {
        sqlx::query(
            "UPDATE jobs SET status = 'running', started_at = NOW(), attempts = attempts + 1, updated_at = NOW() WHERE id = $1"
        )
        .bind(id)
        .execute(&*self.pool)
        .await
        .map_err(|e| format!("Failed to mark job running: {}", e))?;
        Ok(())
    }

    async fn mark_completed(&self, id: i64) -> Result<(), String> {
        sqlx::query(
            "UPDATE jobs SET status = 'completed', completed_at = NOW(), updated_at = NOW() WHERE id = $1"
        )
        .bind(id)
        .execute(&*self.pool)
        .await
        .map_err(|e| format!("Failed to mark job completed: {}", e))?;
        Ok(())
    }

    async fn mark_failed(&self, id: i64, error: &str) -> Result<(), String> {
        let job = self.get_job(id).await?;
        if let Some(job) = job {
            if job.attempts >= job.max_attempts {
                sqlx::query(
                    "UPDATE jobs SET status = 'failed', last_error = $1, completed_at = NOW(), updated_at = NOW() WHERE id = $2"
                )
                .bind(error)
                .bind(id)
                .execute(&*self.pool)
                .await
                .map_err(|e| format!("Failed to mark job failed: {}", e))?;
            } else {
                let backoff = chrono::Duration::seconds(2_i64.pow(job.attempts));
                let scheduled_at = Utc::now() + backoff;
                sqlx::query(
                    "UPDATE jobs SET status = 'pending', last_error = $1, scheduled_at = $2, updated_at = NOW() WHERE id = $3"
                )
                .bind(error)
                .bind(scheduled_at)
                .bind(id)
                .execute(&*self.pool)
                .await
                .map_err(|e| format!("Failed to retry job: {}", e))?;
            }
        }
        Ok(())
    }

    async fn cancel(&self, id: i64) -> Result<(), String> {
        sqlx::query(
            "UPDATE jobs SET status = 'cancelled', completed_at = NOW(), updated_at = NOW() WHERE id = $1 AND status IN ('pending', 'scheduled')"
        )
        .bind(id)
        .execute(&*self.pool)
        .await
        .map_err(|e| format!("Failed to cancel job: {}", e))?;
        Ok(())
    }

    async fn list_by_status(&self, tenant_id: i64, status: JobStatus) -> Result<Vec<Job>, String> {
        let rows = sqlx::query_as::<_, JobRow>(
            "SELECT * FROM jobs WHERE tenant_id = $1 AND status = $2 ORDER BY created_at DESC",
        )
        .bind(tenant_id)
        .bind(status_str(status))
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| format!("Failed to list jobs: {}", e))?;
        Ok(rows.into_iter().map(Job::from).collect())
    }

    async fn retry(&self, id: i64) -> Result<(), String> {
        sqlx::query(
            "UPDATE jobs SET status = 'pending', last_error = NULL, scheduled_at = NULL, completed_at = NULL, updated_at = NOW() WHERE id = $1"
        )
        .bind(id)
        .execute(&*self.pool)
        .await
        .map_err(|e| format!("Failed to retry job: {}", e))?;
        Ok(())
    }

    async fn cleanup(&self, older_than: Duration) -> Result<u64, String> {
        let cutoff = Utc::now()
            - chrono::Duration::from_std(older_than).unwrap_or(chrono::Duration::days(30));
        let result = sqlx::query(
            "DELETE FROM jobs WHERE status IN ('completed', 'failed', 'cancelled') AND completed_at < $1"
        )
        .bind(cutoff)
        .execute(&*self.pool)
        .await
        .map_err(|e| format!("Failed to cleanup jobs: {}", e))?;

        Ok(result.rows_affected())
    }
}
