//! Webhook repository traits and in-memory implementations

use async_trait::async_trait;
use parking_lot::RwLock;
use std::sync::Arc;

use crate::common::soft_delete::SoftDeletable;
use crate::common::PaginatedResult;
use crate::domain::webhook::model::{
    CreateWebhook, DeliveryStatus, UpdateWebhook, Webhook, WebhookDelivery,
};
use crate::error::ApiError;

/// Repository trait for webhook endpoint operations
#[async_trait]
pub trait WebhookRepository: Send + Sync {
    async fn create(&self, tenant_id: i64, webhook: CreateWebhook) -> Result<Webhook, ApiError>;

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Webhook>, ApiError>;

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Webhook>, ApiError>;

    async fn find_active_by_event(
        &self,
        tenant_id: i64,
        event_type: &str,
    ) -> Result<Vec<Webhook>, ApiError>;

    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        dto: UpdateWebhook,
    ) -> Result<Webhook, ApiError>;

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError>;

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Webhook>, ApiError>;

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
}

/// Repository trait for webhook delivery operations
#[async_trait]
pub trait WebhookDeliveryRepository: Send + Sync {
    async fn create(&self, delivery: WebhookDelivery) -> Result<WebhookDelivery, ApiError>;

    async fn find_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<WebhookDelivery>, ApiError>;

    async fn find_by_webhook(
        &self,
        webhook_id: i64,
        tenant_id: i64,
        page: i64,
        per_page: i64,
    ) -> Result<PaginatedResult<WebhookDelivery>, ApiError>;

    async fn find_pending_retries(
        &self,
        before: chrono::DateTime<chrono::Utc>,
        limit: i64,
    ) -> Result<Vec<WebhookDelivery>, ApiError>;

    async fn update_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: DeliveryStatus,
        http_status: Option<i32>,
        response_body: Option<String>,
        error: Option<String>,
    ) -> Result<(), ApiError>;

    async fn increment_attempt(
        &self,
        id: i64,
        tenant_id: i64,
        next_retry: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<(), ApiError>;
}

pub type BoxWebhookRepository = Arc<dyn WebhookRepository>;
pub type BoxWebhookDeliveryRepository = Arc<dyn WebhookDeliveryRepository>;

// --- InMemory implementations ---

/// Inner state for InMemoryWebhookRepository
struct WebhookInner {
    webhooks: Vec<Webhook>,
    next_id: i64,
}

/// In-memory webhook repository
pub struct InMemoryWebhookRepository {
    inner: RwLock<WebhookInner>,
}

impl InMemoryWebhookRepository {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(WebhookInner {
                webhooks: Vec::new(),
                next_id: 1,
            }),
        }
    }
}

