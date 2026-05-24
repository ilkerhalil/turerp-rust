//! PostgreSQL tenant repository implementation
use async_trait::async_trait;
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::common::pagination::PaginatedResult;
use crate::db::error::map_sqlx_error;
use crate::domain::tenant::model::{
    CreateTenant, CreateTenantConfig, Tenant, TenantConfig, UpdateTenant, UpdateTenantConfig,
};
use crate::domain::tenant::repository::{
    BoxTenantConfigRepository, BoxTenantRepository, TenantConfigRepository, TenantRepository,
};
use crate::error::ApiError;

// Convert sqlx errors to ApiError with proper detection of error types

/// Database row representation for Tenant
#[derive(Debug, FromRow)]
struct TenantRow {
    id: i64,
    name: String,
    subdomain: String,
    is_active: bool,
    base_currency: String,
    supported_currencies: Vec<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: Option<chrono::DateTime<chrono::Utc>>,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    deleted_by: Option<i64>,
}

impl From<TenantRow> for Tenant {
    fn from(row: TenantRow) -> Self {
        let db_name = crate::domain::tenant::model::generate_db_name(&row.subdomain);
        Self {
            id: row.id,
            name: row.name,
            subdomain: row.subdomain,
            db_name,
            is_active: row.is_active,
            base_currency: row.base_currency,
            supported_currencies: row.supported_currencies,
            created_at: row.created_at,
            updated_at: row.updated_at,
            deleted_at: row.deleted_at,
            deleted_by: row.deleted_by,
        }
    }
}

/// Database row representation for paginated tenant queries with total count
#[derive(Debug, FromRow)]
struct TenantRowWithTotal {
    id: i64,
    name: String,
    subdomain: String,
    is_active: bool,
    base_currency: String,
    supported_currencies: Vec<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: Option<chrono::DateTime<chrono::Utc>>,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    deleted_by: Option<i64>,
    total_count: i64,
}

impl From<TenantRowWithTotal> for (Tenant, i64) {
    fn from(row: TenantRowWithTotal) -> (Tenant, i64) {
        let db_name = crate::domain::tenant::model::generate_db_name(&row.subdomain);
        let tenant = Tenant {
            id: row.id,
            name: row.name,
            subdomain: row.subdomain,
            db_name,
            is_active: row.is_active,
            base_currency: row.base_currency,
            supported_currencies: row.supported_currencies,
            created_at: row.created_at,
            updated_at: row.updated_at,
            deleted_at: row.deleted_at,
            deleted_by: row.deleted_by,
        };
        (tenant, row.total_count)
    }
}

/// PostgreSQL tenant repository
pub struct PostgresTenantRepository {
    pool: Arc<PgPool>,
}

impl PostgresTenantRepository {
    /// Create a new PostgreSQL tenant repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxTenantRepository {
        Arc::new(self) as BoxTenantRepository
    }
}

