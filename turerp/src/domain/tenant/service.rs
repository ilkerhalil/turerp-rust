//! Tenant service for business logic
use crate::common::pagination::PaginatedResult;
use crate::domain::tenant::model::{
    CreateTenant, CreateTenantConfig, Tenant, TenantConfigResponse, UpdateTenant,
    UpdateTenantConfig,
};
use crate::domain::tenant::repository::{BoxTenantConfigRepository, BoxTenantRepository};
use crate::error::ApiError;
use crate::utils::encryption::{decrypt, encrypt};
use zeroize::Zeroizing;

/// Tenant service
#[derive(Clone)]
pub struct TenantService {
    repo: BoxTenantRepository,
}

impl TenantService {
    pub fn new(repo: BoxTenantRepository) -> Self {
        Self { repo }
    }

    /// Create a new tenant
    pub async fn create_tenant(&self, create: CreateTenant) -> Result<Tenant, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;

        // Check if subdomain exists
        if self.repo.subdomain_exists(&create.subdomain).await? {
            return Err(ApiError::Conflict(format!(
                "Subdomain '{}' already exists",
                create.subdomain
            )));
        }

        let tenant = self.repo.create(create).await?;
        Ok(tenant)
    }

    /// Get tenant by ID
    pub async fn get_tenant(&self, id: i64) -> Result<Tenant, ApiError> {
        self.repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Tenant {} not found", id)))
    }

    /// Get tenant by subdomain
    pub async fn get_tenant_by_subdomain(&self, subdomain: &str) -> Result<Tenant, ApiError> {
        self.repo
            .find_by_subdomain(subdomain)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!("Tenant with subdomain '{}' not found", subdomain))
            })
    }

    /// Get all tenants
    pub async fn get_all_tenants(&self) -> Result<Vec<Tenant>, ApiError> {
        self.repo.find_all().await
    }

    /// Get all tenants paginated
    pub async fn get_all_tenants_paginated(
        &self,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Tenant>, ApiError> {
        crate::common::pagination::PaginationParams { page, per_page }
            .validate()
            .map_err(ApiError::Validation)?;
        self.repo.find_all_paginated(page, per_page).await
    }

    /// Update a tenant
    pub async fn update_tenant(&self, id: i64, update: UpdateTenant) -> Result<Tenant, ApiError> {
        // Check if subdomain changed and exists
        if let Some(ref subdomain) = update.subdomain {
            let existing = self.repo.find_by_subdomain(subdomain).await?;
            if let Some(t) = existing {
                if t.id != id {
                    return Err(ApiError::Conflict(format!(
                        "Subdomain '{}' already exists",
                        subdomain
                    )));
                }
            }
        }

        self.repo.update(id, update).await
    }

    /// Delete a tenant
    pub async fn delete_tenant(&self, id: i64) -> Result<(), ApiError> {
        self.repo.delete(id).await
    }

    /// Get tenant database URL
    pub fn get_database_url(&self, base_url: &str, tenant: &Tenant) -> String {
        format!("{}/{}", base_url.trim_end_matches('/'), tenant.db_name)
    }
}

/// Tenant config service with optional encryption support
#[derive(Clone)]
pub struct TenantConfigService {
    repo: BoxTenantConfigRepository,
    /// Optional encryption key for sensitive values (securely zeroed on drop)
    encryption_key: Option<Zeroizing<Vec<u8>>>,
}

impl TenantConfigService {
    /// Create a new config service without encryption
    pub fn new(repo: BoxTenantConfigRepository) -> Self {
        Self {
            repo,
            encryption_key: None,
        }
    }

    /// Create a config service with encryption support
    ///
    /// The encryption key is wrapped in `Zeroizing` to ensure it's securely
    /// cleared from memory when the service is dropped.
    pub fn with_encryption(repo: BoxTenantConfigRepository, encryption_key: Vec<u8>) -> Self {
        Self {
            repo,
            encryption_key: Some(Zeroizing::new(encryption_key)),
        }
    }

