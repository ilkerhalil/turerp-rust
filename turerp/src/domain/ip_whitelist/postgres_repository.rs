//! PostgreSQL IP whitelist repository implementation

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::db::error::map_sqlx_error;
use crate::domain::ip_whitelist::model::{
    CreateIpWhitelistEntry, IpWhitelistEntry, UpdateIpWhitelistEntry,
};
use crate::domain::ip_whitelist::repository::IpWhitelistRepository;
use crate::error::ApiError;

/// Database row representation for IpWhitelistEntry
#[derive(Debug, FromRow)]
struct IpWhitelistEntryRow {
    id: i64,
    tenant_id: i64,
    ip_address: String,
    description: Option<String>,
    is_active: bool,
    created_at: DateTime<Utc>,
}

impl From<IpWhitelistEntryRow> for IpWhitelistEntry {
    fn from(row: IpWhitelistEntryRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            ip_address: row.ip_address,
            description: row.description,
            is_active: row.is_active,
            created_at: row.created_at,
        }
    }
}

/// PostgreSQL IP whitelist repository
pub struct PostgresIpWhitelistRepository {
    pool: Arc<PgPool>,
}

impl PostgresIpWhitelistRepository {
    /// Create a new PostgreSQL IP whitelist repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> Arc<dyn IpWhitelistRepository> {
        Arc::new(self) as Arc<dyn IpWhitelistRepository>
    }
}

#[async_trait]
impl IpWhitelistRepository for PostgresIpWhitelistRepository {
    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<IpWhitelistEntry>, ApiError> {
        let rows: Vec<IpWhitelistEntryRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, ip_address, description, is_active, created_at
            FROM ip_whitelist_entries
            WHERE tenant_id = $1 AND is_active = true
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!(
                "Failed to get IP whitelist entries by tenant: {}",
                e
            ))
        })?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn create(&self, entry: CreateIpWhitelistEntry) -> Result<IpWhitelistEntry, ApiError> {
        let row: IpWhitelistEntryRow = sqlx::query_as(
            r#"
            INSERT INTO ip_whitelist_entries (tenant_id, ip_address, description, is_active, created_at)
            VALUES ($1, $2, $3, $4, NOW())
            RETURNING id, tenant_id, ip_address, description, is_active, created_at
            "#,
        )
        .bind(entry.tenant_id)
        .bind(&entry.ip_address)
        .bind(&entry.description)
        .bind(entry.is_active)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "IP whitelist entry"))?;

        Ok(row.into())
    }

    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateIpWhitelistEntry,
    ) -> Result<IpWhitelistEntry, ApiError> {
        let result: Option<IpWhitelistEntryRow> = sqlx::query_as(
            r#"
            UPDATE ip_whitelist_entries
            SET
                ip_address = COALESCE($1, ip_address),
                description = COALESCE($2, description),
                is_active = COALESCE($3, is_active)
            WHERE id = $4 AND tenant_id = $5
            RETURNING id, tenant_id, ip_address, description, is_active, created_at
            "#,
        )
        .bind(&update.ip_address)
        .bind(&update.description)
        .bind(update.is_active)
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "IP whitelist entry"))?;

        result
            .map(|r| r.into())
            .ok_or_else(|| ApiError::NotFound(format!("IP whitelist entry {} not found", id)))
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM ip_whitelist_entries
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete IP whitelist entry: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!(
                "IP whitelist entry {} not found",
                id
            )));
        }

        Ok(())
    }

    async fn exists(&self, id: i64, tenant_id: i64) -> Result<bool, ApiError> {
        let result: Option<(i64,)> = sqlx::query_as(
            r#"
            SELECT 1
            FROM ip_whitelist_entries
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!(
                "Failed to check IP whitelist entry existence: {}",
                e
            ))
        })?;

        Ok(result.is_some())
    }

    async fn find_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<IpWhitelistEntry>, ApiError> {
        let result: Option<IpWhitelistEntryRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, ip_address, description, is_active, created_at
            FROM ip_whitelist_entries
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to get IP whitelist entry by id: {}", e))
        })?;

        Ok(result.map(|r| r.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_row_conversion_full() {
        let now = Utc::now();
        let row = IpWhitelistEntryRow {
            id: 1,
            tenant_id: 2,
            ip_address: "192.168.1.0/24".to_string(),
            description: Some("Office".to_string()),
            is_active: true,
            created_at: now,
        };

        let entry: IpWhitelistEntry = row.into();
        assert_eq!(entry.id, 1);
        assert_eq!(entry.tenant_id, 2);
        assert_eq!(entry.ip_address, "192.168.1.0/24");
        assert_eq!(entry.description, Some("Office".to_string()));
        assert!(entry.is_active);
        assert_eq!(entry.created_at, now);
    }

    #[test]
    fn test_row_conversion_no_description() {
        let now = Utc::now();
        let row = IpWhitelistEntryRow {
            id: 3,
            tenant_id: 4,
            ip_address: "10.0.0.0/8".to_string(),
            description: None,
            is_active: false,
            created_at: now,
        };

        let entry: IpWhitelistEntry = row.into();
        assert_eq!(entry.id, 3);
        assert_eq!(entry.tenant_id, 4);
        assert_eq!(entry.ip_address, "10.0.0.0/8");
        assert_eq!(entry.description, None);
        assert!(!entry.is_active);
        assert_eq!(entry.created_at, now);
    }
}
