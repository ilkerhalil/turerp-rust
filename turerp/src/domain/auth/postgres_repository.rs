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
    async fn is_revoked(&self, token_hash: &str) -> Result<bool, ApiError> {
        let result = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM revoked_tokens WHERE token_hash = $1)",
        )
        .bind(token_hash)
        .fetch_one(&*self.pool)
        .await;

        match result {
            Ok(is_revoked) => Ok(is_revoked),
            Err(e) => {
                // Fail CLOSED: on DB errors, treat the token as revoked
                // so it is rejected. The previous implementation used
                // `.unwrap_or(false)` which accepted revoked tokens during
                // transient DB failures (issue #324).
                tracing::error!(
                    "Revoked token check DB error: {}. Denying token (fail-closed).",
                    e
                );
                Err(ApiError::Database(format!(
                    "Failed to check revoked token status: {}",
                    e
                )))
            }
        }
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

    async fn purge_expired(&self) -> Result<u64, ApiError> {
        // Use a 1-hour safety margin to tolerate clock skew between the
        // application server (which sets `expires_at` from JWT `exp`) and
        // the database server (whose `NOW()` we compare against). Without
        // this margin, a DB clock running ahead could prematurely purge
        // revocation rows for tokens the application still considers valid.
        let result =
            sqlx::query("DELETE FROM revoked_tokens WHERE expires_at < NOW() - INTERVAL '1 hour'")
                .execute(&*self.pool)
                .await;

        match result {
            Ok(r) => {
                let rows = r.rows_affected();
                if rows > 0 {
                    tracing::info!("Purged {} expired revoked tokens", rows);
                }
                Ok(rows)
            }
            Err(e) => {
                tracing::error!("Failed to purge expired revoked tokens: {}", e);
                Err(map_sqlx_error(e, "RevokedToken"))
            }
        }
    }
}
