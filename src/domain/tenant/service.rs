//! Tenant service for business logic
#[allow(unused_imports)]
use crate::domain::tenant::model::Tenant;
use crate::domain::tenant::model::{CreateTenant, UpdateTenant};
use crate::domain::tenant::repository::BoxTenantRepository;
use crate::error::ApiError;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::tenant::repository::InMemoryTenantRepository;
    use std::sync::Arc;

    fn create_service() -> TenantService {
        let repo = Arc::new(InMemoryTenantRepository::new()) as BoxTenantRepository;
        TenantService::new(repo)
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
        assert!(result.len() >= 1);
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
}
