//! Tenant domain models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

/// Tenant entity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Tenant {
    pub id: i64,
    pub name: String,
    pub subdomain: String,
    pub db_name: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

/// Create tenant request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateTenant {
    pub name: String,
    pub subdomain: String,
}

impl CreateTenant {
    /// Validate the create tenant request
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.name.trim().is_empty() {
            errors.push("Name is required".to_string());
        }
        if self.name.len() > 255 {
            errors.push("Name must be at most 255 characters".to_string());
        }
        if self.subdomain.trim().is_empty() {
            errors.push("Subdomain is required".to_string());
        }
        if self.subdomain.len() > 63 {
            errors.push("Subdomain must be at most 63 characters".to_string());
        }
        if !valid_subdomain(&self.subdomain) {
            errors.push("Subdomain must be lowercase alphanumeric with hyphens".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Update tenant request
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct UpdateTenant {
    #[serde(default)]
    pub name: Option<String>,

    #[serde(default)]
    pub subdomain: Option<String>,

    #[serde(default)]
    pub is_active: Option<bool>,
}

/// Validate subdomain format
fn valid_subdomain(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Generate database name from subdomain
pub fn generate_db_name(subdomain: &str) -> String {
    format!("turerp_{}", subdomain.to_lowercase().replace('-', "_"))
}

/// Tenant configuration entity
///
/// Stores per-tenant configuration as key-value pairs with JSON values.
/// Supports hierarchical configuration with optional encryption for sensitive values.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TenantConfig {
    pub id: i64,
    pub tenant_id: i64,
    pub key: String,
    pub value: Value,
    pub is_encrypted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create tenant config request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateTenantConfig {
    pub tenant_id: i64,
    pub key: String,
    pub value: Value,
    pub is_encrypted: Option<bool>,
}

impl CreateTenantConfig {
    /// Validate the create tenant config request
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.key.trim().is_empty() {
            errors.push("Key is required".to_string());
        }
        if self.key.len() > 255 {
            errors.push("Key must be at most 255 characters".to_string());
        }
        // Key should be a valid config key (alphanumeric, dots, underscores)
        if !self
            .key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
        {
            errors.push(
                "Key must contain only alphanumeric characters, dots, underscores, or hyphens"
                    .to_string(),
            );
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Update tenant config request
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct UpdateTenantConfig {
    #[serde(default)]
    pub value: Option<Value>,

    #[serde(default)]
    pub is_encrypted: Option<bool>,
}

/// Tenant config response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TenantConfigResponse {
    pub id: i64,
    pub tenant_id: i64,
    pub key: String,
    pub value: Value,
    pub is_encrypted: bool,
}

impl From<TenantConfig> for TenantConfigResponse {
    fn from(config: TenantConfig) -> Self {
        Self {
            id: config.id,
            tenant_id: config.tenant_id,
            key: config.key,
            value: config.value,
            is_encrypted: config.is_encrypted,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_generate_db_name() {
        assert_eq!(generate_db_name("testco"), "turerp_testco");
        assert_eq!(generate_db_name("test-co"), "turerp_test_co");
        assert_eq!(generate_db_name("ABC"), "turerp_abc");
    }

    #[test]
    fn test_valid_subdomain() {
        assert!(valid_subdomain("testco"));
        assert!(valid_subdomain("test-co"));
        assert!(valid_subdomain("test123"));
        assert!(!valid_subdomain("TestCo"));
        assert!(!valid_subdomain("test_co"));
        assert!(!valid_subdomain("test co"));
    }

    #[test]
    fn test_create_tenant_validation() {
        let valid = CreateTenant {
            name: "Test Company".to_string(),
            subdomain: "testco".to_string(),
        };
        assert!(valid.validate().is_ok());

        let invalid_name = CreateTenant {
            name: "".to_string(),
            subdomain: "testco".to_string(),
        };
        assert!(invalid_name.validate().is_err());

        let invalid_subdomain = CreateTenant {
            name: "Test".to_string(),
            subdomain: "TestCo".to_string(),
        };
        assert!(invalid_subdomain.validate().is_err());
    }

    #[test]
    fn test_create_tenant_config_validation() {
        let valid = CreateTenantConfig {
            tenant_id: 1,
            key: "app.theme".to_string(),
            value: json!("dark"),
            is_encrypted: None,
        };
        assert!(valid.validate().is_ok());

        let valid_with_hyphen = CreateTenantConfig {
            tenant_id: 1,
            key: "app-setting-name".to_string(),
            value: json!("value"),
            is_encrypted: None,
        };
        assert!(valid_with_hyphen.validate().is_ok());

        let invalid_key = CreateTenantConfig {
            tenant_id: 1,
            key: "".to_string(),
            value: json!("value"),
            is_encrypted: None,
        };
        assert!(invalid_key.validate().is_err());

        let invalid_key_chars = CreateTenantConfig {
            tenant_id: 1,
            key: "app setting".to_string(),
            value: json!("value"),
            is_encrypted: None,
        };
        assert!(invalid_key_chars.validate().is_err());
    }

    #[test]
    fn test_tenant_config_response_from_config() {
        let config = TenantConfig {
            id: 1,
            tenant_id: 1,
            key: "app.theme".to_string(),
            value: json!({"primary": "blue", "mode": "dark"}),
            is_encrypted: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let response = TenantConfigResponse::from(config);
        assert_eq!(response.id, 1);
        assert_eq!(response.key, "app.theme");
    }
}