#[async_trait]
impl TenantRepository for PostgresTenantRepository {
    async fn create(&self, create: CreateTenant) -> Result<Tenant, ApiError> {
        let row: TenantRow = sqlx::query_as(
            r#"
            INSERT INTO tenants (name, subdomain, is_active, created_at)
            VALUES ($1, $2, true, NOW())
            RETURNING id, name, subdomain, is_active, base_currency, supported_currencies, created_at, updated_at, deleted_at, deleted_by
            "#,
        )
        .bind(&create.name)
        .bind(&create.subdomain)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Tenant"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Tenant>, ApiError> {
        let result: Option<TenantRow> = sqlx::query_as(
            r#"
            SELECT id, name, subdomain, is_active, base_currency, supported_currencies, created_at, updated_at, deleted_at, deleted_by
            FROM tenants
            WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find tenant by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_subdomain(&self, subdomain: &str) -> Result<Option<Tenant>, ApiError> {
        let result: Option<TenantRow> = sqlx::query_as(
            r#"
            SELECT id, name, subdomain, is_active, base_currency, supported_currencies, created_at, updated_at, deleted_at, deleted_by
            FROM tenants
            WHERE subdomain = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(subdomain)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find tenant by subdomain: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_all(&self) -> Result<Vec<Tenant>, ApiError> {
        let rows: Vec<TenantRow> = sqlx::query_as(
            r#"
            SELECT id, name, subdomain, is_active, base_currency, supported_currencies, created_at, updated_at, deleted_at, deleted_by
            FROM tenants
            WHERE deleted_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find all tenants: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_all_paginated(
        &self,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Tenant>, ApiError> {
        let offset = page.saturating_sub(1) * per_page;

        let rows: Vec<TenantRowWithTotal> = sqlx::query_as(
            r#"
            SELECT id, name, subdomain, is_active, base_currency, supported_currencies, created_at, updated_at, deleted_at, deleted_by,
                   COUNT(*) OVER() as total_count
            FROM tenants
            WHERE deleted_at IS NULL
            ORDER BY id DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Tenant"))?;

        let total = rows.first().map(|r| r.total_count as u64).unwrap_or(0);
        let items: Vec<Tenant> = rows
            .into_iter()
            .map(|r| r.into())
            .map(|(tenant, _)| tenant)
            .collect();
        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn update(&self, id: i64, update: UpdateTenant) -> Result<Tenant, ApiError> {
        let row: TenantRow = sqlx::query_as(
            r#"
            UPDATE tenants
            SET
                name = COALESCE($1, name),
                subdomain = COALESCE($2, subdomain),
                is_active = COALESCE($3, is_active),
                updated_at = NOW()
            WHERE id = $4 AND deleted_at IS NULL
            RETURNING id, name, subdomain, is_active, base_currency, supported_currencies, created_at, updated_at, deleted_at, deleted_by
            "#,
        )
        .bind(&update.name)
        .bind(&update.subdomain)
        .bind(update.is_active)
        .bind(id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Tenant"))?;

        Ok(row.into())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE tenants
            SET deleted_at = NOW(), updated_at = NOW()
            WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete tenant: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Tenant not found".to_string()));
        }

        Ok(())
    }

    async fn soft_delete(&self, id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE tenants
            SET deleted_at = NOW(), deleted_by = $2, updated_at = NOW()
            WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(deleted_by)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to soft delete tenant: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Tenant not found".to_string()));
        }

        Ok(())
    }

    async fn restore(&self, id: i64) -> Result<Tenant, ApiError> {
        let row: TenantRow = sqlx::query_as(
            r#"
            UPDATE tenants
            SET deleted_at = NULL, deleted_by = NULL, updated_at = NOW()
            WHERE id = $1 AND deleted_at IS NOT NULL
            RETURNING id, name, subdomain, is_active, base_currency, supported_currencies, created_at, updated_at, deleted_at, deleted_by
            "#,
        )
        .bind(id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Tenant"))?;

        Ok(row.into())
    }

    async fn find_deleted(&self) -> Result<Vec<Tenant>, ApiError> {
        let rows: Vec<TenantRow> = sqlx::query_as(
            r#"
            SELECT id, name, subdomain, is_active, base_currency, supported_currencies, created_at, updated_at, deleted_at, deleted_by
            FROM tenants
            WHERE deleted_at IS NOT NULL
            ORDER BY deleted_at DESC
            "#,
        )
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find deleted tenants: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn destroy(&self, id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM tenants
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to destroy tenant: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Tenant not found".to_string()));
        }

        Ok(())
    }

    async fn subdomain_exists(&self, subdomain: &str) -> Result<bool, ApiError> {
        let result: (bool,) = sqlx::query_as(
            r#"
            SELECT EXISTS(SELECT 1 FROM tenants WHERE subdomain = $1 AND deleted_at IS NULL)
            "#,
        )
        .bind(subdomain)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to check subdomain: {}", e)))?;

        Ok(result.0)
    }
}

/// Database row representation for TenantConfig
#[derive(Debug, FromRow)]
struct TenantConfigRow {
    id: i64,
    tenant_id: i64,
    key: String,
    value: serde_json::Value,
    is_encrypted: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<TenantConfigRow> for TenantConfig {
    fn from(row: TenantConfigRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            key: row.key,
            value: row.value,
            is_encrypted: row.is_encrypted,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// PostgreSQL tenant config repository
pub struct PostgresTenantConfigRepository {
    pool: Arc<PgPool>,
}

impl PostgresTenantConfigRepository {
    /// Create a new PostgreSQL tenant config repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxTenantConfigRepository {
        Arc::new(self) as BoxTenantConfigRepository
    }
}

#[async_trait]
impl TenantConfigRepository for PostgresTenantConfigRepository {
    async fn set(&self, create: CreateTenantConfig) -> Result<TenantConfig, ApiError> {
        let row: TenantConfigRow = sqlx::query_as(
            r#"
            INSERT INTO tenant_configs (tenant_id, key, value, is_encrypted, created_at, updated_at)
            VALUES ($1, $2, $3, $4, NOW(), NOW())
            ON CONFLICT (tenant_id, key) DO UPDATE SET
                value = EXCLUDED.value,
                is_encrypted = EXCLUDED.is_encrypted,
                updated_at = NOW()
            RETURNING id, tenant_id, key, value, is_encrypted, created_at, updated_at
            "#,
        )
        .bind(create.tenant_id)
        .bind(&create.key)
        .bind(&create.value)
        .bind(create.is_encrypted.unwrap_or(false))
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "TenantConfig"))?;

        Ok(row.into())
    }

    async fn get(&self, tenant_id: i64, key: &str) -> Result<Option<TenantConfig>, ApiError> {
        let result: Option<TenantConfigRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, key, value, is_encrypted, created_at, updated_at
            FROM tenant_configs
            WHERE tenant_id = $1 AND key = $2
            "#,
        )
        .bind(tenant_id)
        .bind(key)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to get tenant config: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn get_all(&self, tenant_id: i64) -> Result<Vec<TenantConfig>, ApiError> {
        let rows: Vec<TenantConfigRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, key, value, is_encrypted, created_at, updated_at
            FROM tenant_configs
            WHERE tenant_id = $1
            ORDER BY key ASC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to get tenant configs: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn update(&self, id: i64, update: UpdateTenantConfig) -> Result<TenantConfig, ApiError> {
        let row: TenantConfigRow = sqlx::query_as(
            r#"
            UPDATE tenant_configs
            SET
                value = COALESCE($1, value),
                is_encrypted = COALESCE($2, is_encrypted),
                updated_at = NOW()
            WHERE id = $3
            RETURNING id, tenant_id, key, value, is_encrypted, created_at, updated_at
            "#,
        )
        .bind(&update.value)
        .bind(update.is_encrypted)
        .bind(id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "TenantConfig"))?;

        Ok(row.into())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM tenant_configs
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete tenant config: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("TenantConfig not found".to_string()));
        }

        Ok(())
    }

    async fn delete_by_tenant(&self, tenant_id: i64) -> Result<(), ApiError> {
        sqlx::query(
            r#"
            DELETE FROM tenant_configs
            WHERE tenant_id = $1
            "#,
        )
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete tenant configs: {}", e)))?;

        Ok(())
    }
}
