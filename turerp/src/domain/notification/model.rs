//! Notification domain model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::impl_soft_deletable;

/// Notification channel
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum NotificationChannel {
    Email,
    Sms,
    InApp,
}

impl std::fmt::Display for NotificationChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Email => write!(f, "email"),
            Self::Sms => write!(f, "sms"),
            Self::InApp => write!(f, "inapp"),
        }
    }
}

impl std::str::FromStr for NotificationChannel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "email" => Ok(Self::Email),
            "sms" => Ok(Self::Sms),
            "inapp" => Ok(Self::InApp),
            _ => Err(format!("Invalid notification channel: {}", s)),
        }
    }
}

/// Notification priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum NotificationPriority {
    Low,
    #[default]
    Normal,
    High,
    Urgent,
}

impl std::fmt::Display for NotificationPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Normal => write!(f, "normal"),
            Self::High => write!(f, "high"),
            Self::Urgent => write!(f, "urgent"),
        }
    }
}

impl std::str::FromStr for NotificationPriority {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "low" => Ok(Self::Low),
            "normal" => Ok(Self::Normal),
            "high" => Ok(Self::High),
            "urgent" => Ok(Self::Urgent),
            _ => Err(format!("Invalid notification priority: {}", s)),
        }
    }
}

/// Notification status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum NotificationStatus {
    Queued,
    Sending,
    Sent,
    Delivered,
    Failed,
    Read,
    Cancelled,
}

impl std::fmt::Display for NotificationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Queued => write!(f, "queued"),
            Self::Sending => write!(f, "sending"),
            Self::Sent => write!(f, "sent"),
            Self::Delivered => write!(f, "delivered"),
            Self::Failed => write!(f, "failed"),
            Self::Read => write!(f, "read"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl std::str::FromStr for NotificationStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "queued" => Ok(Self::Queued),
            "sending" => Ok(Self::Sending),
            "sent" => Ok(Self::Sent),
            "delivered" => Ok(Self::Delivered),
            "failed" => Ok(Self::Failed),
            "read" => Ok(Self::Read),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(format!("Invalid notification status: {}", s)),
        }
    }
}

/// Notification request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationRequest {
    pub tenant_id: i64,
    pub user_id: Option<i64>,
    pub channel: NotificationChannel,
    pub priority: NotificationPriority,
    pub template_key: String,
    pub template_vars: serde_json::Value,
    pub recipient: String,
}

/// Notification record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: i64,
    pub tenant_id: i64,
    pub user_id: Option<i64>,
    pub channel: NotificationChannel,
    pub priority: NotificationPriority,
    pub status: NotificationStatus,
    pub notification_type: String,
    pub subject: String,
    pub body: String,
    pub recipient: String,
    pub template_key: Option<String>,
    pub template_vars: Option<serde_json::Value>,
    pub provider_message_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
    pub read_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub attempts: u32,
    pub job_id: Option<i64>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

/// In-app notification for the notification bell
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InAppNotification {
    pub id: i64,
    pub tenant_id: i64,
    pub user_id: i64,
    pub title: String,
    pub message: String,
    pub notification_type: String,
    pub read: bool,
    pub created_at: DateTime<Utc>,
    pub read_at: Option<DateTime<Utc>>,
    pub link: Option<String>,
    pub related_notification_id: Option<i64>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

/// Notification preference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPreference {
    pub id: i64,
    pub tenant_id: i64,
    pub user_id: i64,
    pub channel: NotificationChannel,
    pub notification_type: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

/// DTO for updating a preference
#[derive(Debug, Clone, Deserialize, Serialize, Validate, ToSchema)]
pub struct UpdatePreference {
    pub channel: String,
    pub notification_type: String,
    pub enabled: bool,
}

/// Email template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailTemplate {
    pub key: String,
    pub subject_template: String,
    pub body_template: String,
    pub html_template: Option<String>,
}

