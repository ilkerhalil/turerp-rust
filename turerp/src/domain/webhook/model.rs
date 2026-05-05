//! Webhook domain model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

/// Status of a webhook endpoint
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WebhookStatus {
    Active,
    Inactive,
    Failed,
}

impl std::fmt::Display for WebhookStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Inactive => write!(f, "inactive"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

impl std::str::FromStr for WebhookStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "active" => Ok(Self::Active),
            "inactive" => Ok(Self::Inactive),
            "failed" => Ok(Self::Failed),
            _ => Err(format!("Invalid webhook status: {}", s)),
        }
    }
}

/// Delivery status for a webhook payload
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryStatus {
    Pending,
    Delivered,
    Failed,
    Retrying,
}

impl std::fmt::Display for DeliveryStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Delivered => write!(f, "delivered"),
            Self::Failed => write!(f, "failed"),
            Self::Retrying => write!(f, "retrying"),
        }
    }
}

impl std::str::FromStr for DeliveryStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(Self::Pending),
            "delivered" => Ok(Self::Delivered),
            "failed" => Ok(Self::Failed),
            "retrying" => Ok(Self::Retrying),
            _ => Err(format!("Invalid delivery status: {}", s)),
        }
    }
}

/// Webhook entity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Webhook {
    pub id: i64,
    pub tenant_id: i64,
    pub url: String,
    pub description: Option<String>,
    pub event_types: Vec<String>,
    pub secret: String,
    pub status: WebhookStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

impl crate::common::SoftDeletable for Webhook {
    fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }
    fn deleted_at(&self) -> Option<DateTime<Utc>> {
        self.deleted_at
    }
    fn deleted_by(&self) -> Option<i64> {
        self.deleted_by
    }
    fn mark_deleted(&mut self, by_user_id: i64) {
        self.deleted_at = Some(Utc::now());
        self.deleted_by = Some(by_user_id);
    }
    fn restore(&mut self) {
        self.deleted_at = None;
        self.deleted_by = None;
    }
}

/// Webhook delivery record
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WebhookDelivery {
    pub id: i64,
    pub webhook_id: i64,
    pub tenant_id: i64,
    pub event_type: String,
    pub payload: String,
    pub status: DeliveryStatus,
    pub http_status: Option<i32>,
    pub response_body: Option<String>,
    pub error_message: Option<String>,
    pub attempt_count: i32,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub delivered_at: Option<DateTime<Utc>>,
}

/// DTO for creating a webhook
#[derive(Debug, Clone, Deserialize, Serialize, Validate, ToSchema)]
pub struct CreateWebhook {
    #[validate(length(min = 1, max = 500))]
    pub url: String,

    #[serde(default)]
    pub description: Option<String>,

    #[serde(default)]
    pub event_types: Vec<String>,

    pub secret: Option<String>,
}

/// DTO for updating a webhook
#[derive(Debug, Clone, Deserialize, Serialize, Default, Validate, ToSchema)]
pub struct UpdateWebhook {
    #[validate(length(min = 1, max = 500))]
    #[serde(default)]
    pub url: Option<String>,

    #[serde(default)]
    pub description: Option<String>,

    #[serde(default)]
    pub event_types: Option<Vec<String>>,

    #[serde(default)]
    pub status: Option<WebhookStatus>,
}

/// Response DTO for a webhook
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WebhookResponse {
    pub id: i64,
    pub tenant_id: i64,
    pub url: String,
    pub description: Option<String>,
    pub event_types: Vec<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Webhook> for WebhookResponse {
    fn from(w: Webhook) -> Self {
        Self {
            id: w.id,
            tenant_id: w.tenant_id,
            url: w.url,
            description: w.description,
            event_types: w.event_types,
            status: w.status.to_string(),
            created_at: w.created_at,
            updated_at: w.updated_at,
        }
    }
}

/// Response DTO for a webhook delivery
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WebhookDeliveryResponse {
    pub id: i64,
    pub webhook_id: i64,
    pub tenant_id: i64,
    pub event_type: String,
    pub status: String,
    pub http_status: Option<i32>,
    pub error_message: Option<String>,
    pub attempt_count: i32,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub delivered_at: Option<DateTime<Utc>>,
}

impl From<WebhookDelivery> for WebhookDeliveryResponse {
    fn from(d: WebhookDelivery) -> Self {
        Self {
            id: d.id,
            webhook_id: d.webhook_id,
            tenant_id: d.tenant_id,
            event_type: d.event_type,
            status: d.status.to_string(),
            http_status: d.http_status,
            error_message: d.error_message,
            attempt_count: d.attempt_count,
            scheduled_at: d.scheduled_at,
            created_at: d.created_at,
            delivered_at: d.delivered_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::SoftDeletable;
    use std::str::FromStr;

    #[test]
    fn test_webhook_status_display() {
        assert_eq!(WebhookStatus::Active.to_string(), "active");
        assert_eq!(WebhookStatus::Inactive.to_string(), "inactive");
        assert_eq!(WebhookStatus::Failed.to_string(), "failed");
    }

    #[test]
    fn test_webhook_status_from_str() {
        assert_eq!(
            WebhookStatus::from_str("active").unwrap(),
            WebhookStatus::Active
        );
        assert_eq!(
            WebhookStatus::from_str("FAILED").unwrap(),
            WebhookStatus::Failed
        );
        assert!(WebhookStatus::from_str("invalid").is_err());
    }

    #[test]
    fn test_delivery_status_display() {
        assert_eq!(DeliveryStatus::Pending.to_string(), "pending");
        assert_eq!(DeliveryStatus::Delivered.to_string(), "delivered");
        assert_eq!(DeliveryStatus::Retrying.to_string(), "retrying");
    }

    #[test]
    fn test_soft_delete() {
        let mut wh = Webhook {
            id: 1,
            tenant_id: 1,
            url: "https://example.com".to_string(),
            description: None,
            event_types: vec!["*".to_string()],
            secret: "secret".to_string(),
            status: WebhookStatus::Active,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
            deleted_by: None,
        };
        assert!(!wh.is_deleted());
        wh.mark_deleted(42);
        assert!(wh.is_deleted());
        assert_eq!(wh.deleted_by(), Some(42));
        wh.restore();
        assert!(!wh.is_deleted());
    }

    #[test]
    fn test_response_from_webhook() {
        let wh = Webhook {
            id: 1,
            tenant_id: 1,
            url: "https://example.com".to_string(),
            description: Some("Test".to_string()),
            event_types: vec!["invoice_created".to_string()],
            secret: "secret".to_string(),
            status: WebhookStatus::Active,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
            deleted_by: None,
        };
        let resp: WebhookResponse = wh.into();
        assert_eq!(resp.status, "active");
        assert_eq!(resp.event_types.len(), 1);
    }

    #[test]
    fn test_delivery_response_from_delivery() {
        let d = WebhookDelivery {
            id: 1,
            webhook_id: 1,
            tenant_id: 1,
            event_type: "test".to_string(),
            payload: "{}".to_string(),
            status: DeliveryStatus::Pending,
            http_status: None,
            response_body: None,
            error_message: None,
            attempt_count: 0,
            scheduled_at: None,
            created_at: Utc::now(),
            delivered_at: None,
        };
        let resp: WebhookDeliveryResponse = d.into();
        assert_eq!(resp.status, "pending");
    }
}
