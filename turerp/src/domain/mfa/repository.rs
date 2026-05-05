//! MFA repository trait and in-memory implementation

use async_trait::async_trait;
use parking_lot::Mutex;
use std::sync::Arc;

use super::model::{MfaChallenge, MfaSettings};
use crate::error::ApiError;

/// MFA repository trait
#[async_trait]
pub trait MfaRepository: Send + Sync {
    /// Find MFA settings by user ID
    async fn find_by_user_id(
        &self,
        user_id: i64,
        tenant_id: i64,
    ) -> Result<Option<MfaSettings>, ApiError>;

    /// Create or update MFA settings
    async fn save(&self, settings: &MfaSettings) -> Result<(), ApiError>;

    /// Update TOTP secret
    async fn update_totp_secret(
        &self,
        user_id: i64,
        tenant_id: i64,
        secret: Option<String>,
    ) -> Result<(), ApiError>;

    /// Add backup codes (hashed)
    async fn add_backup_codes(
        &self,
        user_id: i64,
        tenant_id: i64,
        codes: Vec<String>,
    ) -> Result<(), ApiError>;

    /// Invalidate a backup code (remove it)
    async fn invalidate_backup_code(
        &self,
        user_id: i64,
        tenant_id: i64,
        code: &str,
    ) -> Result<bool, ApiError>;

    /// Create an MFA challenge
    async fn create_challenge(&self, challenge: &MfaChallenge) -> Result<(), ApiError>;

    /// Find an MFA challenge by user ID and code
    async fn find_challenge(
        &self,
        user_id: i64,
        tenant_id: i64,
        code: &str,
    ) -> Result<Option<MfaChallenge>, ApiError>;

    /// Delete an MFA challenge
    async fn delete_challenge(
        &self,
        user_id: i64,
        tenant_id: i64,
        code: &str,
    ) -> Result<(), ApiError>;
}

/// Type alias for boxed MFA repository
pub type BoxMfaRepository = Arc<dyn MfaRepository>;

/// Inner state for in-memory MFA repository
struct InMemoryMfaInner {
    settings: Vec<MfaSettings>,
    challenges: Vec<MfaChallenge>,
}

/// In-memory MFA repository for testing
pub struct InMemoryMfaRepository {
    inner: Mutex<InMemoryMfaInner>,
}

impl InMemoryMfaRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryMfaInner {
                settings: Vec::new(),
                challenges: Vec::new(),
            }),
        }
    }
}

impl Default for InMemoryMfaRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MfaRepository for InMemoryMfaRepository {
    async fn find_by_user_id(
        &self,
        user_id: i64,
        tenant_id: i64,
    ) -> Result<Option<MfaSettings>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .settings
            .iter()
            .find(|s| s.user_id == user_id && s.tenant_id == tenant_id)
            .cloned())
    }

    async fn save(&self, settings: &MfaSettings) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        if let Some(existing) = inner
            .settings
            .iter_mut()
            .find(|s| s.user_id == settings.user_id && s.tenant_id == settings.tenant_id)
        {
            *existing = settings.clone();
        } else {
            inner.settings.push(settings.clone());
        }
        Ok(())
    }

    async fn update_totp_secret(
        &self,
        user_id: i64,
        tenant_id: i64,
        secret: Option<String>,
    ) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        if let Some(existing) = inner
            .settings
            .iter_mut()
            .find(|s| s.user_id == user_id && s.tenant_id == tenant_id)
        {
            existing.totp_secret = secret;
            existing.updated_at = Some(Utc::now());
        } else {
            inner.settings.push(MfaSettings {
                user_id,
                tenant_id,
                totp_secret: secret,
                mfa_enabled: false,
                backup_codes: Vec::new(),
                method: super::model::MfaMethod::None,
                created_at: Utc::now(),
                updated_at: Some(Utc::now()),
            });
        }
        Ok(())
    }

    async fn add_backup_codes(
        &self,
        user_id: i64,
        tenant_id: i64,
        codes: Vec<String>,
    ) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        if let Some(existing) = inner
            .settings
            .iter_mut()
            .find(|s| s.user_id == user_id && s.tenant_id == tenant_id)
        {
            existing.backup_codes = codes;
            existing.updated_at = Some(Utc::now());
        } else {
            inner.settings.push(MfaSettings {
                user_id,
                tenant_id,
                totp_secret: None,
                mfa_enabled: false,
                backup_codes: codes,
                method: super::model::MfaMethod::None,
                created_at: Utc::now(),
                updated_at: Some(Utc::now()),
            });
        }
        Ok(())
    }

    async fn invalidate_backup_code(
        &self,
        user_id: i64,
        tenant_id: i64,
        code: &str,
    ) -> Result<bool, ApiError> {
        let mut inner = self.inner.lock();
        if let Some(existing) = inner
            .settings
            .iter_mut()
            .find(|s| s.user_id == user_id && s.tenant_id == tenant_id)
        {
            let hashed = crate::domain::mfa::service::MfaService::hash_backup_code(code);
            let before = existing.backup_codes.len();
            existing.backup_codes.retain(|c| c != &hashed);
            let removed = existing.backup_codes.len() < before;
            if removed {
                existing.updated_at = Some(Utc::now());
            }
            Ok(removed)
        } else {
            Ok(false)
        }
    }

    async fn create_challenge(&self, challenge: &MfaChallenge) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        inner.challenges.retain(|c| {
            !(c.user_id == challenge.user_id
                && c.tenant_id == challenge.tenant_id
                && c.code == challenge.code)
        });
        inner.challenges.push(challenge.clone());
        Ok(())
    }

    async fn find_challenge(
        &self,
        user_id: i64,
        tenant_id: i64,
        code: &str,
    ) -> Result<Option<MfaChallenge>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .challenges
            .iter()
            .find(|c| c.user_id == user_id && c.tenant_id == tenant_id && c.code == code)
            .cloned())
    }

    async fn delete_challenge(
        &self,
        user_id: i64,
        tenant_id: i64,
        code: &str,
    ) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        inner
            .challenges
            .retain(|c| !(c.user_id == user_id && c.tenant_id == tenant_id && c.code == code));
        Ok(())
    }
}

