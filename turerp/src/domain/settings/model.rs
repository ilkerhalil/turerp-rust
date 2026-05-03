//! Settings domain models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Supported data types for settings values
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SettingDataType {
    #[default]
    String,
    Integer,
    Boolean,
    Float,
    Json,
}

impl std::fmt::Display for SettingDataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SettingDataType::String => write!(f, "string"),
            SettingDataType::Integer => write!(f, "integer"),
            SettingDataType::Boolean => write!(f, "boolean"),
            SettingDataType::Float => write!(f, "float"),
            SettingDataType::Json => write!(f, "json"),
        }
    }
}

impl std::str::FromStr for SettingDataType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "string" => Ok(SettingDataType::String),
            "integer" => Ok(SettingDataType::Integer),
            "boolean" => Ok(SettingDataType::Boolean),
            "float" => Ok(SettingDataType::Float),
            "json" => Ok(SettingDataType::Json),
            _ => Err(format!("Unknown setting data type: {}", s)),
        }
    }
}

/// Setting group / category
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SettingGroup {
    #[default]
    General,
    Company,
    Invoice,
    Email,
    Security,
    Localization,
    Integration,
}

impl std::fmt::Display for SettingGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SettingGroup::General => write!(f, "general"),
            SettingGroup::Company => write!(f, "company"),
            SettingGroup::Invoice => write!(f, "invoice"),
            SettingGroup::Email => write!(f, "email"),
            SettingGroup::Security => write!(f, "security"),
            SettingGroup::Localization => write!(f, "localization"),
            SettingGroup::Integration => write!(f, "integration"),
        }
    }
}

impl std::str::FromStr for SettingGroup {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "general" => Ok(SettingGroup::General),
            "company" => Ok(SettingGroup::Company),
            "invoice" => Ok(SettingGroup::Invoice),
            "email" => Ok(SettingGroup::Email),
            "security" => Ok(SettingGroup::Security),
            "localization" => Ok(SettingGroup::Localization),
            "integration" => Ok(SettingGroup::Integration),
            _ => Err(format!("Unknown setting group: {}", s)),
        }
    }
}

/// Setting entity stored per-tenant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Setting {
    pub id: i64,
    pub tenant_id: i64,
    pub key: String,
    pub value: Value,
    pub default_value: Option<Value>,
    pub data_type: SettingDataType,
    pub group: SettingGroup,
    pub description: String,
    pub is_sensitive: bool,
    pub is_editable: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create setting request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSetting {
    pub tenant_id: i64,
    pub key: String,
    pub value: Value,
    #[serde(default)]
    pub default_value: Option<Value>,
    pub data_type: SettingDataType,
    pub group: SettingGroup,
    pub description: String,
    #[serde(default)]
    pub is_sensitive: bool,
    #[serde(default = "default_editable")]
    pub is_editable: bool,
}

fn default_editable() -> bool {
    true
}

impl CreateSetting {
    /// Validate the create setting request
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.key.trim().is_empty() {
            errors.push("Key is required".to_string());
        } else if self.key.len() > 255 {
            errors.push("Key must be at most 255 characters".to_string());
        } else if !valid_key(&self.key) {
            errors.push(
                "Key must contain only lowercase alphanumeric characters, dots, underscores, or hyphens"
                    .to_string(),
            );
        }

        if self.description.len() > 1000 {
            errors.push("Description must be at most 1000 characters".to_string());
        }

        // Validate value matches data_type
        if let Err(e) = validate_value_type(&self.value, &self.data_type) {
            errors.push(e);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Update setting request
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateSetting {
    #[serde(default)]
    pub value: Option<Value>,
    #[serde(default)]
    pub default_value: Option<Option<Value>>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub is_sensitive: Option<bool>,
    #[serde(default)]
    pub is_editable: Option<bool>,
}

/// Setting response (masks sensitive values)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingResponse {
    pub id: i64,
    pub tenant_id: i64,
    pub key: String,
    pub value: Value,
    pub default_value: Option<Value>,
    pub data_type: String,
    pub group: String,
    pub description: String,
    pub is_sensitive: bool,
    pub is_editable: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Setting> for SettingResponse {
    fn from(s: Setting) -> Self {
        let value = if s.is_sensitive {
            Value::String("***MASKED***".to_string())
        } else {
            s.value
        };
        let default_value = if s.is_sensitive {
            s.default_value
                .map(|_| Value::String("***MASKED***".to_string()))
        } else {
            s.default_value
        };

        Self {
            id: s.id,
            tenant_id: s.tenant_id,
            key: s.key,
            value,
            default_value,
            data_type: s.data_type.to_string(),
            group: s.group.to_string(),
            description: s.description,
            is_sensitive: s.is_sensitive,
            is_editable: s.is_editable,
            created_at: s.created_at,
            updated_at: s.updated_at,
        }
    }
}

/// Validate setting key format
fn valid_key(s: &str) -> bool {
    !s.is_empty()
        && s.chars().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_' || c == '-'
        })
}

