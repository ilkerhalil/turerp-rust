//! PostgreSQL LDAP configuration repository implementation
use async_trait::async_trait;
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::db::error::map_sqlx_error;
use crate::domain::ldap::model::{CreateLdapConfig, LdapConfig, UpdateLdapConfig};
use crate::domain::ldap::repository::{BoxLdapConfigRepository, LdapConfigRepository};
use crate::error::ApiError;
use crate::utils::encryption::encrypt;

/// Database row representation for LDAP configuration
#[derive(Debug, FromRow)]
struct LdapConfigRow {
    id: i64,
    tenant_id: i64,
    ldap_url: String,
    bind_dn: String,
    bind_password_encrypted: String,
    base_dn: String,
    user_filter: String,
    is_active: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<LdapConfigRow> for LdapConfig {
    fn from(row: LdapConfigRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            ldap_url: row.ldap_url,
            bind_dn: row.bind_dn,
            bind_password_encrypted: row.bind_password_encrypted,
            base_dn: row.base_dn,
            user_filter: row.user_filter,
            is_active: row.is_active,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// PostgreSQL LDAP configuration repository
pub struct PostgresLdapConfigRepository {
    pool: Arc<PgPool>,
}

impl PostgresLdapConfigRepository {
    /// Create a new PostgreSQL LDAP configuration repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxLdapConfigRepository {
        Arc::new(self) as BoxLdapConfigRepository
    }
}

#[async_trait]
impl LdapConfigRepository for PostgresLdapConfigRepository {
    async fn create(
        &self,
        tenant_id: i64,
        create: CreateLdapConfig,
        encryption_key: &[u8],
    ) -> Result<LdapConfig, ApiError> {
        let encrypted_password = encrypt(&create.bind_password, encryption_key)
            .map_err(|e| ApiError::Internal(format!("Failed to encrypt password: {}", e)))?;

        let row: LdapConfigRow = sqlx::query_as(
            r#"
            INSERT INTO ldap_configs (tenant_id, ldap_url, bind_dn, bind_password_encrypted, base_dn, user_filter, is_active, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, true, NOW())
            RETURNING id, tenant_id, ldap_url, bind_dn, bind_password_encrypted, base_dn, user_filter, is_active, created_at, updated_at
            "#,
        )
        .bind(tenant_id)
        .bind(&create.ldap_url)
        .bind(&create.bind_dn)
        .bind(&encrypted_password)
        .bind(&create.base_dn)
        .bind(&create.user_filter)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "LdapConfig"))?;

        Ok(row.into())
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Option<LdapConfig>, ApiError> {
        let result: Option<LdapConfigRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, ldap_url, bind_dn, bind_password_encrypted, base_dn, user_filter, is_active, created_at, updated_at
            FROM ldap_configs
            WHERE tenant_id = $1
            "#,
        )
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find LDAP config: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn update(
        &self,
        tenant_id: i64,
        update: UpdateLdapConfig,
        encryption_key: &[u8],
    ) -> Result<LdapConfig, ApiError> {
        let bind_password_encrypted =
            if let Some(ref pwd) = update.bind_password {
                Some(encrypt(pwd, encryption_key).map_err(|e| {
                    ApiError::Internal(format!("Failed to encrypt password: {}", e))
                })?)
            } else {
                None
            };

        let row: LdapConfigRow = sqlx::query_as(
            r#"
            UPDATE ldap_configs
            SET
                ldap_url = COALESCE($1, ldap_url),
                bind_dn = COALESCE($2, bind_dn),
                bind_password_encrypted = COALESCE($3, bind_password_encrypted),
                base_dn = COALESCE($4, base_dn),
                user_filter = COALESCE($5, user_filter),
                is_active = COALESCE($6, is_active),
                updated_at = NOW()
            WHERE tenant_id = $7
            RETURNING id, tenant_id, ldap_url, bind_dn, bind_password_encrypted, base_dn, user_filter, is_active, created_at, updated_at
            "#,
        )
        .bind(&update.ldap_url)
        .bind(&update.bind_dn)
        .bind(&bind_password_encrypted)
        .bind(&update.base_dn)
        .bind(&update.user_filter)
        .bind(update.is_active)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "LdapConfig"))?;

        Ok(row.into())
    }

    async fn delete(&self, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM ldap_configs
            WHERE tenant_id = $1
            "#,
        )
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete LDAP config: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(
                "LDAP configuration not found".to_string(),
            ));
        }

        Ok(())
    }
}
