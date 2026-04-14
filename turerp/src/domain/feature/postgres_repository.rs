//! PostgreSQL feature flag repository implementation

use async_trait::async_trait;
use chrono::NaiveDateTime;
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::common::pagination::PaginatedResult;
use crate::db::error::map_sqlx_error;
use crate::domain::feature::model::{
    CreateFeatureFlag, FeatureFlag, FeatureFlagStatus, UpdateFeatureFlag,
};
use crate::domain::feature::repository::FeatureFlagRepository;
use crate::error::ApiError;

/// Convert sqlx errors to ApiError with proper detection of error types

/// Parse a feature flag status string from the database
fn parse_status(s: &str) -> FeatureFlagStatus {
    match s {
        "enabled" => FeatureFlagStatus::Enabled,
        _ => {
            tracing::warn!(
                "Invalid feature flag status '{}' in database, defaulting to Disabled",
                s
            );
            FeatureFlagStatus::Disabled
        }
    }
}

/// Database row representation for FeatureFlag
#[derive(Debug, FromRow)]
struct FeatureFlagRow {
    id: i64,
    name: String,
    description: Option<String>,
    status: String,
    tenant_id: Option<i64>,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
    total_count: Option<i64>,
}

impl From<FeatureFlagRow> for FeatureFlag {
    fn from(row: FeatureFlagRow) -> Self {
        let status = parse_status(&row.status);

        Self {
            id: row.id,
            name: row.name,
            description: row.description,
            status,
            tenant_id: row.tenant_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// PostgreSQL feature flag repository
pub struct PostgresFeatureFlagRepository {
    pool: Arc<PgPool>,
}

impl PostgresFeatureFlagRepository {
    /// Create a new PostgreSQL feature flag repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> Arc<dyn FeatureFlagRepository> {
        Arc::new(self) as Arc<dyn FeatureFlagRepository>
    }
}

#[async_trait]
impl FeatureFlagRepository for PostgresFeatureFlagRepository {
    async fn create(&self, flag: CreateFeatureFlag) -> Result<FeatureFlag, ApiError> {
        let status = flag
            .status
            .unwrap_or(FeatureFlagStatus::Disabled)
            .to_string();

        let row: FeatureFlagRow = sqlx::query_as(
            r#"
            INSERT INTO feature_flags (name, description, status, tenant_id, created_at, updated_at)
            VALUES ($1, $2, $3, $4, NOW(), NOW())
            RETURNING id, name, description, status, tenant_id, created_at, updated_at
            "#,
        )
        .bind(&flag.name)
        .bind(&flag.description)
        .bind(&status)
        .bind(flag.tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Feature flag"))?;

        Ok(row.into())
    }

    async fn get_by_id(&self, id: i64) -> Result<Option<FeatureFlag>, ApiError> {
        let result: Option<FeatureFlagRow> = sqlx::query_as(
            r#"
            SELECT id, name, description, status, tenant_id, created_at, updated_at
            FROM feature_flags
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to get feature flag by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn get_by_name(
        &self,
        name: &str,
        tenant_id: Option<i64>,
    ) -> Result<Option<FeatureFlag>, ApiError> {
        let result = match tenant_id {
            Some(tid) => {
                let row: Option<FeatureFlagRow> = sqlx::query_as(
                    r#"
                    SELECT id, name, description, status, tenant_id, created_at, updated_at
                    FROM feature_flags
                    WHERE name = $1 AND tenant_id = $2
                    "#,
                )
                .bind(name)
                .bind(tid)
                .fetch_optional(&*self.pool)
                .await
                .map_err(|e| {
                    ApiError::Database(format!("Failed to get feature flag by name: {}", e))
                })?;

                row
            }
            None => {
                let row: Option<FeatureFlagRow> = sqlx::query_as(
                    r#"
                    SELECT id, name, description, status, tenant_id, created_at, updated_at
                    FROM feature_flags
                    WHERE name = $1 AND tenant_id IS NULL
                    "#,
                )
                .bind(name)
                .fetch_optional(&*self.pool)
                .await
                .map_err(|e| {
                    ApiError::Database(format!("Failed to get feature flag by name: {}", e))
                })?;

                row
            }
        };

        Ok(result.map(|r| r.into()))
    }

    async fn get_all(&self, tenant_id: Option<i64>) -> Result<Vec<FeatureFlag>, ApiError> {
        let rows = match tenant_id {
            Some(tid) => {
                let rows: Vec<FeatureFlagRow> = sqlx::query_as(
                    r#"
                    SELECT id, name, description, status, tenant_id, created_at, updated_at
                    FROM feature_flags
                    WHERE tenant_id = $1 OR tenant_id IS NULL
                    ORDER BY created_at DESC
                    "#,
                )
                .bind(tid)
                .fetch_all(&*self.pool)
                .await
                .map_err(|e| {
                    ApiError::Database(format!("Failed to get all feature flags: {}", e))
                })?;

                rows
            }
            None => {
                let rows: Vec<FeatureFlagRow> = sqlx::query_as(
                    r#"
                    SELECT id, name, description, status, tenant_id, created_at, updated_at
                    FROM feature_flags
                    WHERE tenant_id IS NULL
                    ORDER BY created_at DESC
                    "#,
                )
                .fetch_all(&*self.pool)
                .await
                .map_err(|e| {
                    ApiError::Database(format!("Failed to get all feature flags: {}", e))
                })?;

                rows
            }
        };

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn get_all_paginated(
        &self,
        tenant_id: Option<i64>,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<FeatureFlag>, ApiError> {
        let offset = (page.saturating_sub(1)) * per_page;

        match tenant_id {
            Some(tid) => {
                let rows: Vec<FeatureFlagRow> = sqlx::query_as(
                    r#"
                    SELECT id, name, description, status, tenant_id, created_at, updated_at,
                           COUNT(*) OVER() as total_count
                    FROM feature_flags
                    WHERE tenant_id = $1 OR tenant_id IS NULL
                    ORDER BY id DESC
                    LIMIT $2 OFFSET $3
                    "#,
                )
                .bind(tid)
                .bind(per_page as i64)
                .bind(offset as i64)
                .fetch_all(&*self.pool)
                .await
                .map_err(|e| {
                    ApiError::Database(format!("Failed to get paginated feature flags: {}", e))
                })?;

                let total = rows.first().and_then(|r| r.total_count).unwrap_or(0) as u64;
                let items: Vec<FeatureFlag> = rows.into_iter().map(|r| r.into()).collect();
                Ok(PaginatedResult::new(items, page, per_page, total))
            }
            None => {
                let rows: Vec<FeatureFlagRow> = sqlx::query_as(
                    r#"
                    SELECT id, name, description, status, tenant_id, created_at, updated_at,
                           COUNT(*) OVER() as total_count
                    FROM feature_flags
                    WHERE tenant_id IS NULL
                    ORDER BY id DESC
                    LIMIT $1 OFFSET $2
                    "#,
                )
                .bind(per_page as i64)
                .bind(offset as i64)
                .fetch_all(&*self.pool)
                .await
                .map_err(|e| {
                    ApiError::Database(format!("Failed to get paginated feature flags: {}", e))
                })?;

                let total = rows.first().and_then(|r| r.total_count).unwrap_or(0) as u64;
                let items: Vec<FeatureFlag> = rows.into_iter().map(|r| r.into()).collect();
                Ok(PaginatedResult::new(items, page, per_page, total))
            }
        }
    }

    async fn update(
        &self,
        id: i64,
        flag: UpdateFeatureFlag,
    ) -> Result<Option<FeatureFlag>, ApiError> {
        let status_str = flag.status.map(|s| s.to_string());

        let result: Option<FeatureFlagRow> = sqlx::query_as(
            r#"
            UPDATE feature_flags
            SET
                description = COALESCE($1, description),
                status = COALESCE($2, status),
                updated_at = NOW()
            WHERE id = $3
            RETURNING id, name, description, status, tenant_id, created_at, updated_at
            "#,
        )
        .bind(&flag.description)
        .bind(&status_str)
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Feature flag"))?;

        Ok(result.map(|r| r.into()))
    }

    async fn delete(&self, id: i64) -> Result<bool, ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM feature_flags
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete feature flag: {}", e)))?;

        Ok(result.rows_affected() > 0)
    }

    async fn is_enabled(&self, name: &str, tenant_id: Option<i64>) -> Result<bool, ApiError> {
        // First check for tenant-specific flag
        if let Some(tid) = tenant_id {
            let row: Option<FeatureFlagRow> = sqlx::query_as(
                r#"
                SELECT id, name, description, status, tenant_id, created_at, updated_at
                FROM feature_flags
                WHERE name = $1 AND tenant_id = $2
                "#,
            )
            .bind(name)
            .bind(tid)
            .fetch_optional(&*self.pool)
            .await
            .map_err(|e| {
                ApiError::Database(format!("Failed to check feature flag status: {}", e))
            })?;

            if let Some(r) = row {
                return Ok(parse_status(&r.status) == FeatureFlagStatus::Enabled);
            }
        }

        // Fall back to global flag
        let row: Option<FeatureFlagRow> = sqlx::query_as(
            r#"
            SELECT id, name, description, status, tenant_id, created_at, updated_at
            FROM feature_flags
            WHERE name = $1 AND tenant_id IS NULL
            "#,
        )
        .bind(name)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to check feature flag status: {}", e)))?;

        match row {
            Some(r) => Ok(parse_status(&r.status) == FeatureFlagStatus::Enabled),
            None => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_status_enabled() {
        assert_eq!(parse_status("enabled"), FeatureFlagStatus::Enabled);
    }

    #[test]
    fn test_parse_status_disabled() {
        assert_eq!(parse_status("disabled"), FeatureFlagStatus::Disabled);
    }

    #[test]
    fn test_parse_status_invalid_defaults_to_disabled() {
        assert_eq!(parse_status("unknown_value"), FeatureFlagStatus::Disabled);
    }

    #[test]
    fn test_feature_flag_row_conversion() {
        let now = chrono::Utc::now().naive_utc();
        let row = FeatureFlagRow {
            id: 1,
            name: "test_feature".to_string(),
            description: Some("A test feature".to_string()),
            status: "enabled".to_string(),
            tenant_id: None,
            created_at: now,
            updated_at: now,
        };

        let flag: FeatureFlag = row.into();
        assert_eq!(flag.id, 1);
        assert_eq!(flag.name, "test_feature");
        assert_eq!(flag.status, FeatureFlagStatus::Enabled);
        assert!(flag.tenant_id.is_none());
    }

    #[test]
    fn test_feature_flag_row_conversion_with_tenant() {
        let now = chrono::Utc::now().naive_utc();
        let row = FeatureFlagRow {
            id: 2,
            name: "tenant_feature".to_string(),
            description: None,
            status: "disabled".to_string(),
            tenant_id: Some(42),
            created_at: now,
            updated_at: now,
        };

        let flag: FeatureFlag = row.into();
        assert_eq!(flag.id, 2);
        assert_eq!(flag.status, FeatureFlagStatus::Disabled);
        assert_eq!(flag.tenant_id, Some(42));
    }
}
