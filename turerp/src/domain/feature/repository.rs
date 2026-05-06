//! Feature Flag Repository
//!
//! Trait defining the repository interface for feature flags.

use async_trait::async_trait;

use super::model::{FeatureFlag, FeatureFlagStatus};
use crate::common::pagination::PaginatedResult;
use crate::common::soft_delete::SoftDeletable;

/// Feature flag repository trait
#[async_trait]
pub trait FeatureFlagRepository: Send + Sync {
    /// Create a new feature flag
    async fn create(
        &self,
        flag: super::model::CreateFeatureFlag,
    ) -> Result<FeatureFlag, crate::error::ApiError>;

    /// Get a feature flag by ID
    async fn get_by_id(&self, id: i64) -> Result<Option<FeatureFlag>, crate::error::ApiError>;

    /// Get a feature flag by name (optionally filtered by tenant)
    async fn get_by_name(
        &self,
        name: &str,
        tenant_id: Option<i64>,
    ) -> Result<Option<FeatureFlag>, crate::error::ApiError>;

    /// Get all feature flags (optionally filtered by tenant)
    async fn get_all(
        &self,
        tenant_id: Option<i64>,
    ) -> Result<Vec<FeatureFlag>, crate::error::ApiError>;

    /// Get all feature flags with pagination (optionally filtered by tenant)
    async fn get_all_paginated(
        &self,
        tenant_id: Option<i64>,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<FeatureFlag>, crate::error::ApiError>;

    /// Update a feature flag
    async fn update(
        &self,
        id: i64,
        flag: super::model::UpdateFeatureFlag,
    ) -> Result<Option<FeatureFlag>, crate::error::ApiError>;

    /// Delete a feature flag
    async fn delete(&self, id: i64) -> Result<bool, crate::error::ApiError>;

    /// Soft delete a feature flag
    async fn soft_delete(&self, id: i64, deleted_by: i64) -> Result<bool, crate::error::ApiError>;

    /// Restore a soft-deleted feature flag
    async fn restore(&self, id: i64) -> Result<bool, crate::error::ApiError>;

    /// List deleted feature flags
    async fn find_deleted(&self) -> Result<Vec<FeatureFlag>, crate::error::ApiError>;

    /// Permanently destroy a soft-deleted feature flag
    async fn destroy(&self, id: i64) -> Result<bool, crate::error::ApiError>;

    /// Check if a feature flag is enabled (looks up by name, with tenant override)
    async fn is_enabled(
        &self,
        name: &str,
        tenant_id: Option<i64>,
    ) -> Result<bool, crate::error::ApiError>;
}

/// In-memory feature flag repository for development/testing
pub struct InMemoryFeatureFlagRepository {
    flags: std::sync::Arc<tokio::sync::RwLock<Vec<FeatureFlag>>>,
    next_id: std::sync::Arc<tokio::sync::RwLock<i64>>,
}

impl InMemoryFeatureFlagRepository {
    pub fn new() -> Self {
        Self {
            flags: std::sync::Arc::new(tokio::sync::RwLock::new(Vec::new())),
            next_id: std::sync::Arc::new(tokio::sync::RwLock::new(1)),
        }
    }
}

impl Default for InMemoryFeatureFlagRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FeatureFlagRepository for InMemoryFeatureFlagRepository {
    async fn create(
        &self,
        flag: super::model::CreateFeatureFlag,
    ) -> Result<FeatureFlag, crate::error::ApiError> {
        let mut next_id = self.next_id.write().await;
        let id = *next_id;
        *next_id += 1;
        drop(next_id);

        let now = chrono::Utc::now().naive_utc();
        let feature_flag = FeatureFlag {
            id,
            name: flag.name,
            description: flag.description,
            status: flag.status.unwrap_or(FeatureFlagStatus::Disabled),
            tenant_id: flag.tenant_id,
            created_at: now,
            updated_at: now,
            deleted_at: None,
            deleted_by: None,
        };

        let mut flags = self.flags.write().await;
        flags.push(feature_flag.clone());
        Ok(feature_flag)
    }

    async fn get_by_id(&self, id: i64) -> Result<Option<FeatureFlag>, crate::error::ApiError> {
        let flags = self.flags.read().await;
        Ok(flags
            .iter()
            .find(|f| f.id == id && !f.is_deleted())
            .cloned())
    }

    async fn get_by_name(
        &self,
        name: &str,
        tenant_id: Option<i64>,
    ) -> Result<Option<FeatureFlag>, crate::error::ApiError> {
        let flags = self.flags.read().await;
        Ok(flags
            .iter()
            .find(|f| f.name == name && f.tenant_id == tenant_id && !f.is_deleted())
            .cloned())
    }

