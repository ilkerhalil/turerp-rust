//! PostgreSQL file metadata repository implementation

use async_trait::async_trait;
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::db::error::map_sqlx_error;
use crate::domain::file::model::{CreateFileRecord, FileRecord, UpdateFileRecord};
use crate::domain::file::repository::{BoxFileRepository, FileRepository};
use crate::error::ApiError;

/// Database row representation for File
#[derive(Debug, FromRow)]
struct FileRow {
    id: i64,
    tenant_id: i64,
    filename: String,
    original_filename: String,
    content_type: String,
    size_bytes: i64,
    storage_path: String,
    storage_backend: String,
    checksum: String,
    uploaded_by: Option<i64>,
    entity_type: Option<String>,
    entity_id: Option<i64>,
    created_at: chrono::DateTime<chrono::Utc>,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    deleted_by: Option<i64>,
}

impl From<FileRow> for FileRecord {
    fn from(row: FileRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            filename: row.filename,
            original_filename: row.original_filename,
            content_type: row.content_type,
            size_bytes: row.size_bytes,
            storage_path: row.storage_path,
            storage_backend: row.storage_backend,
            checksum: row.checksum,
            uploaded_by: row.uploaded_by,
            entity_type: row.entity_type,
            entity_id: row.entity_id,
            created_at: row.created_at,
            deleted_at: row.deleted_at,
            deleted_by: row.deleted_by,
        }
    }
}

/// PostgreSQL implementation of FileRepository
pub struct PostgresFileRepository {
    pool: Arc<PgPool>,
}

impl PostgresFileRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert into a boxed trait object
    pub fn into_boxed(self) -> BoxFileRepository {
        Arc::new(self)
    }
}

#[async_trait]
impl FileRepository for PostgresFileRepository {
    async fn create(&self, file: CreateFileRecord) -> Result<FileRecord, ApiError> {
        let row = sqlx::query_as::<_, FileRow>(
            r#"
            INSERT INTO files (
                tenant_id, filename, original_filename, content_type,
                size_bytes, storage_path, storage_backend, checksum,
                uploaded_by, entity_type, entity_id, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NOW())
            RETURNING *
            "#,
        )
        .bind(file.tenant_id)
        .bind(&file.filename)
        .bind(&file.original_filename)
        .bind(&file.content_type)
        .bind(file.size_bytes)
        .bind(&file.storage_path)
        .bind(&file.storage_backend)
        .bind(&file.checksum)
        .bind(file.uploaded_by)
        .bind(&file.entity_type)
        .bind(file.entity_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "File"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<FileRecord>, ApiError> {
        let row = sqlx::query_as::<_, FileRow>(
            r#"
            SELECT * FROM files
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "File"))?;

        Ok(row.map(Into::into))
    }

    async fn find_all(&self, tenant_id: i64) -> Result<Vec<FileRecord>, ApiError> {
        let rows = sqlx::query_as::<_, FileRow>(
            r#"
            SELECT * FROM files
            WHERE tenant_id = $1 AND deleted_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "File"))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateFileRecord,
    ) -> Result<FileRecord, ApiError> {
        let row = sqlx::query_as::<_, FileRow>(
            r#"
            UPDATE files
            SET filename = COALESCE($3, filename),
                original_filename = COALESCE($4, original_filename),
                content_type = COALESCE($5, content_type),
                entity_type = COALESCE($6, entity_type),
                entity_id = COALESCE($7, entity_id)
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(update.filename)
        .bind(update.original_filename)
        .bind(update.content_type)
        .bind(&update.entity_type)
        .bind(update.entity_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "File"))?;

        match row {
            Some(r) => Ok(r.into()),
            None => Err(ApiError::NotFound(format!("File {} not found", id))),
        }
    }

    async fn find_by_entity(
        &self,
        tenant_id: i64,
        entity_type: &str,
        entity_id: i64,
    ) -> Result<Vec<FileRecord>, ApiError> {
        let rows = sqlx::query_as::<_, FileRow>(
            r#"
            SELECT * FROM files
            WHERE tenant_id = $1
              AND deleted_at IS NULL
              AND entity_type = $2
              AND entity_id = $3
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .bind(entity_type)
        .bind(entity_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "File"))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE files
            SET deleted_at = NOW(), deleted_by = $3
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(deleted_by)
        .execute(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "File"))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!("File {} not found", id)));
        }

        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<FileRecord, ApiError> {
        let row = sqlx::query_as::<_, FileRow>(
            r#"
            UPDATE files
            SET deleted_at = NULL, deleted_by = NULL
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NOT NULL
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "File"))?;

        match row {
            Some(r) => Ok(r.into()),
            None => Err(ApiError::NotFound(format!("File {} not found", id))),
        }
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<FileRecord>, ApiError> {
        let rows = sqlx::query_as::<_, FileRow>(
            r#"
            SELECT * FROM files
            WHERE tenant_id = $1 AND deleted_at IS NOT NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "File"))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM files
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "File"))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!("File {} not found", id)));
        }

        Ok(())
    }

    async fn storage_used(&self, tenant_id: i64) -> Result<i64, ApiError> {
        let result: Option<(i64,)> = sqlx::query_as(
            r#"
            SELECT COALESCE(SUM(size_bytes), 0)
            FROM files
            WHERE tenant_id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "File"))?;

        Ok(result.map(|r| r.0).unwrap_or(0))
    }
}