impl Default for InMemoryWebhookRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WebhookRepository for InMemoryWebhookRepository {
    async fn create(&self, tenant_id: i64, webhook: CreateWebhook) -> Result<Webhook, ApiError> {
        let mut inner = self.inner.write();
        let id = inner.next_id;
        inner.next_id += 1;

        let now = chrono::Utc::now();
        let wh = Webhook {
            id,
            tenant_id,
            url: webhook.url,
            description: webhook.description,
            event_types: webhook.event_types,
            secret: webhook.secret.unwrap_or_else(generate_secret),
            status: crate::domain::webhook::model::WebhookStatus::Active,
            created_at: now,
            updated_at: now,
            deleted_at: None,
            deleted_by: None,
        };
        inner.webhooks.push(wh.clone());
        Ok(wh)
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Webhook>, ApiError> {
        let inner = self.inner.read();
        Ok(inner
            .webhooks
            .iter()
            .find(|w| w.id == id && w.tenant_id == tenant_id && w.deleted_at.is_none())
            .cloned())
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Webhook>, ApiError> {
        let inner = self.inner.read();
        Ok(inner
            .webhooks
            .iter()
            .filter(|w| w.tenant_id == tenant_id && w.deleted_at.is_none())
            .cloned()
            .collect())
    }

    async fn find_active_by_event(
        &self,
        tenant_id: i64,
        event_type: &str,
    ) -> Result<Vec<Webhook>, ApiError> {
        let inner = self.inner.read();
        Ok(inner
            .webhooks
            .iter()
            .filter(|w| {
                w.tenant_id == tenant_id
                    && w.deleted_at.is_none()
                    && w.status == crate::domain::webhook::model::WebhookStatus::Active
                    && (w.event_types.is_empty()
                        || w.event_types.iter().any(|t| t == "*" || t == event_type))
            })
            .cloned()
            .collect())
    }

    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        dto: UpdateWebhook,
    ) -> Result<Webhook, ApiError> {
        let mut inner = self.inner.write();
        let wh = inner
            .webhooks
            .iter_mut()
            .find(|w| w.id == id && w.tenant_id == tenant_id && w.deleted_at.is_none())
            .ok_or_else(|| ApiError::NotFound(format!("Webhook {} not found", id)))?;

        if let Some(url) = dto.url {
            wh.url = url;
        }
        if let Some(desc) = dto.description {
            wh.description = Some(desc);
        }
        if let Some(ets) = dto.event_types {
            wh.event_types = ets;
        }
        if let Some(status) = dto.status {
            wh.status = status;
        }
        wh.updated_at = chrono::Utc::now();
        Ok(wh.clone())
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.write();
        inner
            .webhooks
            .retain(|w| !(w.id == id && w.tenant_id == tenant_id));
        Ok(())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.write();
        let wh = inner
            .webhooks
            .iter_mut()
            .find(|w| w.id == id && w.tenant_id == tenant_id && w.deleted_at.is_none())
            .ok_or_else(|| ApiError::NotFound(format!("Webhook {} not found", id)))?;
        wh.mark_deleted(deleted_by);
        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.write();
        let wh = inner
            .webhooks
            .iter_mut()
            .find(|w| w.id == id && w.tenant_id == tenant_id && w.is_deleted())
            .ok_or_else(|| ApiError::NotFound(format!("Deleted webhook {} not found", id)))?;
        wh.restore();
        Ok(())
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Webhook>, ApiError> {
        let inner = self.inner.read();
        Ok(inner
            .webhooks
            .iter()
            .filter(|w| w.tenant_id == tenant_id && w.is_deleted())
            .cloned()
            .collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.write();
        let len_before = inner.webhooks.len();
        inner
            .webhooks
            .retain(|w| !(w.id == id && w.tenant_id == tenant_id && w.is_deleted()));

        if inner.webhooks.len() == len_before {
            return Err(ApiError::NotFound(format!(
                "Deleted webhook {} not found",
                id
            )));
        }
        Ok(())
    }
}

/// Inner state for InMemoryWebhookDeliveryRepository
struct DeliveryInner {
    deliveries: Vec<WebhookDelivery>,
    next_id: i64,
}

/// In-memory webhook delivery repository
pub struct InMemoryWebhookDeliveryRepository {
    inner: RwLock<DeliveryInner>,
}

impl InMemoryWebhookDeliveryRepository {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(DeliveryInner {
                deliveries: Vec::new(),
                next_id: 1,
            }),
        }
    }
}

impl Default for InMemoryWebhookDeliveryRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WebhookDeliveryRepository for InMemoryWebhookDeliveryRepository {
    async fn create(&self, delivery: WebhookDelivery) -> Result<WebhookDelivery, ApiError> {
        let mut inner = self.inner.write();
        let mut d = delivery;
        d.id = inner.next_id;
        inner.next_id += 1;
        inner.deliveries.push(d.clone());
        Ok(d)
    }

    async fn find_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<WebhookDelivery>, ApiError> {
        let inner = self.inner.read();
        Ok(inner
            .deliveries
            .iter()
            .find(|d| d.id == id && d.tenant_id == tenant_id)
            .cloned())
    }

    async fn find_by_webhook(
        &self,
        webhook_id: i64,
        tenant_id: i64,
        page: i64,
        per_page: i64,
    ) -> Result<PaginatedResult<WebhookDelivery>, ApiError> {
        let inner = self.inner.read();
        let filtered: Vec<_> = inner
            .deliveries
            .iter()
            .filter(|d| d.webhook_id == webhook_id && d.tenant_id == tenant_id)
            .cloned()
            .collect();

        let total = filtered.len() as u64;
        let offset = ((page - 1) * per_page) as usize;
        let items = filtered
            .into_iter()
            .skip(offset)
            .take(per_page as usize)
            .collect();

        Ok(PaginatedResult::new(
            items,
            page as u32,
            per_page as u32,
            total,
        ))
    }

