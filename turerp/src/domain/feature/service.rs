//! Feature Flag Service
//!
//! Business logic for feature flag management.

use std::sync::Arc;

use super::model::{CreateFeatureFlag, FeatureFlagResponse, FeatureFlagStatus, UpdateFeatureFlag};
use super::repository::FeatureFlagRepository;
use crate::common::pagination::PaginatedResult;
use crate::error::ApiError;

/// Feature flag service
pub struct FeatureFlagService {
    repository: Arc<dyn FeatureFlagRepository>,
}

impl FeatureFlagService {
    /// Create a new feature flag service
    pub fn new(repository: Arc<dyn FeatureFlagRepository>) -> Self {
        Self { repository }
    }

    /// Create a new feature flag
    pub async fn create(&self, flag: CreateFeatureFlag) -> Result<FeatureFlagResponse, ApiError> {
        // Check if flag already exists
        if self
            .repository
            .get_by_name(&flag.name, flag.tenant_id)
            .await?
            .is_some()
        {
            return Err(ApiError::Conflict(format!(
                "Feature flag '{}' already exists for this tenant",
                flag.name
            )));
        }

        let flag = self.repository.create(flag).await?;
        Ok(flag.into())
    }

    /// Get a feature flag by ID
    pub async fn get_by_id(&self, id: i64) -> Result<Option<FeatureFlagResponse>, ApiError> {
        let flag = self.repository.get_by_id(id).await?;
        Ok(flag.map(|f| f.into()))
    }

    /// Get a feature flag by name
    pub async fn get_by_name(
        &self,
        name: &str,
        tenant_id: Option<i64>,
    ) -> Result<Option<FeatureFlagResponse>, ApiError> {
        let flag = self.repository.get_by_name(name, tenant_id).await?;
        Ok(flag.map(|f| f.into()))
    }

    /// Get all feature flags
    pub async fn get_all(
        &self,
        tenant_id: Option<i64>,
    ) -> Result<Vec<FeatureFlagResponse>, ApiError> {
        let flags = self.repository.get_all(tenant_id).await?;
        Ok(flags.into_iter().map(|f| f.into()).collect())
    }

