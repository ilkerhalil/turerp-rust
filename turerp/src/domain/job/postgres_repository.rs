//! PostgreSQL job repository implementation

use std::time::Duration;

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

use crate::db::error::map_sqlx_error;
use crate::domain::job::model::{
    CreateJob, CreateJobSchedule, Job, JobCounts, JobPriority, JobSchedule, JobStatus, JobType,
};
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
    deleted_at: Option<DateTime<Utc>>,
    deleted_by: Option<i64>,
}

impl TryFrom<JobRow> for Job {
    type Error = ApiError;

    fn try_from(row: JobRow) -> Result<Self, Self::Error> {
        let job_type: JobType = serde_json::from_value(row.payload.0).map_err(|e| {
            ApiError::Internal(format!("Failed to deserialize job payload: {}", e))
        })?;
        Ok(Job {
            id: row.id,
            job_type,
            status: row.status.into(),
            priority: row.priority.into(),
            tenant_id: row.tenant_id,
            attempts: row.attempts as u32,
            max_attempts: row.max_attempts as u32,
            scheduled_at: row.scheduled_at,
            started_at: row.started_at,
            completed_at: row.completed_at,
            last_error: row.last_error,
            created_at: row.created_at,
            updated_at: row.updated_at,
            deleted_at: row.deleted_at,
            deleted_by: row.deleted_by,
        })
    }
}

/// Database row for a job schedule
#[derive(Debug, FromRow)]
struct JobScheduleRow {
    id: i64,
    job_type: String,
    payload: sqlx::types::Json<serde_json::Value>,
    cron_expression: String,
    priority: String,
    tenant_id: i64,
    max_attempts: i32,
    is_active: bool,
    next_run_at: Option<DateTime<Utc>>,
    last_run_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
    deleted_at: Option<DateTime<Utc>>,
    deleted_by: Option<i64>,
}

impl TryFrom<JobScheduleRow> for JobSchedule {
    type Error = ApiError;

    fn try_from(row: JobScheduleRow) -> Result<Self, Self::Error> {
        let job_type: JobType = serde_json::from_value(row.payload.0).map_err(|e| {
            ApiError::Internal(format!("Failed to deserialize schedule payload: {}", e))
        })?;
        Ok(JobSchedule {
            id: row.id,
            job_type,
            cron_expression: row.cron_expression,
            priority: row.priority.into(),
            tenant_id: row.tenant_id,
            max_attempts: row.max_attempts as u32,
            is_active: row.is_active,
            next_run_at: row.next_run_at,
            last_run_at: row.last_run_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
            deleted_at: row.deleted_at,
            deleted_by: row.deleted_by,
        })
    }
}

/// PostgreSQL job repository
#[derive(Clone)]
pub struct PostgresJobRepository {
    pool: PgPool,
}

impl PostgresJobRepository {
    /// Create a new repository backed by a PostgreSQL pool
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Convert into a boxed trait object
    pub fn into_boxed(self) -> super::BoxJobRepository {
        std::sync::Arc::new(self) as super::BoxJobRepository
    }
}

#[async_trait::async_trait]
impl super::JobRepository for PostgresJobRepository {
    async fn create(&self,
        job: CreateJob,
    ) -> Result<Job, ApiError> {
        let payload =
            serde_json::to_value(&job.job_type).map_err(|e| ApiError::Internal(e.to_string()))?;
        let status = if job.scheduled_at.is_some() {
            "scheduled"
        } else {
            "pending"
        };

        let row = sqlx::query_as::<_, JobRow>(
            r#"
            INSERT INTO jobs (job_type, payload, status, priority, tenant_id, max_attempts, scheduled_at, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
            RETURNING *
            "#,
        )
        .bind(job.job_type.type_name())
        .bind(sqlx::types::Json(payload))
        .bind(status)
        .bind(job.priority.to_string())
        .bind(job.tenant_id)
        .bind(job.max_attempts as i32)
        .bind(job.scheduled_at)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Job"))?;

        row.try_into()
    }

    async fn find_by_id(
        &self,
        id: i64,
    ) -> Result<Option<Job>, ApiError> {
        let row = sqlx::query_as::<_, JobRow>(
            "SELECT * FROM jobs WHERE id = $1 AND deleted_at IS NULL"
        )
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| map_sqlx_error(e, "Job"))?;

        match row {
            Some(r) => Ok(Some(r.try_into()?)),
            None => Ok(None),
        }
    }

