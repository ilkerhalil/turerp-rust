//! Webhook subscriber for EventBus integration

use std::sync::Arc;

use crate::common::events::{DomainEvent, EventSubscriber};
use crate::domain::webhook::service::WebhookService;

/// EventBus subscriber that forwards all domain events to webhook deliveries.
pub struct WebhookSubscriber {
    webhook_service: Arc<WebhookService>,
}

impl WebhookSubscriber {
    pub fn new(webhook_service: Arc<WebhookService>) -> Self {
        Self { webhook_service }
    }
}

#[async_trait::async_trait]
impl EventSubscriber for WebhookSubscriber {
    async fn handle(&self, event: &DomainEvent) -> Result<(), String> {
        self.webhook_service
            .trigger(event)
            .await
            .map_err(|e| e.to_string())
    }

    fn subscribed_to(&self) -> Vec<String> {
        vec!["*".to_string()]
    }

    fn name(&self) -> &str {
        "WebhookSubscriber"
    }
}
