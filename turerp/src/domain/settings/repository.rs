//! Settings repository

use async_trait::async_trait;
use parking_lot::Mutex;
use std::sync::Arc;

use crate::common::pagination::PaginatedResult;
use crate::common::soft_delete::SoftDeletable;
use crate::domain::settings::model::{
    BulkUpdateSettingItem, CreateSetting, Setting, UpdateSetting,
};
use crate::error::ApiError;

/// Repository trait for Settings operations
#[async_trait]
pub trait SettingsRepository: Send + Sync {
    /// Create a new setting
    async fn create(&self, setting: CreateSetting) -> Result<Setting, ApiError>;

    /// Find setting by ID
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Setting>, ApiError>;

    /// Find setting by key
    async fn find_by_key(&self, tenant_id: i64, key: &str) -> Result<Option<Setting>, ApiError>;

    /// Find all settings for a tenant with optional group filter
    async fn find_all(&self, tenant_id: i64, group: Option<&str>)
        -> Result<Vec<Setting>, ApiError>;

    /// Find all settings with pagination
    async fn find_all_paginated(
        &self,
        tenant_id: i64,
        group: Option<&str>,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Setting>, ApiError>;

    /// Update a setting
    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateSetting,
    ) -> Result<Setting, ApiError>;

    /// Delete a setting
    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;

    /// Delete a setting by key
    async fn delete_by_key(&self, tenant_id: i64, key: &str) -> Result<(), ApiError>;

    /// Bulk update settings by key
    async fn bulk_update(
        &self,
        tenant_id: i64,
        updates: Vec<BulkUpdateSettingItem>,
    ) -> Result<Vec<Setting>, ApiError>;

    /// Check if a key exists for a tenant
    async fn key_exists(&self, tenant_id: i64, key: &str) -> Result<bool, ApiError>;

    /// Soft delete a setting
    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError>;

    /// Restore a soft-deleted setting
    async fn restore(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;

    /// List deleted settings for a tenant
    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Setting>, ApiError>;

    /// Permanently destroy a soft-deleted setting
    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
}

/// Type alias for boxed SettingsRepository
pub type BoxSettingsRepository = Arc<dyn SettingsRepository>;

struct InMemoryInner {
    settings: std::collections::HashMap<i64, Setting>,
    next_id: i64,
    tenant_keys: std::collections::HashMap<(i64, String), i64>,
}

/// In-memory settings repository for testing and development
pub struct InMemorySettingsRepository {
    inner: Mutex<InMemoryInner>,
}

impl InMemorySettingsRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryInner {
                settings: std::collections::HashMap::new(),
                next_id: 1,
                tenant_keys: std::collections::HashMap::new(),
            }),
        }
    }
}

