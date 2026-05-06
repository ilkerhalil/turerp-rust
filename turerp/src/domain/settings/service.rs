//! Settings service

use crate::common::pagination::PaginatedResult;
use crate::domain::settings::model::{
    BulkUpdateSettingItem, CreateSetting, Setting, SettingDataType, UpdateSetting,
};
use crate::domain::settings::repository::BoxSettingsRepository;
use crate::error::ApiError;

/// Service for managing application settings per tenant
#[derive(Clone)]
pub struct SettingsService {
    repo: BoxSettingsRepository,
}

impl SettingsService {
    pub fn new(repo: BoxSettingsRepository) -> Self {
        Self { repo }
    }

    /// Create a new setting
    pub async fn create(&self, create: CreateSetting) -> Result<Setting, ApiError> {
        create
            .validate()
            .map_err(|errors| ApiError::Validation(errors.join("; ")))?;

        // Validate value type against declared data_type
        crate::domain::settings::model::validate_value_type(&create.value, &create.data_type)
            .map_err(ApiError::Validation)?;

        self.repo.create(create).await
    }

    /// List settings with pagination
    pub async fn list_paginated(
        &self,
        tenant_id: i64,
        group: Option<&str>,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Setting>, ApiError> {
        self.repo
            .find_all_paginated(tenant_id, group, page, per_page)
            .await
    }

    /// Get a setting by key
    pub async fn get_by_key(&self, tenant_id: i64, key: &str) -> Result<Option<Setting>, ApiError> {
        self.repo.find_by_key(tenant_id, key).await
    }

    /// Get a setting by ID
    pub async fn get_by_id(&self, tenant_id: i64, id: i64) -> Result<Option<Setting>, ApiError> {
        self.repo.find_by_id(id, tenant_id).await
    }

    /// List all settings for a tenant
    pub async fn list(
        &self,
        tenant_id: i64,
        group: Option<&str>,
    ) -> Result<Vec<Setting>, ApiError> {
        self.repo.find_all(tenant_id, group).await
    }

    /// Update a setting
    pub async fn update(
        &self,
        tenant_id: i64,
        id: i64,
        update: UpdateSetting,
    ) -> Result<Setting, ApiError> {
        if let Some(ref value) = update.value {
            // We need to know the data_type of the existing setting to validate
            let existing = self
                .repo
                .find_by_id(id, tenant_id)
                .await?
                .ok_or_else(|| ApiError::NotFound(format!("Setting {} not found", id)))?;

            crate::domain::settings::model::validate_value_type(value, &existing.data_type)
                .map_err(ApiError::Validation)?;
        }

        self.repo.update(id, tenant_id, update).await
    }

    /// Delete a setting
    pub async fn delete(&self, tenant_id: i64, id: i64) -> Result<(), ApiError> {
        self.repo.delete(id, tenant_id).await
    }

    /// Soft delete a setting
    pub async fn soft_delete_setting(
        &self,
        tenant_id: i64,
        id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.repo.soft_delete(id, tenant_id, deleted_by).await
    }

    /// Restore a soft-deleted setting
    pub async fn restore_setting(&self, tenant_id: i64, id: i64) -> Result<(), ApiError> {
        self.repo.restore(id, tenant_id).await
    }

    /// List deleted settings for a tenant
    pub async fn list_deleted_settings(&self, tenant_id: i64) -> Result<Vec<Setting>, ApiError> {
        self.repo.find_deleted(tenant_id).await
    }

    /// Permanently destroy a soft-deleted setting
    pub async fn destroy_setting(&self, tenant_id: i64, id: i64) -> Result<(), ApiError> {
        self.repo.destroy(id, tenant_id).await
    }

    /// Bulk update settings by key
    pub async fn bulk_update(
        &self,
        tenant_id: i64,
        updates: Vec<BulkUpdateSettingItem>,
    ) -> Result<Vec<Setting>, ApiError> {
        self.repo.bulk_update(tenant_id, updates).await
    }

    // Typed getters ----------------------------------------------------------------

    /// Get a string value by key
    pub async fn get_string(
        &self,
        tenant_id: i64,
        key: &str,
        default: Option<String>,
    ) -> Result<String, ApiError> {
        match self.repo.find_by_key(tenant_id, key).await? {
            Some(setting) => match setting.value {
                serde_json::Value::String(s) => Ok(s),
                _ => {
                    if let Some(d) = default {
                        Ok(d)
                    } else {
                        Err(ApiError::BadRequest(format!(
                            "Setting '{}' is not a string",
                            key
                        )))
                    }
                }
            },
            None => {
                if let Some(d) = default {
                    Ok(d)
                } else {
                    Err(ApiError::NotFound(format!("Setting '{}' not found", key)))
                }
            }
        }
    }

    /// Get an integer value by key
    pub async fn get_int(
        &self,
        tenant_id: i64,
        key: &str,
        default: Option<i64>,
    ) -> Result<i64, ApiError> {
        match self.repo.find_by_key(tenant_id, key).await? {
            Some(setting) => match setting.value {
                serde_json::Value::Number(n) => n.as_i64().ok_or_else(|| {
                    ApiError::BadRequest(format!("Setting '{}' is not a valid integer", key))
                }),
                _ => {
                    if let Some(d) = default {
                        Ok(d)
                    } else {
                        Err(ApiError::BadRequest(format!(
                            "Setting '{}' is not an integer",
                            key
                        )))
                    }
                }
            },
            None => {
                if let Some(d) = default {
                    Ok(d)
                } else {
                    Err(ApiError::NotFound(format!("Setting '{}' not found", key)))
                }
            }
        }
    }

    /// Get a boolean value by key
    pub async fn get_bool(
        &self,
        tenant_id: i64,
        key: &str,
        default: Option<bool>,
    ) -> Result<bool, ApiError> {
        match self.repo.find_by_key(tenant_id, key).await? {
            Some(setting) => match setting.value {
                serde_json::Value::Bool(b) => Ok(b),
                _ => {
                    if let Some(d) = default {
                        Ok(d)
                    } else {
                        Err(ApiError::BadRequest(format!(
                            "Setting '{}' is not a boolean",
                            key
                        )))
                    }
                }
            },
            None => {
                if let Some(d) = default {
                    Ok(d)
                } else {
                    Err(ApiError::NotFound(format!("Setting '{}' not found", key)))
                }
            }
        }
    }

    /// Get a float value by key
    pub async fn get_float(
        &self,
        tenant_id: i64,
        key: &str,
        default: Option<f64>,
    ) -> Result<f64, ApiError> {
        match self.repo.find_by_key(tenant_id, key).await? {
            Some(setting) => match setting.value {
                serde_json::Value::Number(n) => n.as_f64().ok_or_else(|| {
                    ApiError::BadRequest(format!("Setting '{}' is not a valid float", key))
                }),
                _ => {
                    if let Some(d) = default {
                        Ok(d)
                    } else {
                        Err(ApiError::BadRequest(format!(
                            "Setting '{}' is not a float",
                            key
                        )))
                    }
                }
            },
            None => {
                if let Some(d) = default {
                    Ok(d)
                } else {
                    Err(ApiError::NotFound(format!("Setting '{}' not found", key)))
                }
            }
        }
    }

    /// Get a JSON value by key
    pub async fn get_json(
        &self,
        tenant_id: i64,
        key: &str,
        default: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, ApiError> {
        match self.repo.find_by_key(tenant_id, key).await? {
            Some(setting) => Ok(setting.value),
            None => {
                if let Some(d) = default {
                    Ok(d)
                } else {
                    Err(ApiError::NotFound(format!("Setting '{}' not found", key)))
                }
            }
        }
    }

    /// Seed default settings for a new tenant
    pub async fn seed_defaults(&self, tenant_id: i64) -> Result<Vec<Setting>, ApiError> {
        let defaults = vec![
            CreateSetting {
                tenant_id,
                key: "company.name".to_string(),
                value: serde_json::json!("My Company"),
                default_value: Some(serde_json::json!("My Company")),
                data_type: SettingDataType::String,
                group: crate::domain::settings::model::SettingGroup::Company,
                description: "Company display name".to_string(),
                is_sensitive: false,
                is_editable: true,
            },
            CreateSetting {
                tenant_id,
                key: "company.currency".to_string(),
                value: serde_json::json!("TRY"),
                default_value: Some(serde_json::json!("TRY")),
                data_type: SettingDataType::String,
                group: crate::domain::settings::model::SettingGroup::Company,
                description: "Default currency code".to_string(),
                is_sensitive: false,
                is_editable: true,
            },
            CreateSetting {
                tenant_id,
                key: "invoice.prefix".to_string(),
                value: serde_json::json!("FAT"),
                default_value: Some(serde_json::json!("FAT")),
                data_type: SettingDataType::String,
                group: crate::domain::settings::model::SettingGroup::Invoice,
                description: "Invoice number prefix".to_string(),
                is_sensitive: false,
                is_editable: true,
            },
            CreateSetting {
                tenant_id,
                key: "invoice.next_number".to_string(),
                value: serde_json::json!(1),
                default_value: Some(serde_json::json!(1)),
                data_type: SettingDataType::Integer,
                group: crate::domain::settings::model::SettingGroup::Invoice,
                description: "Next invoice number counter".to_string(),
                is_sensitive: false,
                is_editable: true,
            },
            CreateSetting {
                tenant_id,
                key: "email.from_address".to_string(),
                value: serde_json::json!("noreply@example.com"),
                default_value: Some(serde_json::json!("noreply@example.com")),
                data_type: SettingDataType::String,
                group: crate::domain::settings::model::SettingGroup::Email,
                description: "Default sender email address".to_string(),
                is_sensitive: false,
                is_editable: true,
            },
            CreateSetting {
                tenant_id,
                key: "security.password_min_length".to_string(),
                value: serde_json::json!(12),
                default_value: Some(serde_json::json!(12)),
                data_type: SettingDataType::Integer,
                group: crate::domain::settings::model::SettingGroup::Security,
                description: "Minimum password length".to_string(),
                is_sensitive: false,
                is_editable: true,
            },
            CreateSetting {
                tenant_id,
                key: "security.mfa_required".to_string(),
                value: serde_json::json!(false),
                default_value: Some(serde_json::json!(false)),
                data_type: SettingDataType::Boolean,
                group: crate::domain::settings::model::SettingGroup::Security,
                description: "Require multi-factor authentication".to_string(),
                is_sensitive: false,
                is_editable: true,
            },
            CreateSetting {
                tenant_id,
                key: "localization.locale".to_string(),
                value: serde_json::json!("tr"),
                default_value: Some(serde_json::json!("tr")),
                data_type: SettingDataType::String,
                group: crate::domain::settings::model::SettingGroup::Localization,
                description: "Default application locale".to_string(),
                is_sensitive: false,
                is_editable: true,
            },
        ];

        let mut created = Vec::new();
        for create in defaults {
            if !self.repo.key_exists(tenant_id, &create.key).await? {
                created.push(self.repo.create(create).await?);
            }
        }

        Ok(created)
    }
}
