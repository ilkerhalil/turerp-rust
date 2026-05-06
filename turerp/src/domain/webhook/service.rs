//! Webhook service with HTTP delivery and retry logic

use std::sync::Arc;

use chrono::Utc;
use hmac::{Hmac, Mac};
use reqwest::Client;
use sha2::Sha256;

use crate::common::events::DomainEvent;
use crate::common::PaginatedResult;
use crate::domain::webhook::model::{
    CreateWebhook, DeliveryStatus, UpdateWebhook, Webhook, WebhookDelivery,
};
use crate::domain::webhook::repository::{
    BoxWebhookDeliveryRepository, BoxWebhookRepository, WebhookDeliveryRepository,
};
use crate::error::ApiError;

/// HMAC-SHA256 type alias
type HmacSha256 = Hmac<Sha256>;

/// Webhook service handling CRUD, delivery, and retries
#[derive(Clone)]
pub struct WebhookService {
    webhook_repo: BoxWebhookRepository,
    delivery_repo: BoxWebhookDeliveryRepository,
    http_client: Client,
}

impl WebhookService {
    pub fn new(
        webhook_repo: BoxWebhookRepository,
        delivery_repo: BoxWebhookDeliveryRepository,
    ) -> Self {
        Self {
            webhook_repo,
            delivery_repo,
            http_client: Client::new(),
        }
    }

    // --- CRUD ---

    pub async fn create_webhook(
        &self,
        tenant_id: i64,
        dto: CreateWebhook,
    ) -> Result<Webhook, ApiError> {
        self.validate_create(&dto)?;
        self.webhook_repo.create(tenant_id, dto).await
    }

    pub async fn get_webhook(&self, id: i64, tenant_id: i64) -> Result<Option<Webhook>, ApiError> {
        self.webhook_repo.find_by_id(id, tenant_id).await
    }

    pub async fn list_webhooks(&self, tenant_id: i64) -> Result<Vec<Webhook>, ApiError> {
        self.webhook_repo.find_by_tenant(tenant_id).await
    }

    pub async fn update_webhook(
        &self,
        id: i64,
        tenant_id: i64,
        dto: UpdateWebhook,
    ) -> Result<Webhook, ApiError> {
        if let Some(ref url) = dto.url {
            self.validate_url(url)?;
        }
        self.webhook_repo.update(id, tenant_id, dto).await
    }

    pub async fn delete_webhook(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.webhook_repo
            .soft_delete(id, tenant_id, deleted_by)
            .await
    }

    pub async fn restore_webhook(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.webhook_repo.restore(id, tenant_id).await
    }

    pub async fn list_deleted_webhooks(&self, tenant_id: i64) -> Result<Vec<Webhook>, ApiError> {
        self.webhook_repo.find_deleted(tenant_id).await
    }

    pub async fn destroy_webhook(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.webhook_repo.destroy(id, tenant_id).await
    }

    // --- Delivery ---

    pub async fn list_deliveries(
        &self,
        webhook_id: i64,
        tenant_id: i64,
        page: i64,
        per_page: i64,
    ) -> Result<PaginatedResult<WebhookDelivery>, ApiError> {
        self.delivery_repo
            .find_by_webhook(webhook_id, tenant_id, page, per_page)
            .await
    }

    /// Trigger webhook deliveries for a domain event.
    /// Creates delivery records synchronously, then spawns async HTTP POSTs.
    pub async fn trigger(&self, event: &DomainEvent) -> Result<(), ApiError> {
        let tenant_id = event.tenant_id();
        let event_type = event.event_type().to_string();
        let webhooks = self
            .webhook_repo
            .find_active_by_event(tenant_id, &event_type)
            .await?;

        if webhooks.is_empty() {
            return Ok(());
        }

        let payload = serde_json::to_string(event).unwrap_or_else(|_| "{}".to_string());

        for webhook in webhooks {
            let delivery = WebhookDelivery {
                id: 0,
                webhook_id: webhook.id,
                tenant_id,
                event_type: event_type.clone(),
                payload: payload.clone(),
                status: DeliveryStatus::Pending,
                http_status: None,
                response_body: None,
                error_message: None,
                attempt_count: 0,
                scheduled_at: None,
                created_at: Utc::now(),
                delivered_at: None,
            };

            let created = self.delivery_repo.create(delivery).await?;

            // Spawn async delivery so event bus isn't blocked
            let repo = self.delivery_repo.clone();
            let client = self.http_client.clone();
            let wh_url = webhook.url.clone();
            let wh_secret = webhook.secret.clone();
            let evt_type = event_type.clone();
            let pl = payload.clone();

            tokio::spawn(async move {
                if let Err(e) = deliver_webhook(
                    &client, &repo, created.id, tenant_id, &wh_url, &wh_secret, &evt_type, &pl,
                )
                .await
                {
                    tracing::warn!("Webhook delivery failed for {}: {}", wh_url, e);
                }
            });
        }

        Ok(())
    }

    /// Retry a failed delivery immediately.
    pub async fn retry_delivery(
        &self,
        delivery_id: i64,
        tenant_id: i64,
    ) -> Result<WebhookDelivery, ApiError> {
        let delivery = self
            .delivery_repo
            .find_by_id(delivery_id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Delivery {} not found", delivery_id)))?;

        let webhook = self
            .webhook_repo
            .find_by_id(delivery.webhook_id, tenant_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!("Webhook {} not found", delivery.webhook_id))
            })?;