/// Notification response DTO for API
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NotificationResponse {
    pub id: i64,
    pub tenant_id: i64,
    pub user_id: Option<i64>,
    pub channel: String,
    pub priority: String,
    pub status: String,
    pub notification_type: String,
    pub subject: String,
    pub body: String,
    pub recipient: String,
    pub template_key: Option<String>,
    pub created_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
    pub attempts: u32,
    pub job_id: Option<i64>,
}

impl From<Notification> for NotificationResponse {
    fn from(n: Notification) -> Self {
        Self {
            id: n.id,
            tenant_id: n.tenant_id,
            user_id: n.user_id,
            channel: n.channel.to_string(),
            priority: n.priority.to_string(),
            status: n.status.to_string(),
            notification_type: n.notification_type,
            subject: n.subject,
            body: n.body,
            recipient: n.recipient,
            template_key: n.template_key,
            created_at: n.created_at,
            sent_at: n.sent_at,
            attempts: n.attempts,
            job_id: None,
        }
    }
}

/// In-app notification response DTO
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InAppNotificationResponse {
    pub id: i64,
    pub tenant_id: i64,
    pub user_id: i64,
    pub title: String,
    pub message: String,
    pub notification_type: String,
    pub read: bool,
    pub created_at: String,
    pub link: Option<String>,
}

impl From<InAppNotification> for InAppNotificationResponse {
    fn from(n: InAppNotification) -> Self {
        Self {
            id: n.id,
            tenant_id: n.tenant_id,
            user_id: n.user_id,
            title: n.title,
            message: n.message,
            notification_type: n.notification_type,
            read: n.read,
            created_at: n.created_at.to_rfc3339(),
            link: n.link,
        }
    }
}

/// Preference response DTO
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NotificationPreferenceResponse {
    pub id: i64,
    pub channel: String,
    pub notification_type: String,
    pub enabled: bool,
    pub updated_at: DateTime<Utc>,
}

impl From<NotificationPreference> for NotificationPreferenceResponse {
    fn from(p: NotificationPreference) -> Self {
        Self {
            id: p.id,
            channel: p.channel.to_string(),
            notification_type: p.notification_type,
            enabled: p.enabled,
            updated_at: p.updated_at,
        }
    }
}

impl_soft_deletable!(Notification);
impl_soft_deletable!(InAppNotification);
impl_soft_deletable!(NotificationPreference);

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_channel_display() {
        assert_eq!(NotificationChannel::Email.to_string(), "email");
        assert_eq!(NotificationChannel::Sms.to_string(), "sms");
        assert_eq!(NotificationChannel::InApp.to_string(), "inapp");
    }

    #[test]
    fn test_channel_from_str() {
        assert_eq!(
            NotificationChannel::from_str("email").unwrap(),
            NotificationChannel::Email
        );
        assert_eq!(
            NotificationChannel::from_str("SMS").unwrap(),
            NotificationChannel::Sms
        );
        assert!(NotificationChannel::from_str("invalid").is_err());
    }

    #[test]
    fn test_priority_display() {
        assert_eq!(NotificationPriority::Low.to_string(), "low");
        assert_eq!(NotificationPriority::Urgent.to_string(), "urgent");
    }

    #[test]
    fn test_status_display() {
        assert_eq!(NotificationStatus::Queued.to_string(), "queued");
        assert_eq!(NotificationStatus::Failed.to_string(), "failed");
    }

    #[test]
    fn test_notification_response_from_notification() {
        let n = Notification {
            id: 1,
            tenant_id: 1,
            user_id: Some(1),
            channel: NotificationChannel::Email,
            priority: NotificationPriority::Normal,
            status: NotificationStatus::Sent,
            notification_type: "invoice_created".to_string(),
            subject: "Test".to_string(),
            body: "Body".to_string(),
            recipient: "test@example.com".to_string(),
            template_key: Some("invoice_created".to_string()),
            template_vars: None,
            provider_message_id: None,
            created_at: Utc::now(),
            sent_at: Some(Utc::now()),
            read_at: None,
            last_error: None,
            attempts: 1,
            job_id: None,
            deleted_at: None,
            deleted_by: None,
        };
        let resp: NotificationResponse = n.into();
        assert_eq!(resp.channel, "email");
        assert_eq!(resp.status, "sent");
    }
}
