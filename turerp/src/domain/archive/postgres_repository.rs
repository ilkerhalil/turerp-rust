//! PostgreSQL archive repository implementations

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::common::pagination::{PaginatedResult, PaginationParams};
use crate::db::error::map_sqlx_error;
use crate::domain::archive::model::{
    ArchiveJob, ArchiveJobStatus, ArchivePolicy, ArchiveRecord, CreateArchiveJob,
    CreateArchivePolicy, UpdateArchivePolicy,
};
use crate::domain::archive::repository::{
    ArchiveJobRepository, ArchivePolicyRepository, ArchiveRecordRepository,
    BoxArchiveJobRepository, BoxArchivePolicyRepository, BoxArchiveRecordRepository,
};
use crate::error::ApiError;

// ---------------------------------------------------------------------------
// ArchivePolicyRow / ArchivePolicy conversion
// ---------------------------------------------------------------------------

/// Database row representation for ArchivePolicy
#[derive(Debug, FromRow)]
struct ArchivePolicyRow {
    id: i64,
    tenant_id: i64,
    name: String,
    table_name: String,
    age_days: i32,
    conditions: Option<serde_json::Value>,
    is_active: bool,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
    total_count: Option<i64>,
}

impl From<ArchivePolicyRow> for ArchivePolicy {
    fn from(row: ArchivePolicyRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            name: row.name,
            table_name: row.table_name,
            age_days: row.age_days,
            conditions: row.conditions,
            is_active: row.is_active,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

// ---------------------------------------------------------------------------
// PostgresArchivePolicyRepository
// ---------------------------------------------------------------------------

/// PostgreSQL archive policy repository
pub struct PostgresArchivePolicyRepository {
    pool: Arc<PgPool>,
}

impl PostgresArchivePolicyRepository {
    /// Create a new PostgreSQL archive policy repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxArchivePolicyRepository {
        Arc::new(self) as BoxArchivePolicyRepository
    }
}

/// Common column list for archive_policies SELECT queries
macro_rules! archive_policy_columns {
    () => {
        r#"id, tenant_id, name, table_name, age_days, conditions,
    is_active, created_at, updated_at"#
    };
}

#[async_trait]
impl ArchivePolicyRepository for PostgresArchivePolicyRepository {
    async fn create(
        &self,
        create: CreateArchivePolicy,
        tenant_id: i64,
    ) -> Result<ArchivePolicy, ApiError> {
        let row: ArchivePolicyRow = sqlx::query_as(concat!(
            r#"
            INSERT INTO archive_policies (tenant_id, name, table_name, age_days, conditions, is_active)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING "#,
            archive_policy_columns!(),
            r#", 0 as total_count
            "#
        ))
        .bind(tenant_id)
        .bind(&create.name)
        .bind(&create.table_name)
        .bind(create.age_days)
        .bind(&create.conditions)
        .bind(create.is_active)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "ArchivePolicy"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<ArchivePolicy>, ApiError> {
        let result: Option<ArchivePolicyRow> = sqlx::query_as(concat!(
            r#"
            SELECT "#,
            archive_policy_columns!(),
            r#", 0 as total_count
            FROM archive_policies
            WHERE id = $1 AND tenant_id = $2
            "#
        ))
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find archive policy: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_all(
        &self,
        tenant_id: i64,
        params: PaginationParams,
    ) -> Result<PaginatedResult<ArchivePolicy>, ApiError> {
        let offset = params.offset() as i64;
        let per_page = params.per_page as i64;

        let rows: Vec<ArchivePolicyRow> = sqlx::query_as(concat!(
            r#"
            SELECT "#,
            archive_policy_columns!(),
            r#",
                   COUNT(*) OVER() as total_count
            FROM archive_policies
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#
        ))
        .bind(tenant_id)
        .bind(per_page)
        .bind(offset)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "ArchivePolicy"))?;

        let total = rows.first().and_then(|r| r.total_count).unwrap_or(0) as u64;
        let items: Vec<ArchivePolicy> = rows.into_iter().map(|r| r.into()).collect();
        Ok(PaginatedResult::new(
            items,
            params.page,
            params.per_page,
            total,
        ))
    }