    /// Get all feature flags with pagination
    pub async fn get_all_paginated(
        &self,
        tenant_id: Option<i64>,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<FeatureFlagResponse>, ApiError> {
        let params = crate::common::pagination::PaginationParams { page, per_page };
        params.validate().map_err(ApiError::Validation)?;
        let result = self
            .repository
            .get_all_paginated(tenant_id, page, per_page)
            .await?;
        Ok(result.map(|f| f.into()))
    }

    /// Update a feature flag
    pub async fn update(
        &self,
        id: i64,
        flag: UpdateFeatureFlag,
    ) -> Result<Option<FeatureFlagResponse>, ApiError> {
        let updated = self.repository.update(id, flag).await?;
        Ok(updated.map(|f| f.into()))
    }

    /// Delete a feature flag
    pub async fn delete(&self, id: i64) -> Result<bool, ApiError> {
        let deleted = self.repository.delete(id).await?;
        if !deleted {
            return Err(ApiError::NotFound(format!(
                "Feature flag with id {} not found",
                id
            )));
        }
        Ok(deleted)
    }

    /// Soft delete a feature flag
    pub async fn soft_delete(&self, id: i64, deleted_by: i64) -> Result<bool, ApiError> {
        let deleted = self.repository.soft_delete(id, deleted_by).await?;
        if !deleted {
            return Err(ApiError::NotFound(format!(
                "Feature flag with id {} not found",
                id
            )));
        }
        Ok(deleted)
    }

    /// Restore a soft-deleted feature flag
    pub async fn restore(&self, id: i64) -> Result<bool, ApiError> {
        let restored = self.repository.restore(id).await?;
        if !restored {
            return Err(ApiError::NotFound(format!(
                "Deleted feature flag with id {} not found",
                id
            )));
        }
        Ok(restored)
    }

    /// List deleted feature flags
    pub async fn find_deleted(&self) -> Result<Vec<FeatureFlagResponse>, ApiError> {
        let flags = self.repository.find_deleted().await?;
        Ok(flags.into_iter().map(|f| f.into()).collect())
    }

    /// Permanently destroy a soft-deleted feature flag
    pub async fn destroy(&self, id: i64) -> Result<bool, ApiError> {
        let destroyed = self.repository.destroy(id).await?;
        if !destroyed {
            return Err(ApiError::NotFound(format!(
                "Deleted feature flag with id {} not found",
                id
            )));
        }
        Ok(destroyed)
    }

    /// Check if a feature is enabled
    pub async fn is_enabled(&self, name: &str, tenant_id: Option<i64>) -> Result<bool, ApiError> {
        self.repository.is_enabled(name, tenant_id).await
    }

    /// Enable a feature flag
    pub async fn enable(&self, id: i64) -> Result<Option<FeatureFlagResponse>, ApiError> {
        self.update(
            id,
            UpdateFeatureFlag {
                description: None,
                status: Some(FeatureFlagStatus::Enabled),
            },
        )
        .await
    }

    /// Disable a feature flag
    pub async fn disable(&self, id: i64) -> Result<Option<FeatureFlagResponse>, ApiError> {
        self.update(
            id,
            UpdateFeatureFlag {
                description: None,
                status: Some(FeatureFlagStatus::Disabled),
            },
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::feature::repository::InMemoryFeatureFlagRepository;

    #[actix_web::test]
    async fn test_create_feature_flag() {
        let repo = Arc::new(InMemoryFeatureFlagRepository::new());
        let service = FeatureFlagService::new(repo);

        let flag = service
            .create(CreateFeatureFlag {
                name: "new_feature".to_string(),
                description: Some("A new feature".to_string()),
                status: Some(FeatureFlagStatus::Enabled),
                tenant_id: None,
            })
            .await
            .unwrap();

        assert_eq!(flag.name, "new_feature");
        assert_eq!(flag.status, FeatureFlagStatus::Enabled);
    }

    #[actix_web::test]
    async fn test_create_duplicate_flag() {
        let repo = Arc::new(InMemoryFeatureFlagRepository::new());
        let service = FeatureFlagService::new(repo);

        service
            .create(CreateFeatureFlag {
                name: "duplicate".to_string(),
                description: None,
                status: None,
                tenant_id: None,
            })
            .await
            .unwrap();

        let result = service
            .create(CreateFeatureFlag {
                name: "duplicate".to_string(),
                description: None,
                status: None,
                tenant_id: None,
            })
            .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ApiError::Conflict(_)));
    }

    #[actix_web::test]
    async fn test_get_by_id() {
        let repo = Arc::new(InMemoryFeatureFlagRepository::new());
        let service = FeatureFlagService::new(repo);

        let created = service
            .create(CreateFeatureFlag {
                name: "test".to_string(),
                description: None,
                status: None,
                tenant_id: None,
            })
            .await
            .unwrap();

        let found = service.get_by_id(created.id).await.unwrap();
        assert!(found.is_some());

        let not_found = service.get_by_id(999).await.unwrap();
        assert!(not_found.is_none());
    }

    #[actix_web::test]
    async fn test_update_flag() {
        let repo = Arc::new(InMemoryFeatureFlagRepository::new());
        let service = FeatureFlagService::new(repo);

        let created = service
            .create(CreateFeatureFlag {
                name: "update_me".to_string(),
                description: Some("Original".to_string()),
                status: Some(FeatureFlagStatus::Disabled),
                tenant_id: None,
            })
            .await
            .unwrap();

        let updated = service
            .update(
                created.id,
                UpdateFeatureFlag {
                    description: Some("Updated".to_string()),
                    status: Some(FeatureFlagStatus::Enabled),
                },
            )
            .await
            .unwrap();

        assert!(updated.is_some());
        let updated = updated.unwrap();
        assert_eq!(updated.description, Some("Updated".to_string()));
        assert_eq!(updated.status, FeatureFlagStatus::Enabled);
    }

    #[actix_web::test]
    async fn test_delete_flag() {
        let repo = Arc::new(InMemoryFeatureFlagRepository::new());
        let service = FeatureFlagService::new(repo);

        let created = service
            .create(CreateFeatureFlag {
                name: "delete_me".to_string(),
                description: None,
                status: None,
                tenant_id: None,
            })
            .await
            .unwrap();

        let deleted = service.delete(created.id).await.unwrap();
        assert!(deleted);

        let not_found = service.get_by_id(created.id).await.unwrap();
        assert!(not_found.is_none());
    }

    #[actix_web::test]
    async fn test_is_enabled() {
        let repo = Arc::new(InMemoryFeatureFlagRepository::new());
        let service = FeatureFlagService::new(repo);

        service
            .create(CreateFeatureFlag {
                name: "enabled_feature".to_string(),
                description: None,
                status: Some(FeatureFlagStatus::Enabled),
                tenant_id: None,
            })
            .await
            .unwrap();

        service
            .create(CreateFeatureFlag {
                name: "disabled_feature".to_string(),
                description: None,
                status: Some(FeatureFlagStatus::Disabled),
                tenant_id: None,
            })
            .await
            .unwrap();

        assert!(service.is_enabled("enabled_feature", None).await.unwrap());
        assert!(!service.is_enabled("disabled_feature", None).await.unwrap());
        assert!(!service.is_enabled("nonexistent", None).await.unwrap());
    }

    #[actix_web::test]
    async fn test_enable_disable() {
        let repo = Arc::new(InMemoryFeatureFlagRepository::new());
        let service = FeatureFlagService::new(repo);

        let created = service
            .create(CreateFeatureFlag {
                name: "toggle_me".to_string(),
                description: None,
                status: Some(FeatureFlagStatus::Disabled),
                tenant_id: None,
            })
            .await
            .unwrap();

        assert_eq!(created.status, FeatureFlagStatus::Disabled);

        let enabled = service.enable(created.id).await.unwrap().unwrap();
        assert_eq!(enabled.status, FeatureFlagStatus::Enabled);

        let disabled = service.disable(created.id).await.unwrap().unwrap();
        assert_eq!(disabled.status, FeatureFlagStatus::Disabled);
    }
}
