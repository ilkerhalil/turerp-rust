//! PostgreSQL API key repository implementation

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::common::pagination::PaginatedResult;
use crate::db::error::map_sqlx_error;
use crate::domain::api_key::model::{ApiKey, ApiKeyScope};
use crate::domain::api_key::repository::ApiKeyRepository;
use crate::error::ApiError;

/// Parse scope strings into ApiKeyScope enum
fn parse_scopes(scopes: Vec<String>) -> Vec<ApiKeyScope> {
    scopes.into_iter().filter_map(|s| s.parse().ok()).collect()
}

/// Serialize scopes to strings for storage
fn scopes_to_strings(scopes: &[ApiKeyScope]) -> Vec<String> {
    scopes.iter().map(|s| s.to_string()).collect()
}

/// Database row representation for ApiKey
#[derive(Debug, FromRow)]
struct ApiKeyRow {
    id: i64,
    tenant_id: i64,
    user_id: i64,
    name: String,
    key_hash: String,
    key_prefix: String,
    scopes: Vec<String>,
    is_active: bool,
    expires_at: Option<DateTime<Utc>>,
    last_used_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
    deleted_at: Option<DateTime<Utc>>,
    deleted_by: Option<i64>,
    total_count: Option<i64>,
}

impl From<ApiKeyRow> for ApiKey {
    fn from(row: ApiKeyRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            key_hash: row.key_hash,
            key_prefix: row.key_prefix,
            tenant_id: row.tenant_id,
            user_id: row.user_id,
            scopes: parse_scopes(row.scopes),
            is_active: row.is_active,
            expires_at: row.expires_at,
            last_used_at: row.last_used_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
            deleted_at: row.deleted_at,
            deleted_by: row.deleted_by,
        }
    }
}

/// PostgreSQL API key repository
pub struct PostgresApiKeyRepository {
    pool: Arc<PgPool>,
}

impl PostgresApiKeyRepository {
    /// Create a new PostgreSQL API key repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> Arc<dyn ApiKeyRepository> {
        Arc::new(self) as Arc<dyn ApiKeyRepository>
    }
}