/// Validate that a JSON value matches the expected data type
pub fn validate_value_type(value: &Value, data_type: &SettingDataType) -> Result<(), String> {
    match data_type {
        SettingDataType::String => {
            if !value.is_string() {
                return Err("Value must be a string".to_string());
            }
        }
        SettingDataType::Integer => {
            if !value.is_i64() && !value.is_u64() {
                return Err("Value must be an integer".to_string());
            }
        }
        SettingDataType::Boolean => {
            if !value.is_boolean() {
                return Err("Value must be a boolean".to_string());
            }
        }
        SettingDataType::Float => {
            if !value.is_f64() && !value.is_i64() && !value.is_u64() {
                return Err("Value must be a number".to_string());
            }
        }
        SettingDataType::Json => {
            // Any JSON value is acceptable
        }
    }
    Ok(())
}

/// Bulk update settings request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkUpdateSettings {
    pub updates: Vec<BulkUpdateSettingItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkUpdateSettingItem {
    pub key: String,
    pub value: Value,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_valid_key() {
        assert!(valid_key("app.theme"));
        assert!(valid_key("invoice_prefix"));
        assert!(valid_key("smtp-host"));
        assert!(!valid_key("App Theme"));
        assert!(!valid_key("invoice/prefix"));
        assert!(!valid_key(""));
    }

    #[test]
    fn test_setting_data_type_from_str() {
        assert_eq!(
            "string".parse::<SettingDataType>().unwrap(),
            SettingDataType::String
        );
        assert_eq!(
            "integer".parse::<SettingDataType>().unwrap(),
            SettingDataType::Integer
        );
        assert!("unknown".parse::<SettingDataType>().is_err());
    }

    #[test]
    fn test_setting_group_from_str() {
        assert_eq!(
            "invoice".parse::<SettingGroup>().unwrap(),
            SettingGroup::Invoice
        );
        assert_eq!(
            "security".parse::<SettingGroup>().unwrap(),
            SettingGroup::Security
        );
        assert!("unknown".parse::<SettingGroup>().is_err());
    }

    #[test]
    fn test_create_setting_validation() {
        let valid = CreateSetting {
            tenant_id: 1,
            key: "app.theme".to_string(),
            value: json!("dark"),
            default_value: None,
            data_type: SettingDataType::String,
            group: SettingGroup::General,
            description: "Application theme".to_string(),
            is_sensitive: false,
            is_editable: true,
        };
        assert!(valid.validate().is_ok());

        let invalid_key = CreateSetting {
            tenant_id: 1,
            key: "App Theme".to_string(),
            value: json!("dark"),
            default_value: None,
            data_type: SettingDataType::String,
            group: SettingGroup::General,
            description: "App theme".to_string(),
            is_sensitive: false,
            is_editable: true,
        };
        assert!(invalid_key.validate().is_err());
    }

    #[test]
    fn test_validate_value_type() {
        assert!(validate_value_type(&json!("hello"), &SettingDataType::String).is_ok());
        assert!(validate_value_type(&json!(42), &SettingDataType::Integer).is_ok());
        assert!(validate_value_type(&json!(true), &SettingDataType::Boolean).is_ok());
        assert!(validate_value_type(&json!(3.14), &SettingDataType::Float).is_ok());
        assert!(validate_value_type(&json!({"a": 1}), &SettingDataType::Json).is_ok());

        assert!(validate_value_type(&json!(42), &SettingDataType::String).is_err());
        assert!(validate_value_type(&json!("text"), &SettingDataType::Integer).is_err());
    }

    #[test]
    fn test_setting_response_masks_sensitive() {
        let setting = Setting {
            id: 1,
            tenant_id: 1,
            key: "smtp.password".to_string(),
            value: json!("secret123"),
            default_value: Some(json!("default")),
            data_type: SettingDataType::String,
            group: SettingGroup::Email,
            description: "SMTP password".to_string(),
            is_sensitive: true,
            is_editable: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let resp = SettingResponse::from(setting);
        assert_eq!(resp.value, json!("***MASKED***"));
        assert_eq!(resp.default_value, Some(json!("***MASKED***")));
    }

    #[test]
    fn test_setting_response_non_sensitive() {
        let setting = Setting {
            id: 1,
            tenant_id: 1,
            key: "app.name".to_string(),
            value: json!("Turerp"),
            default_value: None,
            data_type: SettingDataType::String,
            group: SettingGroup::General,
            description: "App name".to_string(),
            is_sensitive: false,
            is_editable: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let resp = SettingResponse::from(setting);
        assert_eq!(resp.value, json!("Turerp"));
    }
}
