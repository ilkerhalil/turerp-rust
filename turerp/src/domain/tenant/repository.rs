//! Tenant repository

use async_trait::async_trait;
use std::sync::Arc;

use crate::domain::tenant::model::{CreateTenant, Tenant, UpdateTenant};
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

    /// Update a tenant
    async fn update(&self, id: i64, tenant: UpdateTenant) -> Result<Tenant, ApiError>;

    /// Delete a tenant
    async fn delete(&self, id: i64) -> Result<(), ApiError>;

    /// Check if subdomain exists
    async fn subdomain_exists(&self, subdomain: &str) -> Result<bool, ApiError>;
}

/// Type alias for boxed repository
pub type BoxTenantRepository = Arc<dyn TenantRepository>;

/// In-memory tenant repository for testing
pub struct InMemoryTenantRepository {
    tenants: std::sync::Mutex<std::collections::HashMap<i64, Tenant>>,
    next_id: std::sync::Mutex<i64>,
}

impl InMemoryTenantRepository {
    pub fn new() -> Self {
        let repo = Self {
            tenants: std::sync::Mutex::new(std::collections::HashMap::new()),
            next_id: std::sync::Mutex::new(1),
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
        repo.tenants.lock().unwrap().insert(1, default_tenant);

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
        let mut next_id = self.next_id.lock().unwrap();
        let id = *next_id;
        *next_id += 1;

        let db_name = crate::domain::tenant::model::generate_db_name(&create.subdomain);

        let new_tenant = Tenant {
            id,
            name: create.name,
            subdomain: create.subdomain,
            db_name,
            is_active: true,
            created_at: chrono::Utc::now(),
        };

        self.tenants.lock().unwrap().insert(id, new_tenant.clone());
        Ok(new_tenant)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Tenant>, ApiError> {
        let tenants = self.tenants.lock().unwrap();
        Ok(tenants.get(&id).cloned())
    }

    async fn find_by_subdomain(&self, subdomain: &str) -> Result<Option<Tenant>, ApiError> {
        let tenants = self.tenants.lock().unwrap();
        Ok(tenants.values().find(|t| t.subdomain == subdomain).cloned())
    }

    async fn find_all(&self) -> Result<Vec<Tenant>, ApiError> {
        let tenants = self.tenants.lock().unwrap();
        Ok(tenants.values().cloned().collect())
    }

    async fn update(&self, id: i64, update: UpdateTenant) -> Result<Tenant, ApiError> {
        let mut tenants = self.tenants.lock().unwrap();

        let tenant = tenants
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
        let mut tenants = self.tenants.lock().unwrap();

        if !tenants.contains_key(&id) {
            return Err(ApiError::NotFound(format!("Tenant {} not found", id)));
        }

        tenants.remove(&id);
        Ok(())
    }

    async fn subdomain_exists(&self, subdomain: &str) -> Result<bool, ApiError> {
        let tenants = self.tenants.lock().unwrap();
        Ok(tenants.values().any(|t| t.subdomain == subdomain))
    }
}