    async fn find_next_pending(&self,
    ) -> Result<Option<Job>, ApiError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ApiError::Database(format!("Failed to begin transaction: {}", e)))?;

        let row = sqlx::query_as::<_, JobRow>(
            r#"
            SELECT * FROM jobs
            WHERE status = 'pending'
              AND deleted_at IS NULL
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
            FOR UPDATE SKIP LOCKED
            "#,
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| map_sqlx_error(e, "Job"))?;

        let job = match row {
            Some(r) => {
                let job: Job = r.try_into()?;
                sqlx::query(
                    "UPDATE jobs SET status = 'running', started_at = NOW(), attempts = attempts + 1, updated_at = NOW() WHERE id = $1",
                )
                .bind(job.id)
                .execute(&mut *tx)
                .await
                .map_err(|e| map_sqlx_error(e, "Job"))?;
                Some(job)
            }
            None => None,
        };

        tx.commit()
            .await
            .map_err(|e| ApiError::Database(format!("Failed to commit: {}", e)))?;

        Ok(job)
    }

    async fn mark_running(&self,
        id: i64,
    ) -> Result<(), ApiError> {
        let result = sqlx::query(
            "UPDATE jobs SET status = 'running', started_at = NOW(), attempts = attempts + 1, updated_at = NOW() WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Job"))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!("Job {} not found", id)));
        }
        Ok(())
    }

    async fn mark_completed(&self,
        id: i64,
    ) -> Result<(), ApiError> {
        let result = sqlx::query(
            "UPDATE jobs SET status = 'completed', completed_at = NOW(), updated_at = NOW() WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Job"))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!("Job {} not found", id)));
        }
        Ok(())
    }

    async fn mark_failed(
        &self,
        id: i64,
        error: &str,
    ) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE jobs
            SET
                status = CASE WHEN attempts >= max_attempts THEN 'failed' ELSE 'pending' END,
                completed_at = CASE WHEN attempts >= max_attempts THEN NOW() ELSE NULL END,
                scheduled_at = CASE WHEN attempts >= max_attempts THEN NULL ELSE NOW() + INTERVAL '1 second' * LEAST(POWER(2, attempts), 3600) END,
                last_error = $2,
                updated_at = NOW()
            WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(error)
        .execute(&self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Job"))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!("Job {} not found", id)));
        }
        Ok(())
    }

    async fn cancel(&self,
        id: i64,
    ) -> Result<(), ApiError> {
        let result = sqlx::query(
            "UPDATE jobs SET status = 'cancelled', completed_at = NOW(), updated_at = NOW() WHERE id = $1 AND status IN ('pending', 'scheduled') AND deleted_at IS NULL",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Job"))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::BadRequest(
                "Can only cancel pending or scheduled jobs".to_string(),
            ));
        }
        Ok(())
    }

    async fn list_by_status(
        &self,
        tenant_id: i64,
        status: JobStatus,
    ) -> Result<Vec<Job>, ApiError> {
        let rows = sqlx::query_as::<_, JobRow>(
            "SELECT * FROM jobs WHERE tenant_id = $1 AND status = $2 AND deleted_at IS NULL ORDER BY created_at DESC",
        )
        .bind(tenant_id)
        .bind(status.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Job"))?;

        rows.into_iter()
            .map(|r| r.try_into())
            .collect::<Result<Vec<_>, _>>()
    }

    async fn retry(&self,
        id: i64,
    ) -> Result<(), ApiError> {
        let result = sqlx::query(
            "UPDATE jobs SET status = 'pending', attempts = 0, last_error = NULL, scheduled_at = NULL, started_at = NULL, completed_at = NULL, updated_at = NOW() WHERE id = $1 AND status = 'failed' AND deleted_at IS NULL",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Job"))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::BadRequest("Can only retry failed jobs".to_string()));
        }
        Ok(())
    }

    async fn cleanup(
        &self,
        older_than: Duration,
    ) -> Result<u64, ApiError> {
        let secs = older_than.as_secs() as i64;
        let result = sqlx::query(
            "DELETE FROM jobs WHERE status IN ('completed', 'failed', 'cancelled') AND completed_at < NOW() - INTERVAL '1 second' * $1",
        )
        .bind(secs)
        .execute(&self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Job"))?;

        Ok(result.rows_affected())
    }

    async fn create_schedule(
        &self,
        schedule: CreateJobSchedule,
    ) -> Result<JobSchedule, ApiError> {
        let payload = serde_json::to_value(&schedule.job_type)
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        let row = sqlx::query_as::<_, JobScheduleRow>(
            r#"
            INSERT INTO job_schedules (job_type, payload, cron_expression, priority, tenant_id, max_attempts, is_active, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, true, NOW())
            RETURNING *
            "#,
        )
        .bind(schedule.job_type.type_name())
        .bind(sqlx::types::Json(payload))
        .bind(&schedule.cron_expression)
        .bind(schedule.priority.to_string())
        .bind(schedule.tenant_id)
        .bind(schedule.max_attempts as i32)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "JobSchedule"))?;

        row.try_into()
    }

    async fn list_schedules(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<JobSchedule>, ApiError> {
        let rows = sqlx::query_as::<_, JobScheduleRow>(
            "SELECT * FROM job_schedules WHERE tenant_id = $1 AND deleted_at IS NULL ORDER BY created_at DESC",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "JobSchedule"))?;

        rows.into_iter()
            .map(|r| r.try_into())
            .collect::<Result<Vec<_>, _>>()
    }

    async fn update_schedule_next_run(
        &self,
        id: i64,
        next_run: DateTime<Utc>,
        last_run: DateTime<Utc>,
    ) -> Result<(), ApiError> {
        let result = sqlx::query(
            "UPDATE job_schedules SET next_run_at = $1, last_run_at = $2, updated_at = NOW() WHERE id = $3 AND deleted_at IS NULL",
        )
        .bind(next_run)
        .bind(last_run)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "JobSchedule"))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!("Schedule {} not found", id)));
        }
        Ok(())
    }

    async fn toggle_schedule(
        &self,
        id: i64,
        active: bool,
    ) -> Result<(), ApiError> {
        let result = sqlx::query(
            "UPDATE job_schedules SET is_active = $1, updated_at = NOW() WHERE id = $2 AND deleted_at IS NULL",
        )
        .bind(active)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "JobSchedule"))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!("Schedule {} not found", id)));
        }
        Ok(())
    }

    async fn list_due_schedules(&self,
    ) -> Result<Vec<JobSchedule>, ApiError> {
        let rows = sqlx::query_as::<_, JobScheduleRow>(
            "SELECT * FROM job_schedules WHERE is_active = true AND deleted_at IS NULL AND (next_run_at IS NULL OR next_run_at <= NOW())",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "JobSchedule"))?;

        rows.into_iter()
            .map(|r| r.try_into())
            .collect::<Result<Vec<_>, _>>()
    }

    async fn count_by_status(
        &self,
        tenant_id: i64,
    ) -> Result<JobCounts, ApiError> {
        let rows: Vec<(String, i64)> = sqlx::query_as(
            "SELECT status, COUNT(*) FROM jobs WHERE tenant_id = $1 AND deleted_at IS NULL GROUP BY status",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Job"))?;

        let mut counts = JobCounts::default();
        for (status, count) in rows {
            match status.as_str() {
                "pending" => counts.pending = count,
                "running" => counts.running = count,
                "completed" => counts.completed = count,
                "failed" => counts.failed = count,
                "cancelled" => counts.cancelled = count,
                "scheduled" => counts.scheduled = count,
                _ => {}
            }
        }
        Ok(counts)
    }

    async fn list_recent(
        &self,
        tenant_id: i64,
        limit: i64,
    ) -> Result<Vec<Job>, ApiError> {
        let rows = sqlx::query_as::<_, JobRow>(
            "SELECT * FROM jobs WHERE tenant_id = $1 AND deleted_at IS NULL ORDER BY created_at DESC LIMIT $2",
        )
        .bind(tenant_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Job"))?;

        rows.into_iter()
            .map(|r| r.try_into())
            .collect::<Result<Vec<_>, _>>()
    }

    async fn reset_stalled(
        &self,
        timeout: Duration,
    ) -> Result<u64, ApiError> {
        let secs = timeout.as_secs() as i64;
        let result = sqlx::query(
            r#"
            UPDATE jobs
            SET status = 'pending', attempts = attempts + 1, started_at = NULL, updated_at = NOW()
            WHERE status = 'running' AND deleted_at IS NULL AND started_at < NOW() - INTERVAL '1 second' * $1
            "#,
        )
        .bind(secs)
        .execute(&self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Job"))?;

        Ok(result.rows_affected())
    }

    async fn soft_delete(&self, id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE jobs
            SET deleted_at = NOW(), deleted_by = $2, updated_at = NOW()
            WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(deleted_by)
        .execute(&self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Job"))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!("Job {} not found", id)));
        }
        Ok(())
    }

    async fn restore(&self, id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE jobs
            SET deleted_at = NULL, deleted_by = NULL, updated_at = NOW()
            WHERE id = $1 AND deleted_at IS NOT NULL
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Job"))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!(
                "Deleted job {} not found",
                id
            )));
        }
        Ok(())
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Job>, ApiError> {
        let rows = sqlx::query_as::<_, JobRow>(
            "SELECT * FROM jobs WHERE tenant_id = $1 AND deleted_at IS NOT NULL ORDER BY deleted_at DESC",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Job"))?;

        rows.into_iter()
            .map(|r| r.try_into())
            .collect::<Result<Vec<_>, _>>()
    }

    async fn destroy(&self, id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            "DELETE FROM jobs WHERE id = $1 AND deleted_at IS NOT NULL",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Job"))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!(
                "Deleted job {} not found",
                id
            )));
        }
        Ok(())
    }

    async fn soft_delete_schedule(&self, id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE job_schedules
            SET deleted_at = NOW(), deleted_by = $2, updated_at = NOW()
            WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(deleted_by)
        .execute(&self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "JobSchedule"))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!("Schedule {} not found", id)));
        }
        Ok(())
    }

    async fn restore_schedule(&self, id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE job_schedules
            SET deleted_at = NULL, deleted_by = NULL, updated_at = NOW()
            WHERE id = $1 AND deleted_at IS NOT NULL
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "JobSchedule"))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!(
                "Deleted schedule {} not found",
                id
            )));
        }
        Ok(())
    }

    async fn find_deleted_schedules(&self, tenant_id: i64) -> Result<Vec<JobSchedule>, ApiError> {
        let rows = sqlx::query_as::<_, JobScheduleRow>(
            "SELECT * FROM job_schedules WHERE tenant_id = $1 AND deleted_at IS NOT NULL ORDER BY deleted_at DESC",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "JobSchedule"))?;

        rows.into_iter()
            .map(|r| r.try_into())
            .collect::<Result<Vec<_>, _>>()
    }

    async fn destroy_schedule(&self, id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            "DELETE FROM job_schedules WHERE id = $1 AND deleted_at IS NOT NULL",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "JobSchedule"))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!(
                "Deleted schedule {} not found",
                id
            )));
        }
        Ok(())
    }
}
