//! Push notification token model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::impl_soft_deletable;

/// Device type for push notifications
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DeviceType {
    Ios,
    Android,
    Web,
}

impl std::fmt::Display for DeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ios => write!(f, "ios"),
            Self::Android => write!(f, "android"),
            Self::Web => write!(f, "web"),
        }
    }
}

impl std::str::FromStr for DeviceType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "ios" => Ok(Self::Ios),
            "android" => Ok(Self::Android),
            "web" => Ok(Self::Web),
            _ => Err(format!("Invalid device type: {}", s)),
        }
    }
}

/// Push token record for a user's device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushToken {
    pub id: i64,
    pub tenant_id: i64,
    pub user_id: i64,
    pub device_type: DeviceType,
    pub token: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

/// DTO for registering a push token
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct RegisterPushToken {
    pub user_id: i64,
    pub device_type: String,
    pub token: String,
}

/// Push message payload
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PushMessage {
    pub title: String,
    pub body: String,
    pub data: Option<serde_json::Value>,
    pub badge: Option<i32>,
    pub sound: Option<String>,
}

impl_soft_deletable!(PushToken);

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_device_type_display() {
        assert_eq!(DeviceType::Ios.to_string(), "ios");
        assert_eq!(DeviceType::Android.to_string(), "android");
        assert_eq!(DeviceType::Web.to_string(), "web");
    }

    #[test]
    fn test_device_type_from_str() {
        assert_eq!(DeviceType::from_str("ios").unwrap(), DeviceType::Ios);
        assert_eq!(
            DeviceType::from_str("ANDROID").unwrap(),
            DeviceType::Android
        );
        assert!(DeviceType::from_str("invalid").is_err());
    }
}