    async fn find_active(&self, tenant_id: i64) -> Result<Vec<ArchivePolicy>, ApiError> {
        let rows: Vec<ArchivePolicyRow> = sqlx::query_as(concat!(
            r#"
            SELECT "#,
            archive_policy_columns!(),
            r#", 0 as total_count
            FROM archive_policies
            WHERE tenant_id = $1 AND is_active = true
            ORDER BY created_at DESC
            "#
        ))
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to find active archive policies: {}", e))
        })?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateArchivePolicy,
    ) -> Result<ArchivePolicy, ApiError> {
        let row: ArchivePolicyRow = sqlx::query_as(concat!(
            r#"
            UPDATE archive_policies
            SET
                name = COALESCE($1, name),
                table_name = COALESCE($2, table_name),
                age_days = COALESCE($3, age_days),
                conditions = COALESCE($4, conditions),
                is_active = COALESCE($5, is_active),
                updated_at = NOW()
            WHERE id = $6 AND tenant_id = $7
            RETURNING "#,
            archive_policy_columns!(),
            r#", 0 as total_count
            "#
        ))
        .bind(&update.name)
        .bind(&update.table_name)
        .bind(update.age_days)
        .bind(&update.conditions)
        .bind(update.is_active)
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "ArchivePolicy"))?;

        Ok(row.into())
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM archive_policies
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete archive policy: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Archive policy not found".to_string()));
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// ArchiveJobRow / ArchiveJob conversion
// ---------------------------------------------------------------------------

/// Database row representation for ArchiveJob
#[derive(Debug, FromRow)]
struct ArchiveJobRow {
    id: i64,
    tenant_id: i64,
    policy_id: i64,
    status: String,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    records_archived: i64,
    records_failed: i64,
    error_message: Option<String>,
    created_at: DateTime<Utc>,
    total_count: Option<i64>,
}

impl From<ArchiveJobRow> for ArchiveJob {
    fn from(row: ArchiveJobRow) -> Self {
        let status = row.status.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid ArchiveJobStatus '{}' in database: {}, defaulting to Pending",
                row.status,
                e
            );
            ArchiveJobStatus::Pending
        });

        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            policy_id: row.policy_id,
            status,
            started_at: row.started_at,
            completed_at: row.completed_at,
            records_archived: row.records_archived,
            records_failed: row.records_failed,
            error_message: row.error_message,
            created_at: row.created_at,
        }
    }
}

// ---------------------------------------------------------------------------
// PostgresArchiveJobRepository
// ---------------------------------------------------------------------------

/// PostgreSQL archive job repository
pub struct PostgresArchiveJobRepository {
    pool: Arc<PgPool>,
}

impl PostgresArchiveJobRepository {
    /// Create a new PostgreSQL archive job repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxArchiveJobRepository {
        Arc::new(self) as BoxArchiveJobRepository
    }
}

/// Common column list for archive_jobs SELECT queries
macro_rules! archive_job_columns {
    () => {
        r#"id, tenant_id, policy_id, status, started_at, completed_at,
    records_archived, records_failed, error_message, created_at"#
    };
}

#[async_trait]
impl ArchiveJobRepository for PostgresArchiveJobRepository {
    async fn create(
        &self,
        create: CreateArchiveJob,
        tenant_id: i64,
    ) -> Result<ArchiveJob, ApiError> {
        let row: ArchiveJobRow = sqlx::query_as(concat!(
            r#"
            INSERT INTO archive_jobs (tenant_id, policy_id, status, records_archived, records_failed)
            VALUES ($1, $2, 'Pending', 0, 0)
            RETURNING "#,
            archive_job_columns!(),
            r#", 0 as total_count
            "#
        ))
        .bind(tenant_id)
        .bind(create.policy_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "ArchiveJob"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<ArchiveJob>, ApiError> {
        let result: Option<ArchiveJobRow> = sqlx::query_as(concat!(
            r#"
            SELECT "#,
            archive_job_columns!(),
            r#", 0 as total_count
            FROM archive_jobs
            WHERE id = $1 AND tenant_id = $2
            "#
        ))
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find archive job: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_all(
        &self,
        tenant_id: i64,
        params: PaginationParams,
    ) -> Result<PaginatedResult<ArchiveJob>, ApiError> {
        let offset = params.offset() as i64;
        let per_page = params.per_page as i64;

        let rows: Vec<ArchiveJobRow> = sqlx::query_as(concat!(
            r#"
            SELECT "#,
            archive_job_columns!(),
            r#",
                   COUNT(*) OVER() as total_count
            FROM archive_jobs
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#
        ))
        .bind(tenant_id)
        .bind(per_page)
        .bind(offset)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "ArchiveJob"))?;

        let total = rows.first().and_then(|r| r.total_count).unwrap_or(0) as u64;
        let items: Vec<ArchiveJob> = rows.into_iter().map(|r| r.into()).collect();
        Ok(PaginatedResult::new(
            items,
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
        let offset = params.offset() as i64;
        let per_page = params.per_page as i64;

        let rows: Vec<ArchiveJobRow> = sqlx::query_as(concat!(
            r#"
            SELECT "#,
            archive_job_columns!(),
            r#",
                   COUNT(*) OVER() as total_count
            FROM archive_jobs
            WHERE tenant_id = $1 AND policy_id = $2
            ORDER BY created_at DESC
            LIMIT $3 OFFSET $4
            "#
        ))
        .bind(tenant_id)
        .bind(policy_id)
        .bind(per_page)
        .bind(offset)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "ArchiveJob"))?;

        let total = rows.first().and_then(|r| r.total_count).unwrap_or(0) as u64;
        let items: Vec<ArchiveJob> = rows.into_iter().map(|r| r.into()).collect();
        Ok(PaginatedResult::new(
            items,
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
        let status_str = status.to_string();

        let row: ArchiveJobRow = sqlx::query_as(concat!(
            r#"
            UPDATE archive_jobs
            SET status = $1,
                records_archived = $2,
                records_failed = $3,
                error_message = $4,
                completed_at = CASE WHEN $1 IN ('Completed', 'Failed') THEN NOW() ELSE completed_at END
            WHERE id = $5 AND tenant_id = $6
            RETURNING "#,
            archive_job_columns!(),
            r#", 0 as total_count
            "#
        ))
        .bind(&status_str)
        .bind(records_archived)
        .bind(records_failed)
        .bind(&error_message)
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "ArchiveJob"))?;

        Ok(row.into())
    }

    async fn start_job(&self, id: i64, tenant_id: i64) -> Result<ArchiveJob, ApiError> {
        let row: ArchiveJobRow = sqlx::query_as(concat!(
            r#"
            UPDATE archive_jobs
            SET status = 'Running', started_at = NOW()
            WHERE id = $1 AND tenant_id = $2 AND status = 'Pending'
            RETURNING "#,
            archive_job_columns!(),
            r#", 0 as total_count
            "#
        ))
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "ArchiveJob"))?;