        if delivery.attempt_count >= 5 {
            return Err(ApiError::BadRequest(
                "Maximum retry attempts exceeded".to_string(),
            ));
        }

        deliver_webhook(
            &self.http_client,
            &self.delivery_repo,
            delivery_id,
            tenant_id,
            &webhook.url,
            &webhook.secret,
            &delivery.event_type,
            &delivery.payload,
        )
        .await?;

        self.delivery_repo
            .find_by_id(delivery_id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Delivery {} not found", delivery_id)))
    }

    // --- Validation ---

    fn validate_create(&self, dto: &CreateWebhook) -> Result<(), ApiError> {
        self.validate_url(&dto.url)?;
        if let Some(ref secret) = dto.secret {
            if secret.len() < 8 {
                return Err(ApiError::Validation(
                    "Webhook secret must be at least 8 characters".to_string(),
                ));
            }
        }
        Ok(())
    }

    fn validate_url(&self, url: &str) -> Result<(), ApiError> {
        if !url.starts_with("https://") {
            return Err(ApiError::Validation(
                "Webhook URL must use HTTPS".to_string(),
            ));
        }
        Ok(())
    }
}

/// Deliver a webhook payload via HTTP POST with HMAC-SHA256 signature.
#[allow(clippy::too_many_arguments)]
async fn deliver_webhook(
    client: &Client,
    repo: &Arc<dyn WebhookDeliveryRepository>,
    delivery_id: i64,
    tenant_id: i64,
    url: &str,
    secret: &str,
    event_type: &str,
    payload: &str,
) -> Result<(), ApiError> {
    let signature = compute_signature(secret, payload);
    let timestamp = Utc::now().timestamp().to_string();

    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .header("X-Webhook-Signature", format!("sha256={}", signature))
        .header("X-Webhook-Event", event_type)
        .header("X-Webhook-Timestamp", &timestamp)
        .body(payload.to_string())
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status().as_u16() as i32;
            let body = resp.text().await.unwrap_or_default();
            let is_success = (200..300).contains(&status);

            repo.update_status(
                delivery_id,
                tenant_id,
                if is_success {
                    DeliveryStatus::Delivered
                } else {
                    DeliveryStatus::Failed
                },
                Some(status),
                Some(body),
                if is_success {
                    None
                } else {
                    Some(format!("HTTP {}", status))
                },
            )
            .await?;

            if !is_success {
                return Err(ApiError::Internal(format!("HTTP {}", status)));
            }

            Ok(())
        }
        Err(e) => {
            let err_msg = e.to_string();
            repo.update_status(
                delivery_id,
                tenant_id,
                DeliveryStatus::Failed,
                None,
                None,
                Some(err_msg.clone()),
            )
            .await?;
            Err(ApiError::Internal(err_msg))
        }
    }
}

/// Compute HMAC-SHA256 signature (Stripe-style: hex-encoded).
fn compute_signature(secret: &str, payload: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
    mac.update(payload.as_bytes());
    let result = mac.finalize();
    hex::encode(result.into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_service() -> WebhookService {
        let wh_repo = Arc::new(crate::domain::webhook::repository::InMemoryWebhookRepository::new())
            as BoxWebhookRepository;
        let dl_repo =
            Arc::new(crate::domain::webhook::repository::InMemoryWebhookDeliveryRepository::new())
                as BoxWebhookDeliveryRepository;
        WebhookService::new(wh_repo, dl_repo)
    }

    #[tokio::test]
    async fn test_create_webhook() {
        let svc = make_service();
        let wh = svc
            .create_webhook(
                1,
                CreateWebhook {
                    url: "https://example.com/webhook".to_string(),
                    description: None,
                    event_types: vec!["*".to_string()],
                    secret: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(wh.url, "https://example.com/webhook");
        assert!(!wh.secret.is_empty());
    }

    #[tokio::test]
    async fn test_create_webhook_http_validation() {
        let svc = make_service();
        let result = svc
            .create_webhook(
                1,
                CreateWebhook {
                    url: "http://example.com".to_string(),
                    description: None,
                    event_types: vec![],
                    secret: None,
                },
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_webhooks() {
        let svc = make_service();
        svc.create_webhook(
            1,
            CreateWebhook {
                url: "https://a.com".to_string(),
                description: None,
                event_types: vec![],
                secret: None,
            },
        )
        .await
        .unwrap();
        let list = svc.list_webhooks(1).await.unwrap();
        assert_eq!(list.len(), 1);
    }

    #[tokio::test]
    async fn test_update_webhook() {
        let svc = make_service();
        svc.create_webhook(
            1,
            CreateWebhook {
                url: "https://a.com".to_string(),
                description: None,
                event_types: vec![],
                secret: None,
            },
        )
        .await
        .unwrap();

        let updated = svc
            .update_webhook(
                1,
                1,
                UpdateWebhook {
                    url: Some("https://b.com".to_string()),
                    description: None,
                    event_types: None,
                    status: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(updated.url, "https://b.com");
    }

    #[tokio::test]
    async fn test_trigger_no_matching_webhooks() {
        let svc = make_service();
        let event = DomainEvent::Custom {
            name: "test".to_string(),
            tenant_id: 1,
            payload: "{}".to_string(),
        };
        let result = svc.trigger(&event).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_compute_signature() {
        let sig = compute_signature("secret", r#"{"test":true}"#);
        assert_eq!(sig.len(), 64); // hex-encoded SHA256 = 64 chars
    }
}