    /// Encrypt a value if encryption is enabled
    fn encrypt_value(&self, value: &serde_json::Value) -> Result<serde_json::Value, ApiError> {
        if let Some(ref key) = self.encryption_key {
            let plaintext = value.to_string();
            let encrypted = encrypt(&plaintext, key)
                .map_err(|e| ApiError::Internal(format!("Encryption failed: {}", e)))?;
            Ok(serde_json::Value::String(encrypted))
        } else {
            Err(ApiError::Internal(
                "Encryption key not configured for encrypted values".to_string(),
            ))
        }
    }

    /// Decrypt a value if encryption is enabled
    fn decrypt_value(&self, value: &serde_json::Value) -> Result<serde_json::Value, ApiError> {
        if let Some(ref key) = self.encryption_key {
            let ciphertext = value
                .as_str()
                .ok_or_else(|| ApiError::Internal("Encrypted value is not a string".to_string()))?;
            let decrypted = decrypt(ciphertext, key)
                .map_err(|e| ApiError::Internal(format!("Decryption failed: {}", e)))?;
            serde_json::from_str(&decrypted)
                .map_err(|e| ApiError::Internal(format!("Invalid decrypted JSON: {}", e)))
        } else {
            // Return as-is if no encryption configured (for development)
            Ok(value.clone())
        }
    }

    /// Set a config value
    pub async fn set_config(
        &self,
        mut create: CreateTenantConfig,
    ) -> Result<TenantConfigResponse, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;

        // Encrypt if requested and encryption is enabled
        if create.is_encrypted.unwrap_or(false) {
            create.value = self.encrypt_value(&create.value)?;
        }

