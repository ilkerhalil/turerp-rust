//! Feature Flag Model
//!
//! Represents a feature flag that can be toggled on/off globally or per-tenant.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

/// Feature flag status
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum FeatureFlagStatus {
    /// Feature is enabled
    Enabled,
    /// Feature is disabled
    #[default]
    Disabled,
}

impl std::fmt::Display for FeatureFlagStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FeatureFlagStatus::Enabled => write!(f, "enabled"),
            FeatureFlagStatus::Disabled => write!(f, "disabled"),
        }
    }
}

/// Feature flag entity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FeatureFlag {
    /// Unique identifier
    pub id: i64,
    /// Feature flag name (e.g., "purchase_requests", "new_dashboard")
    pub name: String,
    /// Human-readable description
    pub description: Option<String>,
    /// Current status (enabled/disabled)
    pub status: FeatureFlagStatus,
    /// Optional tenant ID (None = global flag)
    pub tenant_id: Option<i64>,
    /// Creation timestamp
    pub created_at: chrono::NaiveDateTime,
    /// Last update timestamp
    pub updated_at: chrono::NaiveDateTime,
}

/// Create feature flag request
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct CreateFeatureFlag {
    /// Feature flag name (required, 1-100 characters, alphanumeric with underscores/hyphens)
    #[validate(length(min = 1, max = 100, message = "Name must be 1-100 characters"))]
    pub name: String,
    /// Human-readable description (optional, max 500 characters)
    #[validate(length(max = 500, message = "Description must be at most 500 characters"))]
    pub description: Option<String>,
    /// Initial status (defaults to Disabled)
    pub status: Option<FeatureFlagStatus>,
    /// Optional tenant ID for tenant-specific flags
    pub tenant_id: Option<i64>,
}

/// Update feature flag request
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct UpdateFeatureFlag {
    /// Human-readable description (optional, max 500 characters)
    #[validate(length(max = 500, message = "Description must be at most 500 characters"))]
    pub description: Option<String>,
    /// New status
    pub status: Option<FeatureFlagStatus>,
}

/// Feature flag response for API
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FeatureFlagResponse {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub status: FeatureFlagStatus,
    pub tenant_id: Option<i64>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

impl From<FeatureFlag> for FeatureFlagResponse {
    fn from(flag: FeatureFlag) -> Self {
        Self {
            id: flag.id,
            name: flag.name,
            description: flag.description,
            status: flag.status,
            tenant_id: flag.tenant_id,
            created_at: flag.created_at,
            updated_at: flag.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_flag_status_default() {
        assert_eq!(FeatureFlagStatus::default(), FeatureFlagStatus::Disabled);
    }

    #[test]
    fn test_feature_flag_status_display() {
        assert_eq!(FeatureFlagStatus::Enabled.to_string(), "enabled");
        assert_eq!(FeatureFlagStatus::Disabled.to_string(), "disabled");
    }

    #[test]
    fn test_feature_flag_status_serialization() {
        let enabled = FeatureFlagStatus::Enabled;
        let json = serde_json::to_string(&enabled).unwrap();
        assert_eq!(json, "\"enabled\"");

        let disabled = FeatureFlagStatus::Disabled;
        let json = serde_json::to_string(&disabled).unwrap();
        assert_eq!(json, "\"disabled\"");
    }

    #[test]
    fn test_feature_flag_status_deserialization() {
        let enabled: FeatureFlagStatus = serde_json::from_str("\"enabled\"").unwrap();
        assert_eq!(enabled, FeatureFlagStatus::Enabled);

        let disabled: FeatureFlagStatus = serde_json::from_str("\"disabled\"").unwrap();
        assert_eq!(disabled, FeatureFlagStatus::Disabled);
    }

    #[test]
    fn test_create_feature_flag_validation() {
        use validator::Validate;

        // Valid flag
        let valid = CreateFeatureFlag {
            name: "new_feature".to_string(),
            description: Some("A new feature".to_string()),
            status: Some(FeatureFlagStatus::Enabled),
            tenant_id: None,
        };
        assert!(valid.validate().is_ok());

        // Name too short
        let invalid_name = CreateFeatureFlag {
            name: "".to_string(),
            description: None,
            status: None,
            tenant_id: None,
        };
        assert!(invalid_name.validate().is_err());

        // Name too long
        let long_name = CreateFeatureFlag {
            name: "a".repeat(101),
            description: None,
            status: None,
            tenant_id: None,
        };
        assert!(long_name.validate().is_err());
    }

    #[test]
    fn test_feature_flag_response_from_flag() {
        let flag = FeatureFlag {
            id: 1,
            name: "test_feature".to_string(),
            description: Some("Test feature".to_string()),
            status: FeatureFlagStatus::Enabled,
            tenant_id: Some(1),
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        };

        let response: FeatureFlagResponse = flag.into();
        assert_eq!(response.id, 1);
        assert_eq!(response.name, "test_feature");
        assert_eq!(response.status, FeatureFlagStatus::Enabled);
        assert_eq!(response.tenant_id, Some(1));
    }
}
