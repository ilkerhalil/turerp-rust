//! Tenant repository

use async_trait::async_trait;
use parking_lot::Mutex;
use std::sync::Arc;

use crate::common::pagination::PaginatedResult;
use crate::domain::tenant::model::{
    CreateTenant, CreateTenantConfig, Tenant, TenantConfig, UpdateTenant, UpdateTenantConfig,
};
use crate::error::ApiError;

/// Repository trait for Tenant operations
#[async_trait]
pub trait TenantRepository: Send + Sync {
    /// Create a new tenant
    async fn create(&self, tenant: CreateTenant) -> Result<Tenant, ApiError>;

    /// Find tenant by ID
    async fn find_by_id(&self, id: i64) -> Result<Option<Tenant>, ApiError>;

    /// Find tenant by subdomain
    async fn find_by_subdomain(&self, subdomain: &str) -> Result<Option<Tenant>, ApiError>;

    /// Find all tenants
    async fn find_all(&self) -> Result<Vec<Tenant>, ApiError>;

    /// Find all tenants with pagination
    async fn find_all_paginated(
        &self,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Tenant>, ApiError>;

    /// Update a tenant
    async fn update(&self, id: i64, tenant: UpdateTenant) -> Result<Tenant, ApiError>;

    /// Delete a tenant
    async fn delete(&self, id: i64) -> Result<(), ApiError>;

    /// Check if subdomain exists
    async fn subdomain_exists(&self, subdomain: &str) -> Result<bool, ApiError>;
}

/// Repository trait for TenantConfig operations
#[async_trait]
pub trait TenantConfigRepository: Send + Sync {
    /// Create or update a config entry
    async fn set(&self, config: CreateTenantConfig) -> Result<TenantConfig, ApiError>;

    /// Get a config value by key
    async fn get(&self, tenant_id: i64, key: &str) -> Result<Option<TenantConfig>, ApiError>;

    /// Get all config entries for a tenant
    async fn get_all(&self, tenant_id: i64) -> Result<Vec<TenantConfig>, ApiError>;

    /// Update a config entry
    async fn update(&self, id: i64, update: UpdateTenantConfig) -> Result<TenantConfig, ApiError>;

    /// Delete a config entry
    async fn delete(&self, id: i64) -> Result<(), ApiError>;

    /// Delete all config entries for a tenant
    async fn delete_by_tenant(&self, tenant_id: i64) -> Result<(), ApiError>;
}

/// Type aliases
pub type BoxTenantRepository = Arc<dyn TenantRepository>;
pub type BoxTenantConfigRepository = Arc<dyn TenantConfigRepository>;

/// Inner state for InMemoryTenantRepository
struct InMemoryTenantInner {
    tenants: std::collections::HashMap<i64, Tenant>,
    next_id: i64,
}

/// In-memory tenant repository for testing
pub struct InMemoryTenantRepository {
    inner: Mutex<InMemoryTenantInner>,
}

impl InMemoryTenantRepository {
    pub fn new() -> Self {
        let repo = Self {
            inner: Mutex::new(InMemoryTenantInner {
                tenants: std::collections::HashMap::new(),
                next_id: 1,
            }),
        };

        // Add a default tenant
        let default_tenant = Tenant {
            id: 1,
            name: "Default Tenant".to_string(),
            subdomain: "default".to_string(),
            db_name: "turerp_default".to_string(),
            is_active: true,
            created_at: chrono::Utc::now(),
        };
        repo.inner.lock().tenants.insert(1, default_tenant);

        repo
    }
}

impl Default for InMemoryTenantRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TenantRepository for InMemoryTenantRepository {
    async fn create(&self, create: CreateTenant) -> Result<Tenant, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_id;
        inner.next_id += 1;

        let db_name = crate::domain::tenant::model::generate_db_name(&create.subdomain);

        let new_tenant = Tenant {
            id,
            name: create.name,
            subdomain: create.subdomain,
            db_name,
            is_active: true,
            created_at: chrono::Utc::now(),
        };