        let config = self.repo.set(create).await?;
        Ok(config.into())
    }

    /// Get a config value
    pub async fn get_config(
        &self,
        tenant_id: i64,
        key: &str,
    ) -> Result<TenantConfigResponse, ApiError> {
        let config = self
            .repo
            .get(tenant_id, key)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Config '{}' not found", key)))?;

        // Decrypt if encrypted
        let response = if config.is_encrypted {
            let decrypted_value = self.decrypt_value(&config.value)?;
            TenantConfigResponse {
                id: config.id,
                tenant_id: config.tenant_id,
                key: config.key,
                value: decrypted_value,
                is_encrypted: config.is_encrypted,
            }
        } else {
            config.into()
        };

        Ok(response)
    }

    /// Get all configs for a tenant
    pub async fn get_all_configs(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<TenantConfigResponse>, ApiError> {
        let configs = self.repo.get_all(tenant_id).await?;
        let mut responses = Vec::new();

        for config in configs {
            let response = if config.is_encrypted {
                match self.decrypt_value(&config.value) {
                    Ok(decrypted_value) => TenantConfigResponse {
                        id: config.id,
                        tenant_id: config.tenant_id,
                        key: config.key,
                        value: decrypted_value,
                        is_encrypted: config.is_encrypted,
                    },
                    Err(_) => {
                        // Include encrypted values that can't be decrypted
                        // This allows listing configs even if decryption fails
                        config.into()
                    }
                }
            } else {
                config.into()
            };
            responses.push(response);
        }

        Ok(responses)
    }

    /// Update a config
    pub async fn update_config(
        &self,
        id: i64,
        mut update: UpdateTenantConfig,
    ) -> Result<TenantConfigResponse, ApiError> {
        // Encrypt new value if requested
        if let Some(ref value) = update.value {
            // Check if this should be encrypted
            if update.is_encrypted.unwrap_or(false) {
                update.value = Some(self.encrypt_value(value)?);
            }
        }

        let config = self.repo.update(id, update).await?;

        // Decrypt for response if encrypted
        let response = if config.is_encrypted {
            let decrypted_value = self.decrypt_value(&config.value)?;
            TenantConfigResponse {
                id: config.id,
                tenant_id: config.tenant_id,
                key: config.key,
                value: decrypted_value,
                is_encrypted: config.is_encrypted,
            }
        } else {
            config.into()
        };

        Ok(response)
    }

    /// Delete a config
    pub async fn delete_config(&self, id: i64) -> Result<(), ApiError> {
        self.repo.delete(id).await
    }

    /// Delete all configs for a tenant
    pub async fn delete_all_configs(&self, tenant_id: i64) -> Result<(), ApiError> {
        self.repo.delete_by_tenant(tenant_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::tenant::repository::{
        InMemoryTenantConfigRepository, InMemoryTenantRepository,
    };
    use serde_json::json;
    use std::sync::Arc;

    fn create_service() -> TenantService {
        let repo = Arc::new(InMemoryTenantRepository::new()) as BoxTenantRepository;
        TenantService::new(repo)
    }

    fn create_config_service() -> TenantConfigService {
        let repo = Arc::new(InMemoryTenantConfigRepository::new()) as BoxTenantConfigRepository;
        TenantConfigService::new(repo)
    }

    #[tokio::test]
    async fn test_create_tenant_success() {
        let service = create_service();

        let create = CreateTenant {
            name: "Test Company".to_string(),
            subdomain: "testco".to_string(),
        };

        let result = service.create_tenant(create).await;
        assert!(result.is_ok());
        let tenant = result.unwrap();
        assert_eq!(tenant.name, "Test Company");
        assert_eq!(tenant.subdomain, "testco");
    }

    #[tokio::test]
    async fn test_create_tenant_duplicate_subdomain() {
        let service = create_service();

        let create = CreateTenant {
            name: "Test Company".to_string(),
            subdomain: "testco".to_string(),
        };

        service.create_tenant(create.clone()).await.unwrap();

        let result = service.create_tenant(create).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ApiError::Conflict(_)));
    }

    #[tokio::test]
    async fn test_get_tenant_by_id() {
        let service = create_service();

        let create = CreateTenant {
            name: "Test Company".to_string(),
            subdomain: "testco".to_string(),
        };

        let created = service.create_tenant(create).await.unwrap();

        let result = service.get_tenant(created.id).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "Test Company");
    }

    #[tokio::test]
    async fn test_get_tenant_by_subdomain() {
        let service = create_service();

        let create = CreateTenant {
            name: "Test Company".to_string(),
            subdomain: "testco".to_string(),
        };

        service.create_tenant(create).await.unwrap();

        let result = service.get_tenant_by_subdomain("testco").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "Test Company");
    }

    #[tokio::test]
    async fn test_get_all_tenants() {
        let service = create_service();

        // Default tenant exists
        let result = service.get_all_tenants().await.unwrap();
        assert!(!result.is_empty());
    }

    #[tokio::test]
    async fn test_update_tenant() {
        let service = create_service();

        let create = CreateTenant {
            name: "Test Company".to_string(),
            subdomain: "testco".to_string(),
        };

        let created = service.create_tenant(create).await.unwrap();

        let update = UpdateTenant {
            name: Some("Updated Company".to_string()),
            ..Default::default()
        };

        let result = service.update_tenant(created.id, update).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "Updated Company");
    }

    #[tokio::test]
    async fn test_delete_tenant() {
        let service = create_service();

        let create = CreateTenant {
            name: "Test Company".to_string(),
            subdomain: "testco".to_string(),
        };

        let created = service.create_tenant(create).await.unwrap();

        let result = service.delete_tenant(created.id).await;
        assert!(result.is_ok());

        let result = service.get_tenant(created.id).await;
        assert!(result.is_err());
    }

    // TenantConfig tests
    #[tokio::test]
    async fn test_set_config() {
        let service = create_config_service();

        let create = CreateTenantConfig {
            tenant_id: 1,
            key: "app.theme".to_string(),
            value: json!("dark"),
            is_encrypted: None,
        };

        let result = service.set_config(create).await;
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.key, "app.theme");
        assert_eq!(config.value, json!("dark"));
    }

    #[tokio::test]
    async fn test_get_config() {
        let service = create_config_service();

        let create = CreateTenantConfig {
            tenant_id: 1,
            key: "app.locale".to_string(),
            value: json!("en-US"),
            is_encrypted: None,
        };

        service.set_config(create).await.unwrap();

        let result = service.get_config(1, "app.locale").await;
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.value, json!("en-US"));
    }

    #[tokio::test]
    async fn test_get_all_configs() {
        let service = create_config_service();

        // Create multiple configs
        service
            .set_config(CreateTenantConfig {
                tenant_id: 1,
                key: "app.theme".to_string(),
                value: json!("dark"),
                is_encrypted: None,
            })
            .await
            .unwrap();

        service
            .set_config(CreateTenantConfig {
                tenant_id: 1,
                key: "app.locale".to_string(),
                value: json!("en-US"),
                is_encrypted: None,
            })
            .await
            .unwrap();

        let result = service.get_all_configs(1).await;
        assert!(result.is_ok());
        let configs = result.unwrap();
        assert_eq!(configs.len(), 2);
    }

    #[tokio::test]
    async fn test_update_config() {
        let service = create_config_service();

        let create = CreateTenantConfig {
            tenant_id: 1,
            key: "app.theme".to_string(),
            value: json!("dark"),
            is_encrypted: None,
        };

        let created = service.set_config(create).await.unwrap();

        let update = UpdateTenantConfig {
            value: Some(json!("light")),
            is_encrypted: None,
        };

        let result = service.update_config(created.id, update).await;
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.value, json!("light"));
    }

    #[tokio::test]
    async fn test_delete_config() {
        let service = create_config_service();

        let create = CreateTenantConfig {
            tenant_id: 1,
            key: "app.theme".to_string(),
            value: json!("dark"),
            is_encrypted: None,
        };

        let created = service.set_config(create).await.unwrap();

        let result = service.delete_config(created.id).await;
        assert!(result.is_ok());

        let result = service.get_config(1, "app.theme").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_set_config_upsert() {
        let service = create_config_service();

        // Create config
        let create = CreateTenantConfig {
            tenant_id: 1,
            key: "app.theme".to_string(),
            value: json!("dark"),
            is_encrypted: None,
        };
        let created = service.set_config(create).await.unwrap();
        let first_id = created.id;

        // Update same key (should upsert)
        let update = CreateTenantConfig {
            tenant_id: 1,
            key: "app.theme".to_string(),
            value: json!("light"),
            is_encrypted: None,
        };
        let updated = service.set_config(update).await.unwrap();

        // Should have same ID (upsert)
        assert_eq!(updated.id, first_id);
        assert_eq!(updated.value, json!("light"));
    }

    #[tokio::test]
    async fn test_encrypted_config() {
        // Create service with encryption key
        let repo = Arc::new(InMemoryTenantConfigRepository::new()) as BoxTenantConfigRepository;
        let encryption_key = crate::utils::encryption::generate_key().to_vec();
        let service = TenantConfigService::with_encryption(repo, encryption_key);

        // Create encrypted config
        let create = CreateTenantConfig {
            tenant_id: 1,
            key: "db.password".to_string(),
            value: json!("super_secret_password"),
            is_encrypted: Some(true),
        };

        let created = service.set_config(create).await.unwrap();
        assert!(created.is_encrypted);

        // Value should be decrypted when retrieved
        let retrieved = service.get_config(1, "db.password").await.unwrap();
        assert_eq!(retrieved.value, json!("super_secret_password"));

        // Update encrypted value
        let update = UpdateTenantConfig {
            value: Some(json!("new_secret_password")),
            is_encrypted: Some(true),
        };
        let updated = service.update_config(created.id, update).await.unwrap();
        assert_eq!(updated.value, json!("new_secret_password"));
    }
}