    async fn get_all(
        &self,
        tenant_id: Option<i64>,
    ) -> Result<Vec<FeatureFlag>, crate::error::ApiError> {
        let flags = self.flags.read().await;
        let result: Vec<FeatureFlag> = flags
            .iter()
            .filter(|f| {
                !f.is_deleted()
                    && (tenant_id.is_none() || f.tenant_id == tenant_id || f.tenant_id.is_none())
            })
            .cloned()
            .collect();
        Ok(result)
    }

    async fn get_all_paginated(
        &self,
        tenant_id: Option<i64>,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<FeatureFlag>, crate::error::ApiError> {
        let flags = self.flags.read().await;
        let mut filtered: Vec<FeatureFlag> = flags
            .iter()
            .filter(|f| {
                !f.is_deleted()
                    && (tenant_id.is_none() || f.tenant_id == tenant_id || f.tenant_id.is_none())
            })
            .cloned()
            .collect();
        filtered.sort_by_key(|f| f.id);
        let total = filtered.len() as u64;
        let items: Vec<FeatureFlag> = filtered
            .into_iter()
            .skip((page.saturating_sub(1) as usize) * (per_page as usize))
            .take(per_page as usize)
            .collect();
        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn update(
        &self,
        id: i64,
        flag: super::model::UpdateFeatureFlag,
    ) -> Result<Option<FeatureFlag>, crate::error::ApiError> {
        let mut flags = self.flags.write().await;
        if let Some(existing) = flags.iter_mut().find(|f| f.id == id && !f.is_deleted()) {
            if let Some(description) = flag.description {
                existing.description = Some(description);
            }
            if let Some(status) = flag.status {
                existing.status = status;
            }
            existing.updated_at = chrono::Utc::now().naive_utc();
            Ok(Some(existing.clone()))
        } else {
            Ok(None)
        }
    }

    async fn delete(&self, id: i64) -> Result<bool, crate::error::ApiError> {
        let mut flags = self.flags.write().await;
        let initial_len = flags.len();
        flags.retain(|f| f.id != id);
        Ok(flags.len() < initial_len)
    }

    async fn is_enabled(
        &self,
        name: &str,
        tenant_id: Option<i64>,
    ) -> Result<bool, crate::error::ApiError> {
        let flags = self.flags.read().await;

        // First check for tenant-specific flag
        if let Some(tenant_id) = tenant_id {
            if let Some(flag) = flags
                .iter()
                .find(|f| f.name == name && f.tenant_id == Some(tenant_id) && !f.is_deleted())
            {
                return Ok(flag.status == FeatureFlagStatus::Enabled);
            }
        }

        // Fall back to global flag
        if let Some(flag) = flags
            .iter()
            .find(|f| f.name == name && f.tenant_id.is_none() && !f.is_deleted())
        {
            Ok(flag.status == FeatureFlagStatus::Enabled)
        } else {
            // Default to disabled if flag doesn't exist
            Ok(false)
        }
    }

    async fn soft_delete(&self, id: i64, deleted_by: i64) -> Result<bool, crate::error::ApiError> {
        let mut flags = self.flags.write().await;
        if let Some(flag) = flags.iter_mut().find(|f| f.id == id && !f.is_deleted()) {
            flag.mark_deleted(deleted_by);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn restore(&self, id: i64) -> Result<bool, crate::error::ApiError> {
        let mut flags = self.flags.write().await;
        if let Some(flag) = flags.iter_mut().find(|f| f.id == id && f.is_deleted()) {
            flag.restore();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn find_deleted(&self) -> Result<Vec<FeatureFlag>, crate::error::ApiError> {
        let flags = self.flags.read().await;
        Ok(flags.iter().filter(|f| f.is_deleted()).cloned().collect())
    }

    async fn destroy(&self, id: i64) -> Result<bool, crate::error::ApiError> {
        let mut flags = self.flags.write().await;
        let initial_len = flags.len();
        flags.retain(|f| !(f.id == id && f.is_deleted()));
        Ok(flags.len() < initial_len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::feature::model::CreateFeatureFlag;
    use crate::domain::feature::model::UpdateFeatureFlag;

    #[actix_web::test]
    async fn test_create_feature_flag() {
        let repo = InMemoryFeatureFlagRepository::new();
        let flag = repo
            .create(CreateFeatureFlag {
                name: "new_feature".to_string(),
                description: Some("A new feature".to_string()),
                status: Some(FeatureFlagStatus::Enabled),
                tenant_id: None,
            })
            .await
            .unwrap();

        assert_eq!(flag.id, 1);
        assert_eq!(flag.name, "new_feature");
        assert_eq!(flag.status, FeatureFlagStatus::Enabled);
    }

    #[actix_web::test]
    async fn test_get_by_id() {
        let repo = InMemoryFeatureFlagRepository::new();
        let created = repo
            .create(CreateFeatureFlag {
                name: "test_feature".to_string(),
                description: None,
                status: None,
                tenant_id: None,
            })
            .await
            .unwrap();

        let found = repo.get_by_id(created.id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "test_feature");

        let not_found = repo.get_by_id(999).await.unwrap();
        assert!(not_found.is_none());
    }

    #[actix_web::test]
    async fn test_get_by_name() {
        let repo = InMemoryFeatureFlagRepository::new();
        repo.create(CreateFeatureFlag {
            name: "my_feature".to_string(),
            description: None,
            status: Some(FeatureFlagStatus::Enabled),
            tenant_id: None,
        })
        .await
        .unwrap();

        let found = repo.get_by_name("my_feature", None).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().status, FeatureFlagStatus::Enabled);

        let not_found = repo.get_by_name("nonexistent", None).await.unwrap();
        assert!(not_found.is_none());
    }

    #[actix_web::test]
    async fn test_update_feature_flag() {
        let repo = InMemoryFeatureFlagRepository::new();
        let created = repo
            .create(CreateFeatureFlag {
                name: "update_me".to_string(),
                description: Some("Original description".to_string()),
                status: Some(FeatureFlagStatus::Disabled),
                tenant_id: None,
            })
            .await
            .unwrap();

        let updated = repo
            .update(
                created.id,
                UpdateFeatureFlag {
                    description: Some("Updated description".to_string()),
                    status: Some(FeatureFlagStatus::Enabled),
                },
            )
            .await
            .unwrap();

        assert!(updated.is_some());
        let updated = updated.unwrap();
        assert_eq!(updated.description, Some("Updated description".to_string()));
        assert_eq!(updated.status, FeatureFlagStatus::Enabled);
    }

    #[actix_web::test]
    async fn test_delete_feature_flag() {
        let repo = InMemoryFeatureFlagRepository::new();
        let created = repo
            .create(CreateFeatureFlag {
                name: "delete_me".to_string(),
                description: None,
                status: None,
                tenant_id: None,
            })
            .await
            .unwrap();

        let deleted = repo.delete(created.id).await.unwrap();
        assert!(deleted);

        let not_found = repo.get_by_id(created.id).await.unwrap();
        assert!(not_found.is_none());
    }

    #[actix_web::test]
    async fn test_is_enabled() {
        let repo = InMemoryFeatureFlagRepository::new();

        // Create global enabled flag
        repo.create(CreateFeatureFlag {
            name: "global_feature".to_string(),
            description: None,
            status: Some(FeatureFlagStatus::Enabled),
            tenant_id: None,
        })
        .await
        .unwrap();

        // Create tenant-specific disabled flag
        repo.create(CreateFeatureFlag {
            name: "tenant_feature".to_string(),
            description: None,
            status: Some(FeatureFlagStatus::Disabled),
            tenant_id: Some(1),
        })
        .await
        .unwrap();

        // Check global flag
        assert!(repo.is_enabled("global_feature", None).await.unwrap());
        assert!(repo.is_enabled("global_feature", Some(1)).await.unwrap());

        // Check tenant-specific flag
        assert!(!repo.is_enabled("tenant_feature", Some(1)).await.unwrap());

        // Check nonexistent flag
        assert!(!repo.is_enabled("nonexistent", None).await.unwrap());
    }

    #[actix_web::test]
    async fn test_tenant_override() {
        let repo = InMemoryFeatureFlagRepository::new();

        // Create global enabled flag
        repo.create(CreateFeatureFlag {
            name: "feature".to_string(),
            description: None,
            status: Some(FeatureFlagStatus::Enabled),
            tenant_id: None,
        })
        .await
        .unwrap();

        // Create tenant-specific disabled override
        repo.create(CreateFeatureFlag {
            name: "feature".to_string(),
            description: None,
            status: Some(FeatureFlagStatus::Disabled),
            tenant_id: Some(1),
        })
        .await
        .unwrap();

        // Tenant 1 sees disabled (override)
        assert!(!repo.is_enabled("feature", Some(1)).await.unwrap());

        // Tenant 2 sees enabled (global)
        assert!(repo.is_enabled("feature", Some(2)).await.unwrap());

        // No tenant sees enabled (global)
        assert!(repo.is_enabled("feature", None).await.unwrap());
    }
}