#[async_trait]
impl ApiKeyRepository for PostgresApiKeyRepository {
    async fn create(
        &self,
        name: String,
        key_hash: String,
        key_prefix: String,
        tenant_id: i64,
        user_id: i64,
        scopes: Vec<ApiKeyScope>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<ApiKey, ApiError> {
        let scopes_str = scopes_to_strings(&scopes);

        let row: ApiKeyRow = sqlx::query_as(
            r#"
            INSERT INTO api_keys (tenant_id, user_id, name, key_hash, key_prefix, scopes, is_active, expires_at, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, true, $7, NOW())
            RETURNING id, tenant_id, user_id, name, key_hash, key_prefix, scopes, is_active, expires_at, last_used_at, created_at, updated_at, deleted_at, deleted_by
            "#,
        )
        .bind(tenant_id)
        .bind(user_id)
        .bind(&name)
        .bind(&key_hash)
        .bind(&key_prefix)
        .bind(&scopes_str)
        .bind(expires_at)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "API key"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<ApiKey>, ApiError> {
        let result: Option<ApiKeyRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, user_id, name, key_hash, key_prefix, scopes, is_active, expires_at, last_used_at, created_at, updated_at, deleted_at, deleted_by
            FROM api_keys
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to get API key by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_key_hash(&self, key_hash: &str) -> Result<Option<ApiKey>, ApiError> {
        let result: Option<ApiKeyRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, user_id, name, key_hash, key_prefix, scopes, is_active, expires_at, last_used_at, created_at, updated_at, deleted_at, deleted_by
            FROM api_keys
            WHERE key_hash = $1 AND is_active = true AND deleted_at IS NULL
            "#,
        )
        .bind(key_hash)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to get API key by hash: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<ApiKey>, ApiError> {
        let rows: Vec<ApiKeyRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, user_id, name, key_hash, key_prefix, scopes, is_active, expires_at, last_used_at, created_at, updated_at, deleted_at, deleted_by
            FROM api_keys
            WHERE tenant_id = $1 AND deleted_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to get API keys by tenant: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<ApiKey>, ApiError> {
        let offset = (page.saturating_sub(1)) * per_page;

        let rows: Vec<ApiKeyRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, user_id, name, key_hash, key_prefix, scopes, is_active, expires_at, last_used_at, created_at, updated_at, deleted_at, deleted_by,
                   COUNT(*) OVER() as total_count
            FROM api_keys
            WHERE tenant_id = $1 AND deleted_at IS NULL
            ORDER BY id DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(tenant_id)
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to get paginated API keys: {}", e)))?;

        let total = rows.first().and_then(|r| r.total_count).unwrap_or(0) as u64;
        let items: Vec<ApiKey> = rows.into_iter().map(|r| r.into()).collect();
        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        name: Option<String>,
        scopes: Option<Vec<ApiKeyScope>>,
        is_active: Option<bool>,
        expires_at: Option<Option<DateTime<Utc>>>,
    ) -> Result<ApiKey, ApiError> {
        let scopes_str = scopes.as_ref().map(|s| scopes_to_strings(s));

        let result: Option<ApiKeyRow> = sqlx::query_as(
            r#"
            UPDATE api_keys
            SET
                name = COALESCE($1, name),
                scopes = COALESCE($2, scopes),
                is_active = COALESCE($3, is_active),
                expires_at = COALESCE($4, expires_at),
                updated_at = NOW()
            WHERE id = $5 AND tenant_id = $6 AND deleted_at IS NULL
            RETURNING id, tenant_id, user_id, name, key_hash, key_prefix, scopes, is_active, expires_at, last_used_at, created_at, updated_at, deleted_at, deleted_by
            "#,
        )
        .bind(&name)
        .bind(&scopes_str)
        .bind(is_active)
        .bind(expires_at)
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "API key"))?;

        result
            .map(|r| r.into())
            .ok_or_else(|| ApiError::NotFound(format!("API key {} not found", id)))
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM api_keys
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete API key: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!("API key {} not found", id)));
        }

        Ok(())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE api_keys
            SET deleted_at = NOW(), deleted_by = $3, updated_at = NOW()
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(deleted_by)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to soft delete API key: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!("API key {} not found", id)));
        }

        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE api_keys
            SET deleted_at = NULL, deleted_by = NULL, updated_at = NOW()
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NOT NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to restore API key: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!(
                "Deleted API key {} not found",
                id
            )));
        }

        Ok(())
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<ApiKey>, ApiError> {
        let rows: Vec<ApiKeyRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, user_id, name, key_hash, key_prefix, scopes, is_active, expires_at, last_used_at, created_at, updated_at, deleted_at, deleted_by
            FROM api_keys
            WHERE tenant_id = $1 AND deleted_at IS NOT NULL
            ORDER BY deleted_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to get deleted API keys: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM api_keys
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NOT NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to destroy API key: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!(
                "Deleted API key {} not found",
                id
            )));
        }

        Ok(())
    }

    async fn touch_last_used(&self, id: i64) -> Result<(), ApiError> {
        sqlx::query(
            r#"
            UPDATE api_keys
            SET last_used_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to update API key last used: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_scopes() {
        let raw = vec![
            "all".to_string(),
            "cari:read".to_string(),
            "invalid".to_string(),
        ];
        let scopes = parse_scopes(raw);
        assert_eq!(scopes.len(), 2);
        assert!(scopes.contains(&ApiKeyScope::All));
        assert!(scopes.contains(&ApiKeyScope::CariRead));
    }

    #[test]
    fn test_scopes_to_strings() {
        let scopes = vec![ApiKeyScope::All, ApiKeyScope::InvoiceWrite];
        let strings = scopes_to_strings(&scopes);
        assert_eq!(strings, vec!["all", "invoice:write"]);
    }
}
