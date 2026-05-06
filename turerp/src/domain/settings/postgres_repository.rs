//! PostgreSQL repository for Settings

use async_trait::async_trait;
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::common::pagination::PaginatedResult;
use crate::db::error::map_sqlx_error;
use crate::domain::settings::model::{
    BulkUpdateSettingItem, CreateSetting, Setting, SettingDataType, SettingGroup, UpdateSetting,
};
use crate::domain::settings::repository::SettingsRepository;
use crate::error::ApiError;

/// Database row for settings
#[derive(Debug, FromRow)]
struct SettingRow {
    id: i64,
    tenant_id: i64,
    key: String,
    value: serde_json::Value,
    default_value: Option<serde_json::Value>,
    data_type: String,
    group_name: String,
    description: String,
    is_sensitive: bool,
    is_editable: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    deleted_by: Option<i64>,
}

impl From<SettingRow> for Setting {
    fn from(row: SettingRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            key: row.key,
            value: row.value,
            default_value: row.default_value,
            data_type: row.data_type.parse().unwrap_or_else(|_| {
                tracing::warn!("Invalid data_type in DB: {}", row.data_type);
                SettingDataType::String
            }),
            group: row.group_name.parse().unwrap_or_else(|_| {
                tracing::warn!("Invalid group in DB: {}", row.group_name);
                SettingGroup::General
            }),
            description: row.description,
            is_sensitive: row.is_sensitive,
            is_editable: row.is_editable,
            created_at: row.created_at,
            updated_at: row.updated_at,
            deleted_at: row.deleted_at,
            deleted_by: row.deleted_by,
        }
    }
}

/// PostgreSQL settings repository
pub struct PostgresSettingsRepository {
    pool: Arc<PgPool>,
}

impl PostgresSettingsRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub fn into_boxed(self) -> Arc<dyn SettingsRepository> {
        Arc::new(self) as Arc<dyn SettingsRepository>
    }
}

#[async_trait]
impl SettingsRepository for PostgresSettingsRepository {
    async fn create(&self, create: CreateSetting) -> Result<Setting, ApiError> {
        let row = sqlx::query_as::<_, SettingRow>(
            r#"
            INSERT INTO settings (
                tenant_id, key, value, default_value, data_type, group_name,
                description, is_sensitive, is_editable
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING
                id, tenant_id, key, value, default_value, data_type, group_name,
                description, is_sensitive, is_editable, created_at, updated_at,
                deleted_at, deleted_by
            "#,
        )
        .bind(create.tenant_id)
        .bind(&create.key)
        .bind(&create.value)
        .bind(&create.default_value)
        .bind(create.data_type.to_string())
        .bind(create.group.to_string())
        .bind(&create.description)
        .bind(create.is_sensitive)
        .bind(create.is_editable)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("duplicate key")
                || e.to_string().contains("unique constraint")
            {
                ApiError::Conflict(format!("Setting '{}' already exists", create.key))
            } else {
                map_sqlx_error(e, "Setting")
            }
        })?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Setting>, ApiError> {
        let row = sqlx::query_as::<_, SettingRow>(
            r#"
            SELECT id, tenant_id, key, value, default_value, data_type, group_name,
                description, is_sensitive, is_editable, created_at, updated_at,
                deleted_at, deleted_by
            FROM settings
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Setting"))?;

        Ok(row.map(Into::into))
    }

    async fn find_by_key(&self, tenant_id: i64, key: &str) -> Result<Option<Setting>, ApiError> {
        let row = sqlx::query_as::<_, SettingRow>(
            r#"
            SELECT id, tenant_id, key, value, default_value, data_type, group_name,
                description, is_sensitive, is_editable, created_at, updated_at,
                deleted_at, deleted_by
            FROM settings
            WHERE tenant_id = $1 AND key = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(tenant_id)
        .bind(key)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Setting"))?;

        Ok(row.map(Into::into))
    }

    async fn find_all(
        &self,
        tenant_id: i64,
        group: Option<&str>,
    ) -> Result<Vec<Setting>, ApiError> {
        let rows: Vec<SettingRow> = if let Some(group_name) = group {
            sqlx::query_as(
                r#"
                SELECT id, tenant_id, key, value, default_value, data_type, group_name,
                    description, is_sensitive, is_editable, created_at, updated_at,
                    deleted_at, deleted_by
                FROM settings
                WHERE tenant_id = $1 AND group_name = $2 AND deleted_at IS NULL
                ORDER BY key
                "#,
            )
            .bind(tenant_id)
            .bind(group_name)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| map_sqlx_error(e, "Setting"))?
        } else {
            sqlx::query_as(
                r#"
                SELECT id, tenant_id, key, value, default_value, data_type, group_name,
                    description, is_sensitive, is_editable, created_at, updated_at,
                    deleted_at, deleted_by
                FROM settings
                WHERE tenant_id = $1 AND deleted_at IS NULL
                ORDER BY key
                "#,
            )
            .bind(tenant_id)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| map_sqlx_error(e, "Setting"))?
        };

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn find_all_paginated(
        &self,
        tenant_id: i64,
        group: Option<&str>,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Setting>, ApiError> {
        let offset = ((page.saturating_sub(1)) * per_page) as i64;
        let limit = per_page as i64;

        let rows: Vec<SettingRow>;
        let total: i64;

        if let Some(group_name) = group {
            rows = sqlx::query_as(
                r#"
                SELECT id, tenant_id, key, value, default_value, data_type, group_name,
                    description, is_sensitive, is_editable, created_at, updated_at,
                    deleted_at, deleted_by
                FROM settings
                WHERE tenant_id = $1 AND group_name = $2 AND deleted_at IS NULL
                ORDER BY key
                LIMIT $3 OFFSET $4
                "#,
            )
            .bind(tenant_id)
            .bind(group_name)
            .bind(limit)
            .bind(offset)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| map_sqlx_error(e, "Setting"))?;

            total = sqlx::query_scalar(
                "SELECT COUNT(*) FROM settings WHERE tenant_id = $1 AND group_name = $2 AND deleted_at IS NULL",
            )
            .bind(tenant_id)
            .bind(group_name)
            .fetch_one(&*self.pool)
            .await
            .map_err(|e| map_sqlx_error(e, "Setting"))?;
        } else {
            rows = sqlx::query_as(
                r#"
                SELECT id, tenant_id, key, value, default_value, data_type, group_name,
                    description, is_sensitive, is_editable, created_at, updated_at,
                    deleted_at, deleted_by
                FROM settings
                WHERE tenant_id = $1 AND deleted_at IS NULL
                ORDER BY key
                LIMIT $2 OFFSET $3
                "#,
            )
            .bind(tenant_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| map_sqlx_error(e, "Setting"))?;

            total = sqlx::query_scalar(
                "SELECT COUNT(*) FROM settings WHERE tenant_id = $1 AND deleted_at IS NULL",
            )
            .bind(tenant_id)
            .fetch_one(&*self.pool)
            .await
            .map_err(|e| map_sqlx_error(e, "Setting"))?;
        }

        let items = rows.into_iter().map(Into::into).collect();
        Ok(PaginatedResult::new(items, page, per_page, total as u64))
    }

    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateSetting,
    ) -> Result<Setting, ApiError> {
        let existing = self
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Setting {} not found", id)))?;

        let value = update.value.unwrap_or(existing.value);
        let default_value = update.default_value.unwrap_or(existing.default_value);
        let description = update.description.unwrap_or(existing.description);
        let is_sensitive = update.is_sensitive.unwrap_or(existing.is_sensitive);
        let is_editable = update.is_editable.unwrap_or(existing.is_editable);

        let row = sqlx::query_as::<_, SettingRow>(
            r#"
            UPDATE settings
            SET value = $1, default_value = $2, description = $3,
                is_sensitive = $4, is_editable = $5, updated_at = NOW()
            WHERE id = $6 AND tenant_id = $7 AND deleted_at IS NULL
            RETURNING
                id, tenant_id, key, value, default_value, data_type, group_name,
                description, is_sensitive, is_editable, created_at, updated_at,
                deleted_at, deleted_by
            "#,
        )
        .bind(&value)
        .bind(&default_value)
        .bind(&description)
        .bind(is_sensitive)
        .bind(is_editable)
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Setting"))?;

        Ok(row.into())
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            "DELETE FROM settings WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL",
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Setting"))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!("Setting {} not found", id)));
        }

