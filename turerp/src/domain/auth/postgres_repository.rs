//! PostgreSQL revoked-token store implementation

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use std::sync::Arc;

use crate::db::error::map_sqlx_error;
use crate::domain::auth::repository::{BoxRevokedTokenStore, RevokedTokenStore};
use crate::error::ApiError;

/// PostgreSQL implementation of revoked token storage.
pub struct PostgresRevokedTokenStore {
    pool: Arc<PgPool>,
}

impl PostgresRevokedTokenStore {
    /// Create a new PostgreSQL revoked token store.
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object.
    pub fn into_boxed(self) -> BoxRevokedTokenStore {
        Arc::new(self) as BoxRevokedTokenStore
    }
}

#[async_trait]
impl RevokedTokenStore for PostgresRevokedTokenStore {
    async fn is_revoked(&self, token_hash: &str) -> bool {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM revoked_tokens WHERE token_hash = $1)",
        )
        .bind(token_hash)
        .fetch_one(&*self.pool)
        .await
        .unwrap_or(false)
    }

    async fn revoke(&self, token_hash: &str, expires_at: DateTime<Utc>) -> Result<(), ApiError> {
        sqlx::query(
            "INSERT INTO revoked_tokens (token_hash, expires_at, tenant_id) VALUES ($1, $2, $3) ON CONFLICT (token_hash) DO NOTHING",
        )
        .bind(token_hash)
        .bind(expires_at)
        .bind(1_i64) // tenant_id is required by table schema; revoked tokens are global
        .execute(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "RevokedToken"))?;

        Ok(())
    }
}