        Ok(row.into())
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM archive_jobs
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete archive job: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Archive job not found".to_string()));
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// ArchiveRecordRow / ArchiveRecord conversion
// ---------------------------------------------------------------------------

/// Database row representation for ArchiveRecord
#[derive(Debug, FromRow)]
struct ArchiveRecordRow {
    id: i64,
    tenant_id: i64,
    source_table: String,
    source_id: i64,
    archived_data: serde_json::Value,
    archived_at: DateTime<Utc>,
    archive_job_id: i64,
    restored_at: Option<DateTime<Utc>>,
    total_count: Option<i64>,
}

impl From<ArchiveRecordRow> for ArchiveRecord {
    fn from(row: ArchiveRecordRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            source_table: row.source_table,
            source_id: row.source_id,
            archived_data: row.archived_data,
            archived_at: row.archived_at,
            archive_job_id: row.archive_job_id,
            restored_at: row.restored_at,
        }
    }
}

// ---------------------------------------------------------------------------
// PostgresArchiveRecordRepository
// ---------------------------------------------------------------------------

/// PostgreSQL archive record repository
pub struct PostgresArchiveRecordRepository {
    pool: Arc<PgPool>,
}

impl PostgresArchiveRecordRepository {
    /// Create a new PostgreSQL archive record repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxArchiveRecordRepository {
        Arc::new(self) as BoxArchiveRecordRepository
    }
}

/// Common column list for archive_records SELECT queries
macro_rules! archive_record_columns {
    () => {
        r#"id, tenant_id, source_table, source_id, archived_data,
    archived_at, archive_job_id, restored_at"#
    };
}

