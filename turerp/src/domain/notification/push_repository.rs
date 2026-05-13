//! Push token repository traits and in-memory implementation

use async_trait::async_trait;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

use crate::common::soft_delete::SoftDeletable;
use crate::domain::notification::push_token::PushToken;
use crate::error::ApiError;

/// Repository trait for push tokens
#[async_trait]
pub trait PushTokenRepository: Send + Sync {
    async fn create(&self, token: PushToken) -> Result<PushToken, ApiError>;
    async fn find_by_user(&self, tenant_id: i64, user_id: i64) -> Result<Vec<PushToken>, ApiError>;
    async fn find_by_token(
        &self,
        tenant_id: i64,
        token: &str,
    ) -> Result<Option<PushToken>, ApiError>;
    async fn update(&self, token: PushToken) -> Result<PushToken, ApiError>;
    async fn deactivate(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
    async fn find_all_for_tenant(&self, tenant_id: i64) -> Result<Vec<PushToken>, ApiError>;
}

pub type BoxPushTokenRepository = Arc<dyn PushTokenRepository>;

struct PushTokenInner {
    tokens: HashMap<i64, PushToken>,
    next_id: i64,
}

pub struct InMemoryPushTokenRepository {
    inner: Mutex<PushTokenInner>,
}

impl InMemoryPushTokenRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(PushTokenInner {
                tokens: HashMap::new(),
                next_id: 1,
            }),
        }
    }
}

impl Default for InMemoryPushTokenRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PushTokenRepository for InMemoryPushTokenRepository {
    async fn create(&self, mut token: PushToken) -> Result<PushToken, ApiError> {
        let mut inner = self.inner.lock();
        token.id = inner.next_id;
        inner.next_id += 1;
        inner.tokens.insert(token.id, token.clone());
        Ok(token)
    }

    async fn find_by_user(&self, tenant_id: i64, user_id: i64) -> Result<Vec<PushToken>, ApiError> {
        let inner = self.inner.lock();
        let mut results: Vec<_> = inner
            .tokens
            .values()
            .filter(|t| {
                t.tenant_id == tenant_id && t.user_id == user_id && t.is_active && !t.is_deleted()
            })
            .cloned()
            .collect();
        results.sort_by_key(|t| std::cmp::Reverse(t.created_at));
        Ok(results)
    }

    async fn find_by_token(
        &self,
        tenant_id: i64,
        token: &str,
    ) -> Result<Option<PushToken>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .tokens
            .values()
            .find(|t| t.tenant_id == tenant_id && t.token == token && !t.is_deleted())
            .cloned())
    }

    async fn update(&self, token: PushToken) -> Result<PushToken, ApiError> {
        let mut inner = self.inner.lock();
        if !inner.tokens.contains_key(&token.id) {
            return Err(ApiError::NotFound(format!(
                "Push token {} not found",
                token.id
            )));
        }
        inner.tokens.insert(token.id, token.clone());
        Ok(token)
    }

    async fn deactivate(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let token = inner
            .tokens
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Push token {} not found", id)))?;
        if token.tenant_id != tenant_id {
            return Err(ApiError::Unauthorized("Tenant mismatch".to_string()));
        }
        token.is_active = false;
        token.updated_at = chrono::Utc::now();
        Ok(())
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let token = inner
            .tokens
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Push token {} not found", id)))?;
        if token.tenant_id != tenant_id {
            return Err(ApiError::Unauthorized("Tenant mismatch".to_string()));
        }
        token.mark_deleted(-1);
        Ok(())
    }

    async fn find_all_for_tenant(&self, tenant_id: i64) -> Result<Vec<PushToken>, ApiError> {
        let inner = self.inner.lock();
        let mut results: Vec<_> = inner
            .tokens
            .values()
            .filter(|t| t.tenant_id == tenant_id && !t.is_deleted())
            .cloned()
            .collect();
        results.sort_by_key(|t| std::cmp::Reverse(t.created_at));
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::notification::push_token::{DeviceType, PushToken};

    fn make_token(
        tenant_id: i64,
        user_id: i64,
        device_type: DeviceType,
        token_str: &str,
    ) -> PushToken {
        PushToken {
            id: 0,
            tenant_id,
            user_id,
            device_type,
            token: token_str.to_string(),
            is_active: true,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            deleted_at: None,
            deleted_by: None,
        }
    }

    #[tokio::test]
    async fn test_create_and_find() {
        let repo = InMemoryPushTokenRepository::new();
        let token = make_token(1, 1, DeviceType::Ios, "token1");
        let created = repo.create(token).await.unwrap();
        assert_eq!(created.id, 1);

        let found = repo.find_by_user(1, 1).await.unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].token, "token1");
    }

    #[tokio::test]
    async fn test_find_by_token() {
        let repo = InMemoryPushTokenRepository::new();
        let token = make_token(1, 1, DeviceType::Android, "abc123");
        repo.create(token).await.unwrap();

        let found = repo.find_by_token(1, "abc123").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().device_type, DeviceType::Android);

        let missing = repo.find_by_token(1, "missing").await.unwrap();
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn test_deactivate() {
        let repo = InMemoryPushTokenRepository::new();
        let token = make_token(1, 1, DeviceType::Web, "web_token");
        let created = repo.create(token).await.unwrap();

        repo.deactivate(created.id, 1).await.unwrap();

        let found = repo.find_by_user(1, 1).await.unwrap();
        assert!(found.is_empty());

        let by_token = repo.find_by_token(1, "web_token").await.unwrap();
        assert!(by_token.is_some());
        assert!(!by_token.unwrap().is_active);
    }

    #[tokio::test]
    async fn test_tenant_isolation() {
        let repo = InMemoryPushTokenRepository::new();
        repo.create(make_token(1, 1, DeviceType::Ios, "t1"))
            .await
            .unwrap();
        repo.create(make_token(2, 2, DeviceType::Android, "t2"))
            .await
            .unwrap();

        let tenant1 = repo.find_by_user(1, 1).await.unwrap();
        let tenant2 = repo.find_by_user(2, 2).await.unwrap();
        assert_eq!(tenant1.len(), 1);
        assert_eq!(tenant2.len(), 1);
    }
}