use chrono::Utc;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::mfa::model::{MfaChallenge, MfaMethod, MfaSettings};
    use crate::domain::mfa::service::MfaService;

    fn create_settings(user_id: i64, tenant_id: i64) -> MfaSettings {
        MfaSettings {
            user_id,
            tenant_id,
            totp_secret: Some("secret".to_string()),
            mfa_enabled: true,
            backup_codes: vec![
                MfaService::hash_backup_code("code1"),
                MfaService::hash_backup_code("code2"),
            ],
            method: MfaMethod::Totp,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    #[tokio::test]
    async fn test_find_by_user_id() {
        let repo = InMemoryMfaRepository::new();
        let settings = create_settings(1, 1);
        repo.save(&settings).await.unwrap();

        let found = repo.find_by_user_id(1, 1).await.unwrap();
        assert!(found.is_some());

        let not_found = repo.find_by_user_id(2, 1).await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_update_totp_secret() {
        let repo = InMemoryMfaRepository::new();
        repo.update_totp_secret(1, 1, Some("new_secret".to_string()))
            .await
            .unwrap();

        let found = repo.find_by_user_id(1, 1).await.unwrap().unwrap();
        assert_eq!(found.totp_secret, Some("new_secret".to_string()));
    }

    #[tokio::test]
    async fn test_add_backup_codes() {
        let repo = InMemoryMfaRepository::new();
        repo.add_backup_codes(1, 1, vec!["a".to_string(), "b".to_string()])
            .await
            .unwrap();

        let found = repo.find_by_user_id(1, 1).await.unwrap().unwrap();
        assert_eq!(found.backup_codes.len(), 2);
    }

    #[tokio::test]
    async fn test_invalidate_backup_code() {
        let repo = InMemoryMfaRepository::new();
        let settings = create_settings(1, 1);
        repo.save(&settings).await.unwrap();

        let removed = repo.invalidate_backup_code(1, 1, "code1").await.unwrap();
        assert!(removed);

        let found = repo.find_by_user_id(1, 1).await.unwrap().unwrap();
        assert_eq!(found.backup_codes.len(), 1);

        let removed = repo
            .invalidate_backup_code(1, 1, "nonexistent")
            .await
            .unwrap();
        assert!(!removed);
    }

    #[tokio::test]
    async fn test_challenge_lifecycle() {
        let repo = InMemoryMfaRepository::new();
        let challenge = MfaChallenge {
            user_id: 1,
            tenant_id: 1,
            code: "123456".to_string(),
            expires_at: Utc::now() + chrono::Duration::minutes(5),
            attempts: 0,
            created_at: Utc::now(),
        };

        repo.create_challenge(&challenge).await.unwrap();

        let found = repo.find_challenge(1, 1, "123456").await.unwrap();
        assert!(found.is_some());

        repo.delete_challenge(1, 1, "123456").await.unwrap();

        let not_found = repo.find_challenge(1, 1, "123456").await.unwrap();
        assert!(not_found.is_none());
    }
}