#[async_trait]
impl ArchiveRecordRepository for PostgresArchiveRecordRepository {
    async fn create(
        &self,
        tenant_id: i64,
        source_table: String,
        source_id: i64,
        archived_data: serde_json::Value,
        archive_job_id: i64,
    ) -> Result<ArchiveRecord, ApiError> {
        let row: ArchiveRecordRow = sqlx::query_as(concat!(
            r#"
            INSERT INTO archive_records (tenant_id, source_table, source_id, archived_data, archive_job_id)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING "#,
            archive_record_columns!(),
            r#", 0 as total_count
            "#
        ))
        .bind(tenant_id)
        .bind(&source_table)
        .bind(source_id)
        .bind(&archived_data)
        .bind(archive_job_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "ArchiveRecord"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<ArchiveRecord>, ApiError> {
        let result: Option<ArchiveRecordRow> = sqlx::query_as(concat!(
            r#"
            SELECT "#,
            archive_record_columns!(),
            r#", 0 as total_count
            FROM archive_records
            WHERE id = $1 AND tenant_id = $2
            "#
        ))
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find archive record: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_all(
        &self,
        tenant_id: i64,
        source_table: Option<String>,
        source_id: Option<i64>,
        params: PaginationParams,
    ) -> Result<PaginatedResult<ArchiveRecord>, ApiError> {
        let offset = params.offset() as i64;
        let per_page = params.per_page as i64;

        match (&source_table, source_id) {
            (Some(st), Some(sid)) => {
                let rows: Vec<ArchiveRecordRow> = sqlx::query_as(concat!(
                    r#"
                    SELECT "#,
                    archive_record_columns!(),
                    r#",
                           COUNT(*) OVER() as total_count
                    FROM archive_records
                    WHERE tenant_id = $1 AND source_table = $2 AND source_id = $3
                    ORDER BY archived_at DESC
                    LIMIT $4 OFFSET $5
                    "#
                ))
                .bind(tenant_id)
                .bind(st)
                .bind(sid)
                .bind(per_page)
                .bind(offset)
                .fetch_all(&*self.pool)
                .await
                .map_err(|e| map_sqlx_error(e, "ArchiveRecord"))?;

                let total = rows.first().and_then(|r| r.total_count).unwrap_or(0) as u64;
                let items: Vec<ArchiveRecord> = rows.into_iter().map(|r| r.into()).collect();
                Ok(PaginatedResult::new(
                    items,
                    params.page,
                    params.per_page,
                    total,
                ))
            }
            (Some(st), None) => {
                let rows: Vec<ArchiveRecordRow> = sqlx::query_as(concat!(
                    r#"
                    SELECT "#,
                    archive_record_columns!(),
                    r#",
                           COUNT(*) OVER() as total_count
                    FROM archive_records
                    WHERE tenant_id = $1 AND source_table = $2
                    ORDER BY archived_at DESC
                    LIMIT $3 OFFSET $4
                    "#
                ))
                .bind(tenant_id)
                .bind(st)
                .bind(per_page)
                .bind(offset)
                .fetch_all(&*self.pool)
                .await
                .map_err(|e| map_sqlx_error(e, "ArchiveRecord"))?;

                let total = rows.first().and_then(|r| r.total_count).unwrap_or(0) as u64;
                let items: Vec<ArchiveRecord> = rows.into_iter().map(|r| r.into()).collect();
                Ok(PaginatedResult::new(
                    items,
                    params.page,
                    params.per_page,
                    total,
                ))
            }
            (None, Some(sid)) => {
                let rows: Vec<ArchiveRecordRow> = sqlx::query_as(concat!(
                    r#"
                    SELECT "#,
                    archive_record_columns!(),
                    r#",
                           COUNT(*) OVER() as total_count
                    FROM archive_records
                    WHERE tenant_id = $1 AND source_id = $2
                    ORDER BY archived_at DESC
                    LIMIT $3 OFFSET $4
                    "#
                ))
                .bind(tenant_id)
                .bind(sid)
                .bind(per_page)
                .bind(offset)
                .fetch_all(&*self.pool)
                .await
                .map_err(|e| map_sqlx_error(e, "ArchiveRecord"))?;

                let total = rows.first().and_then(|r| r.total_count).unwrap_or(0) as u64;
                let items: Vec<ArchiveRecord> = rows.into_iter().map(|r| r.into()).collect();
                Ok(PaginatedResult::new(
                    items,
                    params.page,
                    params.per_page,
                    total,
                ))
            }
            (None, None) => {
                let rows: Vec<ArchiveRecordRow> = sqlx::query_as(concat!(
                    r#"
                    SELECT "#,
                    archive_record_columns!(),
                    r#",
                           COUNT(*) OVER() as total_count
                    FROM archive_records
                    WHERE tenant_id = $1
                    ORDER BY archived_at DESC
                    LIMIT $2 OFFSET $3
                    "#
                ))
                .bind(tenant_id)
                .bind(per_page)
                .bind(offset)
                .fetch_all(&*self.pool)
                .await
                .map_err(|e| map_sqlx_error(e, "ArchiveRecord"))?;

                let total = rows.first().and_then(|r| r.total_count).unwrap_or(0) as u64;
                let items: Vec<ArchiveRecord> = rows.into_iter().map(|r| r.into()).collect();
                Ok(PaginatedResult::new(
                    items,
                    params.page,
                    params.per_page,
                    total,
                ))
            }
        }
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<ArchiveRecord, ApiError> {
        let row: ArchiveRecordRow = sqlx::query_as(concat!(
            r#"
            UPDATE archive_records
            SET restored_at = NOW()
            WHERE id = $1 AND tenant_id = $2 AND restored_at IS NULL
            RETURNING "#,
            archive_record_columns!(),
            r#", 0 as total_count
            "#
        ))
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "ArchiveRecord"))?;

        Ok(row.into())
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM archive_records
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete archive record: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Archive record not found".to_string()));
        }

        Ok(())
    }
}
