//! Tenant domain models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Tenant entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub id: i64,
    pub name: String,
    pub subdomain: String,
    pub db_name: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

/// Create tenant request
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_db_name() {
        assert_eq!(generate_db_name("testco"), "turerp_testco");
        assert_eq!(generate_db_name("test-co"), "turerp_test_co");
        assert_eq!(generate_db_name("ABC"), "turerp_abc");
    }

    #[test]
    fn test_valid_subdomain() {
        assert!(valid_subdomain(&"testco"));
        assert!(valid_subdomain(&"test-co"));
        assert!(valid_subdomain(&"test123"));
        assert!(!valid_subdomain(&"TestCo"));
        assert!(!valid_subdomain(&"test_co"));
        assert!(!valid_subdomain(&"test co"));
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
}
