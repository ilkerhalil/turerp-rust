//! API Key service for business logic

use crate::common::pagination::PaginatedResult;
use crate::domain::api_key::model::{
    extract_prefix, hash_api_key, ApiKey, ApiKeyCreationResult, ApiKeyResponse, ApiKeyScope,
    CreateApiKey, UpdateApiKey,
};
use crate::domain::api_key::repository::BoxApiKeyRepository;
use crate::error::ApiError;

/// API Key service
#[derive(Clone)]
pub struct ApiKeyService {
    repo: BoxApiKeyRepository,
}

impl ApiKeyService {
    pub fn new(repo: BoxApiKeyRepository) -> Self {
        Self { repo }
    }

    /// Create a new API key — returns the plain key only once
    pub async fn create_api_key(
        &self,
        create: CreateApiKey,
    ) -> Result<ApiKeyCreationResult, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;

        let plain_key = crate::domain::api_key::model::generate_api_key();
        let key_hash = hash_api_key(&plain_key);
        let key_prefix = extract_prefix(&plain_key);

        let key = self
            .repo
            .create(
                create.name,
                key_hash,
                key_prefix,
                create.tenant_id,
                create.user_id,
                create.scopes,
                create.expires_at,
            )
            .await?;

        Ok(ApiKeyCreationResult {
            api_key: key.into(),
            plain_key,
        })
    }

    /// Get an API key by ID
    pub async fn get_api_key(&self, id: i64, tenant_id: i64) -> Result<ApiKeyResponse, ApiError> {
        let key = self
            .repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("API key {} not found", id)))?;
        Ok(key.into())
    }

    /// List API keys for a tenant
    pub async fn list_api_keys(&self, tenant_id: i64) -> Result<Vec<ApiKeyResponse>, ApiError> {
        let keys = self.repo.find_by_tenant(tenant_id).await?;
        Ok(keys.into_iter().map(ApiKeyResponse::from).collect())
    }

    /// List API keys for a tenant with pagination
    pub async fn list_api_keys_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<ApiKeyResponse>, ApiError> {
        crate::common::pagination::PaginationParams { page, per_page }
            .validate()
            .map_err(ApiError::Validation)?;
        let result = self
            .repo
            .find_by_tenant_paginated(tenant_id, page, per_page)
            .await?;
        Ok(result.map(ApiKeyResponse::from))
    }

    /// Update an API key
    pub async fn update_api_key(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateApiKey,
    ) -> Result<ApiKeyResponse, ApiError> {
        let key = self
            .repo
            .update(
                id,
                tenant_id,
                update.name,
                update.scopes,
                update.is_active,
                update.expires_at,
            )
            .await?;
        Ok(key.into())
    }

    /// Delete an API key
    pub async fn delete_api_key(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.repo.delete(id, tenant_id).await
    }

    /// Soft delete an API key
    pub async fn soft_delete_api_key(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.repo.soft_delete(id, tenant_id, deleted_by).await
    }

    /// Restore a soft-deleted API key
    pub async fn restore_api_key(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.repo.restore(id, tenant_id).await
    }

    /// List deleted API keys for a tenant
    pub async fn list_deleted_api_keys(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<ApiKeyResponse>, ApiError> {
        let keys = self.repo.find_deleted(tenant_id).await?;
        Ok(keys.into_iter().map(ApiKeyResponse::from).collect())
    }

    /// Permanently destroy a soft-deleted API key
    pub async fn destroy_api_key(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.repo.destroy(id, tenant_id).await
    }

    /// Authenticate an API key (used by middleware/extractor)
    pub async fn authenticate(&self, plain_key: &str) -> Result<ApiKey, ApiError> {
        let key_hash = hash_api_key(plain_key);
        let key = self
            .repo
            .find_by_key_hash(&key_hash)
            .await?
            .ok_or_else(|| ApiError::Unauthorized("Invalid API key".to_string()))?;

        if !key.is_active {
            return Err(ApiError::Unauthorized("API key is disabled".to_string()));
        }

        if let Some(expires_at) = key.expires_at {
            if expires_at < chrono::Utc::now() {
                return Err(ApiError::Unauthorized("API key has expired".to_string()));
            }
        }

        // Update last_used_at (fire and forget)
        let repo = self.repo.clone();
        let key_id = key.id;
        tokio::spawn(async move {
            let _ = repo.touch_last_used(key_id).await;
        });

        Ok(key)
    }

    /// Check if an API key has a specific scope
    pub fn has_scope(key: &ApiKey, scope: &ApiKeyScope) -> bool {
        key.scopes.contains(&ApiKeyScope::All) || key.scopes.contains(scope)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::api_key::repository::InMemoryApiKeyRepository;
    use std::sync::Arc;

    fn create_service() -> ApiKeyService {
        let repo = Arc::new(InMemoryApiKeyRepository::new()) as BoxApiKeyRepository;
        ApiKeyService::new(repo)
    }

    #[tokio::test]
    async fn test_create_api_key() {
        let service = create_service();
        let create = CreateApiKey {
            name: "Test Key".to_string(),
            tenant_id: 1,
            user_id: 1,
            scopes: vec![ApiKeyScope::CariRead, ApiKeyScope::CariWrite],
            expires_at: None,
        };

        let result = service.create_api_key(create).await;
        assert!(result.is_ok());
        let creation = result.unwrap();
        assert!(creation.plain_key.starts_with("tuk_"));
        assert!(!creation.api_key.key_prefix.is_empty());
    }

    #[tokio::test]
    async fn test_authenticate_api_key() {
        let service = create_service();
        let create = CreateApiKey {
            name: "Auth Test".to_string(),
            tenant_id: 1,
            user_id: 1,
            scopes: vec![ApiKeyScope::All],
            expires_at: None,
        };

        let creation = service.create_api_key(create).await.unwrap();
        let auth_result = service.authenticate(&creation.plain_key).await;
        assert!(auth_result.is_ok());
        let key = auth_result.unwrap();
        assert_eq!(key.name, "Auth Test");
    }

    #[tokio::test]
    async fn test_authenticate_invalid_key() {
        let service = create_service();
        let result = service.authenticate("tuk_invalid_key_value").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_has_scope() {
        let service = create_service();
        let create = CreateApiKey {
            name: "Scoped Key".to_string(),
            tenant_id: 1,
            user_id: 1,
            scopes: vec![ApiKeyScope::CariRead, ApiKeyScope::CariWrite],
            expires_at: None,
        };

        let creation = service.create_api_key(create).await.unwrap();
        let key = service.authenticate(&creation.plain_key).await.unwrap();

        assert!(ApiKeyService::has_scope(&key, &ApiKeyScope::CariRead));
        assert!(ApiKeyService::has_scope(&key, &ApiKeyScope::CariWrite));
        assert!(!ApiKeyService::has_scope(&key, &ApiKeyScope::InvoiceRead));
    }

    #[tokio::test]
    async fn test_has_scope_all() {
        let service = create_service();
        let create = CreateApiKey {
            name: "Super Key".to_string(),
            tenant_id: 1,
            user_id: 1,
            scopes: vec![ApiKeyScope::All],
            expires_at: None,
        };

        let creation = service.create_api_key(create).await.unwrap();
        let key = service.authenticate(&creation.plain_key).await.unwrap();

        assert!(ApiKeyService::has_scope(&key, &ApiKeyScope::CariRead));
        assert!(ApiKeyService::has_scope(&key, &ApiKeyScope::InvoiceWrite));
    }

    #[tokio::test]
    async fn test_list_api_keys() {
        let service = create_service();
        for i in 0..3 {
            let create = CreateApiKey {
                name: format!("Key {}", i),
                tenant_id: 1,
                user_id: 1,
                scopes: vec![ApiKeyScope::All],
                expires_at: None,
            };
            service.create_api_key(create).await.unwrap();
        }

        let keys = service.list_api_keys(1).await.unwrap();
        assert_eq!(keys.len(), 3);
    }

    #[tokio::test]
    async fn test_delete_api_key() {
        let service = create_service();
        let create = CreateApiKey {
            name: "Delete Me".to_string(),
            tenant_id: 1,
            user_id: 1,
            scopes: vec![ApiKeyScope::All],
            expires_at: None,
        };

        let creation = service.create_api_key(create).await.unwrap();
        let id = creation.api_key.id;

        service.delete_api_key(id, 1).await.unwrap();
        let result = service.get_api_key(id, 1).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_api_key() {
        let service = create_service();
        let create = CreateApiKey {
            name: "Original".to_string(),
            tenant_id: 1,
            user_id: 1,
            scopes: vec![ApiKeyScope::CariRead],
            expires_at: None,
        };

        let creation = service.create_api_key(create).await.unwrap();
        let id = creation.api_key.id;

        let update = UpdateApiKey {
            name: Some("Updated".to_string()),
            scopes: None,
            is_active: Some(false),
            expires_at: None,
        };

        let updated = service.update_api_key(id, 1, update).await.unwrap();
        assert_eq!(updated.name, "Updated");
        assert!(!updated.is_active);
    }
}