impl Default for InMemorySettingsRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SettingsRepository for InMemorySettingsRepository {
    async fn create(&self, create: CreateSetting) -> Result<Setting, ApiError> {
        let mut inner = self.inner.lock();

        if inner
            .tenant_keys
            .contains_key(&(create.tenant_id, create.key.clone()))
        {
            return Err(ApiError::Conflict(format!(
                "Setting '{}' already exists for tenant {}",
                create.key, create.tenant_id
            )));
        }

        let id = inner.next_id;
        inner.next_id += 1;
        let now = chrono::Utc::now();

        let setting = Setting {
            id,
            tenant_id: create.tenant_id,
            key: create.key.clone(),
            value: create.value,
            default_value: create.default_value,
            data_type: create.data_type,
            group: create.group,
            description: create.description,
            is_sensitive: create.is_sensitive,
            is_editable: create.is_editable,
            created_at: now,
            updated_at: now,
            deleted_at: None,
            deleted_by: None,
        };

        inner.settings.insert(id, setting.clone());
        inner.tenant_keys.insert((create.tenant_id, create.key), id);

        Ok(setting)
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Setting>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .settings
            .get(&id)
            .filter(|s| s.tenant_id == tenant_id)
            .cloned())
    }

    async fn find_by_key(&self, tenant_id: i64, key: &str) -> Result<Option<Setting>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .tenant_keys
            .get(&(tenant_id, key.to_string()))
            .and_then(|id| inner.settings.get(id))
            .filter(|s| !s.is_deleted())
            .cloned())
    }

    async fn find_all(
        &self,
        tenant_id: i64,
        group: Option<&str>,
    ) -> Result<Vec<Setting>, ApiError> {
        let inner = self.inner.lock();
        let items: Vec<Setting> = inner
            .settings
            .values()
            .filter(|s| s.tenant_id == tenant_id && !s.is_deleted())
            .filter(|s| {
                if let Some(g) = group {
                    s.group.to_string() == g.to_lowercase()
                } else {
                    true
                }
            })
            .cloned()
            .collect();
        Ok(items)
    }

    async fn find_all_paginated(
        &self,
        tenant_id: i64,
        group: Option<&str>,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Setting>, ApiError> {
        let inner = self.inner.lock();
        let mut items: Vec<Setting> = inner
            .settings
            .values()
            .filter(|s| s.tenant_id == tenant_id && !s.is_deleted())
            .filter(|s| {
                if let Some(g) = group {
                    s.group.to_string() == g.to_lowercase()
                } else {
                    true
                }
            })
            .cloned()
            .collect();

        items.sort_by(|a, b| a.key.cmp(&b.key));
        let total = items.len() as u64;
        let start = ((page.saturating_sub(1)) * per_page) as usize;
        let paginated = items
            .into_iter()
            .skip(start)
            .take(per_page as usize)
            .collect();
        Ok(PaginatedResult::new(paginated, page, per_page, total))
    }

    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateSetting,
    ) -> Result<Setting, ApiError> {
        let mut inner = self.inner.lock();

        let setting = inner
            .settings
            .get_mut(&id)
            .filter(|s| s.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Setting {} not found", id)))?;

        if let Some(value) = update.value {
            setting.value = value;
        }
        if let Some(default_value) = update.default_value {
            setting.default_value = default_value;
        }
        if let Some(description) = update.description {
            setting.description = description;
        }
        if let Some(is_sensitive) = update.is_sensitive {
            setting.is_sensitive = is_sensitive;
        }
        if let Some(is_editable) = update.is_editable {
            setting.is_editable = is_editable;
        }
        setting.updated_at = chrono::Utc::now();

        Ok(setting.clone())
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();

        let setting = inner
            .settings
            .get(&id)
            .filter(|s| s.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Setting {} not found", id)))?;

        let key = setting.key.clone();
        inner.settings.remove(&id);
        inner.tenant_keys.remove(&(tenant_id, key));

        Ok(())
    }

    async fn delete_by_key(&self, tenant_id: i64, key: &str) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();

        if let Some(id) = inner.tenant_keys.remove(&(tenant_id, key.to_string())) {
            inner.settings.remove(&id);
        }

        Ok(())
    }

    async fn bulk_update(
        &self,
        tenant_id: i64,
        updates: Vec<BulkUpdateSettingItem>,
    ) -> Result<Vec<Setting>, ApiError> {
        let mut updated = Vec::new();
        let mut inner = self.inner.lock();

        for item in updates {
            if let Some(&id) = inner.tenant_keys.get(&(tenant_id, item.key.clone())) {
                if let Some(setting) = inner.settings.get_mut(&id) {
                    setting.value = item.value;
                    setting.updated_at = chrono::Utc::now();
                    updated.push(setting.clone());
                }
            }
        }

        Ok(updated)
    }

    async fn key_exists(&self, tenant_id: i64, key: &str) -> Result<bool, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .tenant_keys
            .get(&(tenant_id, key.to_string()))
            .and_then(|id| inner.settings.get(id))
            .map(|s| !s.is_deleted())
            .unwrap_or(false))
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let setting = inner
            .settings
            .get_mut(&id)
            .filter(|s| s.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Setting {} not found", id)))?;

        if setting.is_deleted() {
            return Err(ApiError::Conflict(format!(
                "Setting {} is already deleted",
                id
            )));
        }

        setting.mark_deleted(deleted_by);
        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let setting = inner
            .settings
            .get_mut(&id)
            .filter(|s| s.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Setting {} not found", id)))?;

        if !setting.is_deleted() {
            return Err(ApiError::BadRequest(format!(
                "Setting {} is not deleted",
                id
            )));
        }

        setting.restore();
        Ok(())
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Setting>, ApiError> {
        let inner = self.inner.lock();
        let items: Vec<Setting> = inner
            .settings
            .values()
            .filter(|s| s.tenant_id == tenant_id && s.is_deleted())
            .cloned()
            .collect();
        Ok(items)
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let setting = inner
            .settings
            .get(&id)
            .filter(|s| s.tenant_id == tenant_id && s.is_deleted())
            .ok_or_else(|| ApiError::NotFound(format!("Deleted setting {} not found", id)))?;

        let key = setting.key.clone();
        inner.settings.remove(&id);
        inner.tenant_keys.remove(&(tenant_id, key));
        Ok(())
    }
}
