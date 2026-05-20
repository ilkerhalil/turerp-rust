//! LDAP configuration repository trait and implementations

use async_trait::async_trait;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

use crate::domain::ldap::model::{CreateLdapConfig, LdapConfig, UpdateLdapConfig};
use crate::error::ApiError;
use crate::utils::encryption::encrypt;

/// LDAP configuration repository trait
#[async_trait]
pub trait LdapConfigRepository: Send + Sync {
    /// Create a new LDAP configuration
    async fn create(
        &self,
        tenant_id: i64,
        config: CreateLdapConfig,
        encryption_key: &[u8],
    ) -> Result<LdapConfig, ApiError>;

    /// Find LDAP configuration by tenant ID
    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Option<LdapConfig>, ApiError>;

    /// Update an existing LDAP configuration
    async fn update(
        &self,
        tenant_id: i64,
        config: UpdateLdapConfig,
        encryption_key: &[u8],
    ) -> Result<LdapConfig, ApiError>;

    /// Delete an LDAP configuration
    async fn delete(&self, tenant_id: i64) -> Result<(), ApiError>;
}

/// In-memory state for LDAP configuration repository
struct InMemoryLdapConfigInner {
    configs: HashMap<i64, LdapConfig>,
    next_id: i64,
}

/// In-memory LDAP configuration repository for testing
pub struct InMemoryLdapConfigRepository {
    inner: Mutex<InMemoryLdapConfigInner>,
}

impl InMemoryLdapConfigRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryLdapConfigInner {
                configs: HashMap::new(),
                next_id: 1,
            }),
        }
    }
}

impl Default for InMemoryLdapConfigRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LdapConfigRepository for InMemoryLdapConfigRepository {
    async fn create(
        &self,
        tenant_id: i64,
        create: CreateLdapConfig,
        encryption_key: &[u8],
    ) -> Result<LdapConfig, ApiError> {
        let mut inner = self.inner.lock();

        if inner.configs.contains_key(&tenant_id) {
            return Err(ApiError::Conflict(
                "LDAP configuration already exists for this tenant".to_string(),
            ));
        }

        let encrypted_password = encrypt(&create.bind_password, encryption_key)
            .map_err(|e| ApiError::Internal(format!("Failed to encrypt password: {}", e)))?;

        let config = LdapConfig {
            id: inner.next_id,
            tenant_id,
            ldap_url: create.ldap_url,
            bind_dn: create.bind_dn,
            bind_password_encrypted: encrypted_password,
            base_dn: create.base_dn,
            user_filter: create.user_filter,
            is_active: true,
            created_at: chrono::Utc::now(),
            updated_at: None,
        };

        inner.next_id += 1;
        inner.configs.insert(tenant_id, config.clone());
        Ok(config)
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Option<LdapConfig>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner.configs.get(&tenant_id).cloned())
    }

    async fn update(
        &self,
        tenant_id: i64,
        update: UpdateLdapConfig,
        encryption_key: &[u8],
    ) -> Result<LdapConfig, ApiError> {
        let mut inner = self.inner.lock();

        let config = inner
            .configs
            .get_mut(&tenant_id)
            .ok_or_else(|| ApiError::NotFound("LDAP configuration not found".to_string()))?;

        if let Some(ldap_url) = update.ldap_url {
            config.ldap_url = ldap_url;
        }
        if let Some(bind_dn) = update.bind_dn {
            config.bind_dn = bind_dn;
        }
        if let Some(bind_password) = update.bind_password {
            let encrypted = encrypt(&bind_password, encryption_key)
                .map_err(|e| ApiError::Internal(format!("Failed to encrypt password: {}", e)))?;
            config.bind_password_encrypted = encrypted;
        }
        if let Some(base_dn) = update.base_dn {
            config.base_dn = base_dn;
        }
        if let Some(user_filter) = update.user_filter {
            config.user_filter = user_filter;
        }
        if let Some(is_active) = update.is_active {
            config.is_active = is_active;
        }

        config.updated_at = Some(chrono::Utc::now());
        Ok(config.clone())
    }

    async fn delete(&self, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();

        if inner.configs.remove(&tenant_id).is_none() {
            return Err(ApiError::NotFound(
                "LDAP configuration not found".to_string(),
            ));
        }
        Ok(())
    }
}