        inner.tenants.insert(id, new_tenant.clone());
        Ok(new_tenant)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Tenant>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner.tenants.get(&id).cloned())
    }

    async fn find_by_subdomain(&self, subdomain: &str) -> Result<Option<Tenant>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .tenants
            .values()
            .find(|t| t.subdomain == subdomain)
            .cloned())
    }

    async fn find_all(&self) -> Result<Vec<Tenant>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner.tenants.values().cloned().collect())
    }

    async fn find_all_paginated(
        &self,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Tenant>, ApiError> {
        let inner = self.inner.lock();
        let all: Vec<_> = inner.tenants.values().cloned().collect();
        let total = all.len() as u64;
        let items: Vec<_> = all
            .into_iter()
            .skip(((page.saturating_sub(1)) * per_page) as usize)
            .take(per_page as usize)
            .collect();
        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn update(&self, id: i64, update: UpdateTenant) -> Result<Tenant, ApiError> {
        let mut inner = self.inner.lock();

        let tenant = inner
            .tenants
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Tenant {} not found", id)))?;

        if let Some(name) = update.name {
            tenant.name = name;
        }
        if let Some(subdomain) = update.subdomain {
            tenant.subdomain = subdomain.clone();
            tenant.db_name = crate::domain::tenant::model::generate_db_name(&subdomain);
        }
        if let Some(is_active) = update.is_active {
            tenant.is_active = is_active;
        }

        Ok(tenant.clone())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();

        if !inner.tenants.contains_key(&id) {
            return Err(ApiError::NotFound(format!("Tenant {} not found", id)));
        }

        inner.tenants.remove(&id);
        Ok(())
    }

    async fn subdomain_exists(&self, subdomain: &str) -> Result<bool, ApiError> {
        let inner = self.inner.lock();
        Ok(inner.tenants.values().any(|t| t.subdomain == subdomain))
    }
}

/// Inner state for InMemoryTenantConfigRepository
struct InMemoryTenantConfigInner {
    configs: std::collections::HashMap<i64, TenantConfig>,
    next_id: i64,
    tenant_configs: std::collections::HashMap<i64, Vec<i64>>,
}

/// In-memory tenant config repository for testing
pub struct InMemoryTenantConfigRepository {
    inner: Mutex<InMemoryTenantConfigInner>,
}

impl InMemoryTenantConfigRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryTenantConfigInner {
                configs: std::collections::HashMap::new(),
                next_id: 1,
                tenant_configs: std::collections::HashMap::new(),
            }),
        }
    }
}

impl Default for InMemoryTenantConfigRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TenantConfigRepository for InMemoryTenantConfigRepository {
    async fn set(&self, create: CreateTenantConfig) -> Result<TenantConfig, ApiError> {
        // Check if key already exists for this tenant
        let existing_id = {
            let inner = self.inner.lock();
            inner
                .configs
                .values()
                .find(|c| c.tenant_id == create.tenant_id && c.key == create.key)
                .map(|c| c.id)
        };

        if let Some(id) = existing_id {
            // Update existing config
            let mut inner = self.inner.lock();
            let config = inner
                .configs
                .get_mut(&id)
                .ok_or_else(|| ApiError::NotFound(format!("Config {} not found", id)))?;
            config.value = create.value;
            if let Some(is_encrypted) = create.is_encrypted {
                config.is_encrypted = is_encrypted;
            }
            config.updated_at = chrono::Utc::now();
            Ok(config.clone())
        } else {
            // Create new config
            let mut inner = self.inner.lock();
            let id = inner.next_id;
            inner.next_id += 1;

            let now = chrono::Utc::now();
            let config = TenantConfig {
                id,
                tenant_id: create.tenant_id,
                key: create.key,
                value: create.value,
                is_encrypted: create.is_encrypted.unwrap_or(false),
                created_at: now,
                updated_at: now,
            };

            inner.configs.insert(id, config.clone());
            inner
                .tenant_configs
                .entry(create.tenant_id)
                .or_default()
                .push(id);

            Ok(config)
        }
    }

    async fn get(&self, tenant_id: i64, key: &str) -> Result<Option<TenantConfig>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .configs
            .values()
            .find(|c| c.tenant_id == tenant_id && c.key == key)
            .cloned())
    }

    async fn get_all(&self, tenant_id: i64) -> Result<Vec<TenantConfig>, ApiError> {
        let inner = self.inner.lock();

        let ids = inner
            .tenant_configs
            .get(&tenant_id)
            .cloned()
            .unwrap_or_default();
        Ok(ids
            .iter()
            .filter_map(|id| inner.configs.get(id).cloned())
            .collect())
    }

    async fn update(&self, id: i64, update: UpdateTenantConfig) -> Result<TenantConfig, ApiError> {
        let mut inner = self.inner.lock();

        let config = inner
            .configs
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Config {} not found", id)))?;

        if let Some(value) = update.value {
            config.value = value;
        }
        if let Some(is_encrypted) = update.is_encrypted {
            config.is_encrypted = is_encrypted;
        }
        config.updated_at = chrono::Utc::now();

        Ok(config.clone())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();

        if !inner.configs.contains_key(&id) {
            return Err(ApiError::NotFound(format!("Config {} not found", id)));
        }

        let tenant_id = inner.configs.get(&id).map(|c| c.tenant_id);
        inner.configs.remove(&id);

        if let Some(tid) = tenant_id {
            if let Some(ids) = inner.tenant_configs.get_mut(&tid) {
                ids.retain(|x| *x != id);
            }
        }

        Ok(())
    }

    async fn delete_by_tenant(&self, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();

        let ids = inner.tenant_configs.remove(&tenant_id).unwrap_or_default();
        for id in ids {
            inner.configs.remove(&id);
        }

        Ok(())
    }
}
