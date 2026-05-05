//! API Key repository trait and implementations

use async_trait::async_trait;
use parking_lot::Mutex;
use std::sync::Arc;

use crate::common::pagination::PaginatedResult;
use crate::domain::api_key::model::{ApiKey, ApiKeyScope};
use crate::error::ApiError;

/// API Key repository trait
#[allow(clippy::too_many_arguments)]
#[async_trait]
pub trait ApiKeyRepository: Send + Sync {
    /// Create a new API key
    async fn create(
        &self,
        name: String,
        key_hash: String,
        key_prefix: String,
        tenant_id: i64,
        user_id: i64,
        scopes: Vec<ApiKeyScope>,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<ApiKey, ApiError>;

    /// Find API key by ID
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<ApiKey>, ApiError>;

    /// Find API key by hash (for authentication)
    async fn find_by_key_hash(&self, key_hash: &str) -> Result<Option<ApiKey>, ApiError>;

    /// List API keys for a tenant
    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<ApiKey>, ApiError>;

    /// List API keys for a tenant with pagination
    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<ApiKey>, ApiError>;

    /// Update an API key
    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        name: Option<String>,
        scopes: Option<Vec<ApiKeyScope>>,
        is_active: Option<bool>,
        expires_at: Option<Option<chrono::DateTime<chrono::Utc>>>,
    ) -> Result<ApiKey, ApiError>;

    /// Delete an API key
    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;

    /// Update last_used_at timestamp
    async fn touch_last_used(&self, id: i64) -> Result<(), ApiError>;
}

/// In-memory API key repository for testing
struct InMemoryApiKeyInner {
    keys: Vec<ApiKey>,
    next_id: i64,
}

/// In-memory API key repository
pub struct InMemoryApiKeyRepository {
    inner: Mutex<InMemoryApiKeyInner>,
}

impl InMemoryApiKeyRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryApiKeyInner {
                keys: Vec::new(),
                next_id: 1,
            }),
        }
    }
}

impl Default for InMemoryApiKeyRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ApiKeyRepository for InMemoryApiKeyRepository {
    async fn create(
        &self,
        name: String,
        key_hash: String,
        key_prefix: String,
        tenant_id: i64,
        user_id: i64,
        scopes: Vec<ApiKeyScope>,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<ApiKey, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_id;
        inner.next_id += 1;

        let key = ApiKey {
            id,
            name,
            key_hash,
            key_prefix,
            tenant_id,
            user_id,
            scopes,
            is_active: true,
            expires_at,
            last_used_at: None,
            created_at: chrono::Utc::now(),
            updated_at: None,
        };

        inner.keys.push(key.clone());
        Ok(key)
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<ApiKey>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .keys
            .iter()
            .find(|k| k.id == id && k.tenant_id == tenant_id)
            .cloned())
    }

    async fn find_by_key_hash(&self, key_hash: &str) -> Result<Option<ApiKey>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .keys
            .iter()
            .find(|k| k.key_hash == key_hash && k.is_active)
            .cloned())
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<ApiKey>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .keys
            .iter()
            .filter(|k| k.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<ApiKey>, ApiError> {
        let inner = self.inner.lock();
        let all: Vec<_> = inner
            .keys
            .iter()
            .filter(|k| k.tenant_id == tenant_id)
            .cloned()
            .collect();
        let total = all.len() as u64;
        let items: Vec<_> = all
            .into_iter()
            .skip(((page.saturating_sub(1)) * per_page) as usize)
            .take(per_page as usize)
            .collect();
        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        name: Option<String>,
        scopes: Option<Vec<ApiKeyScope>>,
        is_active: Option<bool>,
        expires_at: Option<Option<chrono::DateTime<chrono::Utc>>>,
    ) -> Result<ApiKey, ApiError> {
        let mut inner = self.inner.lock();
        let key = inner
            .keys
            .iter_mut()
            .find(|k| k.id == id && k.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("API key {} not found", id)))?;

        if let Some(n) = name {
            key.name = n;
        }
        if let Some(s) = scopes {
            key.scopes = s;
        }
        if let Some(active) = is_active {
            key.is_active = active;
        }
        // Handle nested Option: expires_at = Some(None) clears, Some(Some(dt)) sets, None = no change
        if let Some(exp_opt) = expires_at {
            key.expires_at = exp_opt;
        }
        key.updated_at = Some(chrono::Utc::now());

        Ok(key.clone())
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let len_before = inner.keys.len();
        inner
            .keys
            .retain(|k| !(k.id == id && k.tenant_id == tenant_id));

        if inner.keys.len() == len_before {
            return Err(ApiError::NotFound(format!("API key {} not found", id)));
        }
        Ok(())
    }

    async fn touch_last_used(&self, id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let key = inner
            .keys
            .iter_mut()
            .find(|k| k.id == id)
            .ok_or_else(|| ApiError::NotFound(format!("API key {} not found", id)))?;
        key.last_used_at = Some(chrono::Utc::now());
        Ok(())
    }
}

/// Type alias for a boxed API key repository
pub type BoxApiKeyRepository = Arc<dyn ApiKeyRepository>;