/// Type alias for a boxed LDAP configuration repository
pub type BoxLdapConfigRepository = Arc<dyn LdapConfigRepository>;

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> [u8; 32] {
        [
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
            25, 26, 27, 28, 29, 30, 31, 32,
        ]
    }

    #[tokio::test]
    async fn test_create_and_find() {
        let repo = InMemoryLdapConfigRepository::new();
        let key = test_key();

        let create = CreateLdapConfig {
            ldap_url: "ldap://localhost:389".to_string(),
            bind_dn: "cn=admin,dc=example,dc=com".to_string(),
            bind_password: "secret123".to_string(),
            base_dn: "dc=example,dc=com".to_string(),
            user_filter: "(objectClass=person)".to_string(),
        };

        let config = repo.create(1, create, &key).await.unwrap();
        assert_eq!(config.id, 1);
        assert_eq!(config.tenant_id, 1);
        assert_ne!(config.bind_password_encrypted, "secret123"); // encrypted

        let found = repo.find_by_tenant(1).await.unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn test_duplicate_create_fails() {
        let repo = InMemoryLdapConfigRepository::new();
        let key = test_key();

        let create = CreateLdapConfig {
            ldap_url: "ldap://localhost:389".to_string(),
            bind_dn: "cn=admin,dc=example,dc=com".to_string(),
            bind_password: "secret123".to_string(),
            base_dn: "dc=example,dc=com".to_string(),
            user_filter: "(objectClass=person)".to_string(),
        };

        repo.create(1, create.clone(), &key).await.unwrap();
        let result = repo.create(1, create, &key).await;
        assert!(matches!(result, Err(ApiError::Conflict(_))));
    }

    #[tokio::test]
    async fn test_update() {
        let repo = InMemoryLdapConfigRepository::new();
        let key = test_key();

        let create = CreateLdapConfig {
            ldap_url: "ldap://localhost:389".to_string(),
            bind_dn: "cn=admin,dc=example,dc=com".to_string(),
            bind_password: "secret123".to_string(),
            base_dn: "dc=example,dc=com".to_string(),
            user_filter: "(objectClass=person)".to_string(),
        };

        repo.create(1, create, &key).await.unwrap();

        let update = UpdateLdapConfig {
            ldap_url: Some("ldap://newhost:636".to_string()),
            bind_password: Some("newpass".to_string()),
            ..Default::default()
        };

        let updated = repo.update(1, update, &key).await.unwrap();
        assert_eq!(updated.ldap_url, "ldap://newhost:636");
        assert_ne!(updated.bind_password_encrypted, "newpass"); // still encrypted
    }

    #[tokio::test]
    async fn test_delete() {
        let repo = InMemoryLdapConfigRepository::new();
        let key = test_key();

        let create = CreateLdapConfig {
            ldap_url: "ldap://localhost:389".to_string(),
            bind_dn: "cn=admin,dc=example,dc=com".to_string(),
            bind_password: "secret123".to_string(),
            base_dn: "dc=example,dc=com".to_string(),
            user_filter: "(objectClass=person)".to_string(),
        };

        repo.create(1, create, &key).await.unwrap();
        repo.delete(1).await.unwrap();

        let found = repo.find_by_tenant(1).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_delete_not_found() {
        let repo = InMemoryLdapConfigRepository::new();
        let result = repo.delete(99).await;
        assert!(matches!(result, Err(ApiError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_find_not_found() {
        let repo = InMemoryLdapConfigRepository::new();
        let found = repo.find_by_tenant(99).await.unwrap();
        assert!(found.is_none());
    }
}