        Ok(())
    }

    async fn delete_by_key(&self, tenant_id: i64, key: &str) -> Result<(), ApiError> {
        sqlx::query(
            "DELETE FROM settings WHERE tenant_id = $1 AND key = $2 AND deleted_at IS NULL",
        )
        .bind(tenant_id)
        .bind(key)
        .execute(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Setting"))?;

        Ok(())
    }

    async fn bulk_update(
        &self,
        tenant_id: i64,
        updates: Vec<BulkUpdateSettingItem>,
    ) -> Result<Vec<Setting>, ApiError> {
        let mut updated = Vec::new();

        for item in updates {
            let result = sqlx::query_as::<_, SettingRow>(
                r#"
                UPDATE settings
                SET value = $1, updated_at = NOW()
                WHERE tenant_id = $2 AND key = $3 AND deleted_at IS NULL
                RETURNING
                    id, tenant_id, key, value, default_value, data_type, group_name,
                    description, is_sensitive, is_editable, created_at, updated_at,
                    deleted_at, deleted_by
                "#,
            )
            .bind(&item.value)
            .bind(tenant_id)
            .bind(&item.key)
            .fetch_optional(&*self.pool)
            .await
            .map_err(|e| map_sqlx_error(e, "Setting"))?;

            if let Some(row) = result {
                updated.push(row.into());
            }
        }

        Ok(updated)
    }

    async fn key_exists(&self, tenant_id: i64, key: &str) -> Result<bool, ApiError> {
        let count: i64 =
            sqlx::query_scalar(
                "SELECT COUNT(*) FROM settings WHERE tenant_id = $1 AND key = $2 AND deleted_at IS NULL"
            )
                .bind(tenant_id)
                .bind(key)
                .fetch_one(&*self.pool)
                .await
                .map_err(|e| map_sqlx_error(e, "Setting"))?;

        Ok(count > 0)
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE settings
            SET deleted_at = NOW(), deleted_by = $3, updated_at = NOW()
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(deleted_by)
        .execute(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Setting"))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!("Setting {} not found", id)));
        }

        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE settings
            SET deleted_at = NULL, deleted_by = NULL, updated_at = NOW()
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NOT NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Setting"))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!(
                "Deleted setting {} not found",
                id
            )));
        }

        Ok(())
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Setting>, ApiError> {
        let rows: Vec<SettingRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, key, value, default_value, data_type, group_name,
                description, is_sensitive, is_editable, created_at, updated_at,
                deleted_at, deleted_by
            FROM settings
            WHERE tenant_id = $1 AND deleted_at IS NOT NULL
            ORDER BY deleted_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Setting"))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            "DELETE FROM settings WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NOT NULL",
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Setting"))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!(
                "Deleted setting {} not found",
                id
            )));
        }

        Ok(())
    }
}