    async fn find_pending_retries(
        &self,
        before: chrono::DateTime<chrono::Utc>,
        limit: i64,
    ) -> Result<Vec<WebhookDelivery>, ApiError> {
        let inner = self.inner.read();
        Ok(inner
            .deliveries
            .iter()
            .filter(|d| {
                (d.status == DeliveryStatus::Pending || d.status == DeliveryStatus::Retrying)
                    && d.scheduled_at.is_none_or(|s| s <= before)
            })
            .take(limit as usize)
            .cloned()
            .collect())
    }

    async fn update_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: DeliveryStatus,
        http_status: Option<i32>,
        response_body: Option<String>,
        error: Option<String>,
    ) -> Result<(), ApiError> {
        let mut inner = self.inner.write();
        let d = inner
            .deliveries
            .iter_mut()
            .find(|d| d.id == id && d.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Delivery {} not found", id)))?;
        d.status = status;
        if let Some(hs) = http_status {
            d.http_status = Some(hs);
        }
        if let Some(rb) = response_body {
            d.response_body = Some(rb);
        }
        if let Some(e) = error {
            d.error_message = Some(e);
        }
        if status == DeliveryStatus::Delivered {
            d.delivered_at = Some(chrono::Utc::now());
        }
        Ok(())
    }

    async fn increment_attempt(
        &self,
        id: i64,
        tenant_id: i64,
        next_retry: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<(), ApiError> {
        let mut inner = self.inner.write();
        let d = inner
            .deliveries
            .iter_mut()
            .find(|d| d.id == id && d.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Delivery {} not found", id)))?;
        d.attempt_count += 1;
        d.scheduled_at = next_retry;
        d.status = DeliveryStatus::Retrying;
        Ok(())
    }
}

fn generate_secret() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..32)
        .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_create() -> CreateWebhook {
        CreateWebhook {
            url: "https://example.com/webhook".to_string(),
            description: Some("Test".to_string()),
            event_types: vec!["*".to_string()],
            secret: None,
        }
    }

    #[tokio::test]
    async fn test_create_and_find() {
        let repo = InMemoryWebhookRepository::new();
        let wh = repo.create(1, make_create()).await.unwrap();
        assert_eq!(wh.id, 1);

        let found = repo.find_by_id(1, 1).await.unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn test_find_active_by_event() {
        let repo = InMemoryWebhookRepository::new();
        repo.create(1, make_create()).await.unwrap();

        let active = repo
            .find_active_by_event(1, "invoice_created")
            .await
            .unwrap();
        assert_eq!(active.len(), 1);
    }

    #[tokio::test]
    async fn test_update() {
        let repo = InMemoryWebhookRepository::new();
        repo.create(1, make_create()).await.unwrap();

        let updated = repo
            .update(
                1,
                1,
                UpdateWebhook {
                    url: Some("https://new.com".to_string()),
                    description: None,
                    event_types: None,
                    status: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(updated.url, "https://new.com");
    }

    #[tokio::test]
    async fn test_soft_delete() {
        let repo = InMemoryWebhookRepository::new();
        repo.create(1, make_create()).await.unwrap();

        repo.soft_delete(1, 1, 42).await.unwrap();
        let found = repo.find_by_id(1, 1).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_delivery_create_and_find() {
        let repo = InMemoryWebhookDeliveryRepository::new();
        let d = WebhookDelivery {
            id: 0,
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
            created_at: chrono::Utc::now(),
            delivered_at: None,
        };
        let created = repo.create(d).await.unwrap();
        assert_eq!(created.id, 1);

        let found = repo.find_by_id(1, 1).await.unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn test_delivery_update_status() {
        let repo = InMemoryWebhookDeliveryRepository::new();
        let d = WebhookDelivery {
            id: 0,
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
            created_at: chrono::Utc::now(),
            delivered_at: None,
        };
        repo.create(d).await.unwrap();

        repo.update_status(1, 1, DeliveryStatus::Delivered, Some(200), None, None)
            .await
            .unwrap();

        let found = repo.find_by_id(1, 1).await.unwrap().unwrap();
        assert_eq!(found.status, DeliveryStatus::Delivered);
        assert_eq!(found.http_status, Some(200));
        assert!(found.delivered_at.is_some());
    }
}
