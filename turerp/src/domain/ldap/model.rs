//! LDAP domain model

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

/// LDAP configuration entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LdapConfig {
    pub id: i64,
    pub tenant_id: i64,
    pub ldap_url: String,
    pub bind_dn: String,
    /// Encrypted bind password (AES-256-GCM)
    pub bind_password_encrypted: String,
    pub base_dn: String,
    pub user_filter: String,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl LdapConfig {
    /// Create a new LDAP config (for testing/in-memory)
    pub fn new(
        id: i64,
        tenant_id: i64,
        ldap_url: String,
        bind_dn: String,
        bind_password_encrypted: String,
        base_dn: String,
        user_filter: String,
    ) -> Self {
        Self {
            id,
            tenant_id,
            ldap_url,
            bind_dn,
            bind_password_encrypted,
            base_dn,
            user_filter,
            is_active: true,
            created_at: chrono::Utc::now(),
            updated_at: None,
        }
    }
}

/// Data for creating a new LDAP configuration
#[derive(Debug, Clone, Deserialize, Serialize, Validate, ToSchema)]
pub struct CreateLdapConfig {
    #[validate(length(min = 1, max = 500))]
    pub ldap_url: String,

    #[validate(length(min = 1, max = 500))]
    pub bind_dn: String,

    #[validate(length(min = 1))]
    pub bind_password: String,

    #[validate(length(min = 1, max = 500))]
    pub base_dn: String,

    #[validate(length(min = 1, max = 500))]
    pub user_filter: String,
}

/// Data for updating an existing LDAP configuration
#[derive(Debug, Clone, Deserialize, Serialize, Default, Validate, ToSchema)]
pub struct UpdateLdapConfig {
    #[validate(length(min = 1, max = 500))]
    #[serde(default)]
    pub ldap_url: Option<String>,

    #[validate(length(min = 1, max = 500))]
    #[serde(default)]
    pub bind_dn: Option<String>,

    #[serde(default)]
    pub bind_password: Option<String>,

    #[validate(length(min = 1, max = 500))]
    #[serde(default)]
    pub base_dn: Option<String>,

    #[validate(length(min = 1, max = 500))]
    #[serde(default)]
    pub user_filter: Option<String>,

    #[serde(default)]
    pub is_active: Option<bool>,
}

/// LDAP user mapped from Active Directory / LDAP directory
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LdapUser {
    pub dn: String,
    pub username: String,
    pub email: String,
    pub full_name: String,
    pub groups: Vec<String>,
}

/// Result of an LDAP user synchronization operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct LdapSyncResult {
    pub imported: u32,
    pub updated: u32,
    pub skipped: u32,
    pub errors: u32,
}

impl LdapSyncResult {
    pub fn new() -> Self {
        Self::default()
    }
}

/// LDAP config response (without sensitive data)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LdapConfigResponse {
    pub id: i64,
    pub tenant_id: i64,
    pub ldap_url: String,
    pub bind_dn: String,
    pub base_dn: String,
    pub user_filter: String,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl From<LdapConfig> for LdapConfigResponse {
    fn from(config: LdapConfig) -> Self {
        Self {
            id: config.id,
            tenant_id: config.tenant_id,
            ldap_url: config.ldap_url,
            bind_dn: config.bind_dn,
            base_dn: config.base_dn,
            user_filter: config.user_filter,
            is_active: config.is_active,
            created_at: config.created_at,
        }
    }
}

/// Test connection request payload
#[derive(Debug, Clone, Deserialize, Serialize, Validate, ToSchema)]
pub struct TestLdapConnectionRequest {
    #[validate(length(min = 1, max = 500))]
    pub ldap_url: String,

    #[validate(length(min = 1, max = 500))]
    pub bind_dn: String,

    #[validate(length(min = 1))]
    pub bind_password: String,

    #[validate(length(min = 1, max = 500))]
    pub base_dn: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn test_ldap_config_new() {
        let config = LdapConfig::new(
            1,
            2,
            "ldap://localhost:389".to_string(),
            "cn=admin,dc=example,dc=com".to_string(),
            "encrypted_password".to_string(),
            "dc=example,dc=com".to_string(),
            "(objectClass=person)".to_string(),
        );

        assert_eq!(config.id, 1);
        assert_eq!(config.tenant_id, 2);
        assert_eq!(config.ldap_url, "ldap://localhost:389");
        assert!(config.is_active);
    }

    #[test]
    fn test_create_ldap_config_validation() {
        let create = CreateLdapConfig {
            ldap_url: "ldap://localhost:389".to_string(),
            bind_dn: "cn=admin,dc=example,dc=com".to_string(),
            bind_password: "secret123".to_string(),
            base_dn: "dc=example,dc=com".to_string(),
            user_filter: "(objectClass=person)".to_string(),
        };

        assert!(create.validate().is_ok());
    }

    #[test]
    fn test_create_ldap_config_empty_url_fails() {
        let create = CreateLdapConfig {
            ldap_url: "".to_string(),
            bind_dn: "cn=admin,dc=example,dc=com".to_string(),
            bind_password: "secret123".to_string(),
            base_dn: "dc=example,dc=com".to_string(),
            user_filter: "(objectClass=person)".to_string(),
        };

        assert!(create.validate().is_err());
    }

    #[test]
    fn test_ldap_sync_result_new() {
        let result = LdapSyncResult::new();
        assert_eq!(result.imported, 0);
        assert_eq!(result.updated, 0);
        assert_eq!(result.skipped, 0);
        assert_eq!(result.errors, 0);
    }

    #[test]
    fn test_ldap_user_serialization() {
        let user = LdapUser {
            dn: "cn=john,dc=example,dc=com".to_string(),
            username: "john".to_string(),
            email: "john@example.com".to_string(),
            full_name: "John Doe".to_string(),
            groups: vec!["users".to_string(), "admins".to_string()],
        };

        let json = serde_json::to_string(&user).unwrap();
        assert!(json.contains("john"));
        assert!(json.contains("John Doe"));
    }

    #[test]
    fn test_ldap_config_response_from_config() {
        let config = LdapConfig::new(
            1,
            2,
            "ldap://localhost:389".to_string(),
            "cn=admin,dc=example,dc=com".to_string(),
            "encrypted_password".to_string(),
            "dc=example,dc=com".to_string(),
            "(objectClass=person)".to_string(),
        );

        let response: LdapConfigResponse = config.into();
        assert_eq!(response.id, 1);
        assert_eq!(response.ldap_url, "ldap://localhost:389");
        // Password should not be exposed
        assert!(!serde_json::to_string(&response)
            .unwrap()
            .contains("encrypted_password"));
    }

    #[test]
    fn test_update_ldap_config_default() {
        let update = UpdateLdapConfig::default();
        assert!(update.ldap_url.is_none());
        assert!(update.bind_dn.is_none());
        assert!(update.bind_password.is_none());
        assert!(update.base_dn.is_none());
        assert!(update.user_filter.is_none());
        assert!(update.is_active.is_none());
    }
}
