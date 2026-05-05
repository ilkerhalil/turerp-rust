//! PostgreSQL MFA repository implementation

use async_trait::async_trait;
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use super::model::{MfaChallenge, MfaMethod, MfaSettings};
use super::repository::{BoxMfaRepository, MfaRepository};
use crate::error::ApiError;

/// Database row representation for MFA settings
#[derive(Debug, FromRow)]
struct MfaSettingsRow {
    user_id: i64,
    tenant_id: i64,
    totp_secret: Option<String>,
    mfa_enabled: bool,
    backup_codes: Vec<String>,
    method: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<MfaSettingsRow> for MfaSettings {
    fn from(row: MfaSettingsRow) -> Self {
        let method = row.method.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid MFA method '{}' in database: {}, defaulting to None",
                row.method,
                e
            );
            MfaMethod::None
        });

        Self {
            user_id: row.user_id,
            tenant_id: row.tenant_id,
            totp_secret: row.totp_secret,
            mfa_enabled: row.mfa_enabled,
            backup_codes: row.backup_codes,
            method,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Database row representation for MFA challenge
#[derive(Debug, FromRow)]
struct MfaChallengeRow {
    user_id: i64,
    tenant_id: i64,
    code: String,
    expires_at: chrono::DateTime<chrono::Utc>,
    attempts: i32,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<MfaChallengeRow> for MfaChallenge {
    fn from(row: MfaChallengeRow) -> Self {
        Self {
            user_id: row.user_id,
            tenant_id: row.tenant_id,
            code: row.code,
            expires_at: row.expires_at,
            attempts: row.attempts,
            created_at: row.created_at,
        }
    }
}

/// PostgreSQL MFA repository
pub struct PostgresMfaRepository {
    pool: Arc<PgPool>,
}

impl PostgresMfaRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub fn into_boxed(self) -> BoxMfaRepository {
        Arc::new(self) as BoxMfaRepository
    }
}

#[async_trait]
impl MfaRepository for PostgresMfaRepository {
    async fn find_by_user_id(
        &self,
        user_id: i64,
        tenant_id: i64,
    ) -> Result<Option<MfaSettings>, ApiError> {
        let result: Option<MfaSettingsRow> = sqlx::query_as(
            r#"
            SELECT user_id, tenant_id, totp_secret, mfa_enabled, backup_codes, method, created_at, updated_at
            FROM mfa_settings
            WHERE user_id = $1 AND tenant_id = $2
            "#,
        )
        .bind(user_id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find MFA settings: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn save(&self, settings: &MfaSettings) -> Result<(), ApiError> {
        let method_str = settings.method.to_string();

        sqlx::query(
            r#"
            INSERT INTO mfa_settings (user_id, tenant_id, totp_secret, mfa_enabled, backup_codes, method, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())
            ON CONFLICT (user_id, tenant_id) DO UPDATE SET
                totp_secret = EXCLUDED.totp_secret,
                mfa_enabled = EXCLUDED.mfa_enabled,
                backup_codes = EXCLUDED.backup_codes,
                method = EXCLUDED.method,
                updated_at = NOW()
            "#,
        )
        .bind(settings.user_id)
        .bind(settings.tenant_id)
        .bind(&settings.totp_secret)
        .bind(settings.mfa_enabled)
        .bind(&settings.backup_codes)
        .bind(&method_str)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to save MFA settings: {}", e)))?;

        Ok(())
    }

    async fn update_totp_secret(
        &self,
        user_id: i64,
        tenant_id: i64,
        secret: Option<String>,
    ) -> Result<(), ApiError> {
        sqlx::query(
            r#"
            INSERT INTO mfa_settings (user_id, tenant_id, totp_secret, mfa_enabled, backup_codes, method, created_at, updated_at)
            VALUES ($1, $2, $3, false, ARRAY[]::text[], 'none', NOW(), NOW())
            ON CONFLICT (user_id, tenant_id) DO UPDATE SET
                totp_secret = EXCLUDED.totp_secret,
                updated_at = NOW()
            "#,
        )
        .bind(user_id)
        .bind(tenant_id)
        .bind(&secret)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to update TOTP secret: {}", e)))?;

        Ok(())
    }

    async fn add_backup_codes(
        &self,
        user_id: i64,
        tenant_id: i64,
        codes: Vec<String>,
    ) -> Result<(), ApiError> {
        sqlx::query(
            r#"
            UPDATE mfa_settings
            SET backup_codes = $1, updated_at = NOW()
            WHERE user_id = $2 AND tenant_id = $3
            "#,
        )
        .bind(&codes)
        .bind(user_id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to add backup codes: {}", e)))?;

        Ok(())
    }

    async fn invalidate_backup_code(
        &self,
        user_id: i64,
        tenant_id: i64,
        code: &str,
    ) -> Result<bool, ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE mfa_settings
            SET backup_codes = array_remove(backup_codes, $1), updated_at = NOW()
            WHERE user_id = $2 AND tenant_id = $3 AND $1 = ANY(backup_codes)
            "#,
        )
        .bind(code)
        .bind(user_id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to invalidate backup code: {}", e)))?;

        Ok(result.rows_affected() > 0)
    }

    async fn create_challenge(&self, challenge: &MfaChallenge) -> Result<(), ApiError> {
        sqlx::query(
            r#"
            INSERT INTO mfa_challenges (user_id, tenant_id, code, expires_at, attempts, created_at)
            VALUES ($1, $2, $3, $4, $5, NOW())
            ON CONFLICT (user_id, tenant_id, code) DO UPDATE SET
                expires_at = EXCLUDED.expires_at,
                attempts = EXCLUDED.attempts,
                created_at = EXCLUDED.created_at
            "#,
        )
        .bind(challenge.user_id)
        .bind(challenge.tenant_id)
        .bind(&challenge.code)
        .bind(challenge.expires_at)
        .bind(challenge.attempts)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to create challenge: {}", e)))?;

        Ok(())
    }

    async fn find_challenge(
        &self,
        user_id: i64,
        tenant_id: i64,
        code: &str,
    ) -> Result<Option<MfaChallenge>, ApiError> {
        let result: Option<MfaChallengeRow> = sqlx::query_as(
            r#"
            SELECT user_id, tenant_id, code, expires_at, attempts, created_at
            FROM mfa_challenges
            WHERE user_id = $1 AND tenant_id = $2 AND code = $3
            "#,
        )
        .bind(user_id)
        .bind(tenant_id)
        .bind(code)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find challenge: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn delete_challenge(
        &self,
        user_id: i64,
        tenant_id: i64,
        code: &str,
    ) -> Result<(), ApiError> {
        sqlx::query(
            r#"
            DELETE FROM mfa_challenges
            WHERE user_id = $1 AND tenant_id = $2 AND code = $3
            "#,
        )
        .bind(user_id)
        .bind(tenant_id)
        .bind(code)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete challenge: {}", e)))?;

        Ok(())
    }
}
