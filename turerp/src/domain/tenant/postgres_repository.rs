//! PostgreSQL tenant repository implementation

use async_trait::async_trait;
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::domain::tenant::model::{CreateTenant, Tenant, UpdateTenant};
use crate::domain::tenant::repository::{BoxTenantRepository, TenantRepository};
use crate::error::ApiError;

/// Convert sqlx errors to ApiError with proper detection of error types
fn map_sqlx_error(e: sqlx::Error, entity: &str) -> ApiError {
    match e {
        sqlx::Error::RowNotFound => ApiError::NotFound(format!("{} not found", entity)),
        _ => {
            let msg = e.to_string();
            if msg.contains("duplicate key") || msg.contains("unique constraint") {
                ApiError::Conflict(format!("{} already exists", entity))
            } else {
                ApiError::Database(format!("Failed to operate on {}: {}", entity, e))
            }
        }
    }
}

/// Database row representation for Tenant
#[derive(Debug, FromRow)]
struct TenantRow {
    id: i64,
    name: String,
    subdomain: String,
    is_active: bool,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<TenantRow> for Tenant {
    fn from(row: TenantRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            subdomain: row.subdomain.clone(),
            db_name: crate::domain::tenant::model::generate_db_name(&row.subdomain),
            is_active: row.is_active,
            created_at: row.created_at,
        }
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
            RETURNING id, name, subdomain, is_active, created_at
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
            SELECT id, name, subdomain, is_active, created_at
            FROM tenants
            WHERE id = $1
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
            SELECT id, name, subdomain, is_active, created_at
            FROM tenants
            WHERE subdomain = $1
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
            SELECT id, name, subdomain, is_active, created_at
            FROM tenants
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find all tenants: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
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
            WHERE id = $4
            RETURNING id, name, subdomain, is_active, created_at
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
            DELETE FROM tenants
            WHERE id = $1
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

    async fn subdomain_exists(&self, subdomain: &str) -> Result<bool, ApiError> {
        let result: (bool,) = sqlx::query_as(
            r#"
            SELECT EXISTS(SELECT 1 FROM tenants WHERE subdomain = $1)
            "#,
        )
        .bind(subdomain)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to check subdomain: {}", e)))?;

        Ok(result.0)
    }
}
