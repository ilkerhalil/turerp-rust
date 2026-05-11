//! Change Data Capture (CDC) service using PostgreSQL LISTEN/NOTIFY
//!
//! Provides a `CdcListener` that subscribes to PostgreSQL notification channels,
//! parses CDC payloads, converts them to `DomainEvent` types, and publishes them
//! to the application's `EventBus`.

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::postgres::PgListener;
use tokio::sync::watch;

use crate::common::events::{DomainEvent, EventBus};
use crate::config::CdcConfig;

/// CDC operation type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum CdcOperation {
    Insert,
    Update,
    Delete,
}

/// Parsed CDC event from a PostgreSQL notification payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdcEvent {
    pub table: String,
    pub operation: CdcOperation,
    pub id: i64,
    pub tenant_id: i64,
    pub old_data: Option<Value>,
    pub new_data: Option<Value>,
    pub timestamp: DateTime<Utc>,
}

/// CDC listener wrapper around `sqlx::PgListener` with reconnect logic
#[derive(Debug)]
pub struct CdcListener {
    pool: Arc<sqlx::PgPool>,
    channels: Vec<String>,
    last_event_time: parking_lot::Mutex<Option<DateTime<Utc>>>,
    active: parking_lot::Mutex<bool>,
}

impl CdcListener {
    /// Create a new CDC listener from config
    pub fn new(pool: Arc<sqlx::PgPool>, config: &CdcConfig) -> Self {
        Self {
            pool,
            channels: config.channels.clone(),
            last_event_time: parking_lot::Mutex::new(None),
            active: parking_lot::Mutex::new(false),
        }
    }

    /// List of channels this listener subscribes to
    pub fn active_channels(&self) -> Vec<String> {
        self.channels.clone()
    }

    /// Timestamp of the last successfully handled CDC event
    pub fn last_event_time(&self) -> Option<DateTime<Utc>> {
        *self.last_event_time.lock()
    }

    /// Whether the listener is currently connected and active
    pub fn is_active(&self) -> bool {
        *self.active.lock()
    }

    /// Run the CDC listener loop until shutdown is signaled
    pub async fn run(&self, event_bus: Arc<dyn EventBus>, mut shutdown: watch::Receiver<bool>) {
        if self.channels.is_empty() {
            tracing::info!("CDC listener has no channels configured, not starting");
            return;
        }

        let mut backoff_secs = 1u64;
        loop {
            tracing::info!("CDC listener connecting to channels: {:?}", self.channels);
            match self
                .connect_and_listen(event_bus.clone(), &mut shutdown)
                .await
            {
                Ok(()) => {
                    tracing::info!("CDC listener stopped gracefully");
                    break;
                }
                Err(e) => {
                    tracing::error!(
                        "CDC listener error: {}. Reconnecting in {}s",
                        e,
                        backoff_secs
                    );
                    tokio::select! {
                        _ = tokio::time::sleep(Duration::from_secs(backoff_secs)) => {},
                        _ = shutdown.changed() => {
                            if *shutdown.borrow() {
                                tracing::info!("CDC listener shutting down on signal");
                                break;
                            }
                        }
                    }
                    backoff_secs = (backoff_secs * 2).min(60);
                }
            }
        }
    }

    async fn connect_and_listen(
        &self,
        event_bus: Arc<dyn EventBus>,
        shutdown: &mut watch::Receiver<bool>,
    ) -> Result<(), String> {
        let mut listener = PgListener::connect_with(&self.pool)
            .await
            .map_err(|e| format!("Failed to connect PgListener: {}", e))?;

        for ch in &self.channels {
            listener
                .listen(ch)
                .await
                .map_err(|e| format!("Failed to listen on channel {}: {}", ch, e))?;
        }

        *self.active.lock() = true;
        tracing::info!("CDC listener active on {} channels", self.channels.len());

        loop {
            tokio::select! {
                notification = listener.recv() => {
                    match notification {
                        Ok(notif) => {
                            if let Err(e) = self.handle_notification(&notif, &event_bus).await {
                                tracing::warn!("Failed to handle CDC notification: {}", e);
                            }
                        }
                        Err(e) => {
                            tracing::error!("CDC listener recv error: {}", e);
                            *self.active.lock() = false;
                            return Err(format!("Connection lost: {}", e));
                        }
                    }
                }
                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        *self.active.lock() = false;
                        return Ok(());
                    }
                }
            }
        }
    }

    async fn handle_notification(
        &self,
        notif: &sqlx::postgres::PgNotification,
        event_bus: &Arc<dyn EventBus>,
    ) -> Result<(), String> {
        let payload: Value = serde_json::from_str(notif.payload())
            .map_err(|e| format!("Invalid JSON payload: {}", e))?;

        let cdc_event = parse_cdc_event(payload)?;
        *self.last_event_time.lock() = Some(cdc_event.timestamp);

        if let Some(domain_event) = convert_to_domain_event(&cdc_event) {
            event_bus
                .publish(domain_event)
                .await
                .map_err(|e| format!("EventBus publish failed: {}", e))?;
        }

        Ok(())
    }
}

/// Parse a JSON payload into a `CdcEvent`
pub fn parse_cdc_event(payload: Value) -> Result<CdcEvent, String> {
    let table = payload
        .get("table")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'table' in CDC payload")?
        .to_string();

    let operation = payload
        .get("operation")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'operation' in CDC payload")?;
    let operation = match operation {
        "INSERT" => CdcOperation::Insert,
        "UPDATE" => CdcOperation::Update,
        "DELETE" => CdcOperation::Delete,
        other => return Err(format!("Unknown CDC operation: {}", other)),
    };

    let id = payload
        .get("id")
        .and_then(|v| v.as_i64())
        .ok_or("Missing 'id' in CDC payload")?;

    let tenant_id = payload
        .get("tenant_id")
        .and_then(|v| v.as_i64())
        .ok_or("Missing 'tenant_id' in CDC payload")?;

    let old_data = payload
        .get("old")
        .and_then(|v| if v.is_null() { None } else { Some(v.clone()) });
    let new_data = payload
        .get("new")
        .and_then(|v| if v.is_null() { None } else { Some(v.clone()) });

    Ok(CdcEvent {
        table,
        operation,
        id,
        tenant_id,
        old_data,
        new_data,
        timestamp: Utc::now(),
    })
}

/// Convert a `CdcEvent` into a `DomainEvent` when applicable
pub fn convert_to_domain_event(cdc: &CdcEvent) -> Option<DomainEvent> {
    match (cdc.table.as_str(), &cdc.operation) {
        ("invoices", CdcOperation::Insert) => {
            let amount = cdc
                .new_data
                .as_ref()
                .and_then(|d| d.get("total_amount"))
                .and_then(|v| v.as_str())
                .unwrap_or("0")
                .to_string();
            let currency = cdc
                .new_data
                .as_ref()
                .and_then(|d| d.get("currency"))
                .and_then(|v| v.as_str())
                .unwrap_or("TRY")
                .to_string();
            Some(DomainEvent::InvoiceCreated {
                invoice_id: cdc.id,
                tenant_id: cdc.tenant_id,
                amount,
                currency,
            })
        }
        ("payments", CdcOperation::Insert) => {
            let invoice_id = cdc
                .new_data
                .as_ref()
                .and_then(|d| d.get("invoice_id"))
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let amount = cdc
                .new_data
                .as_ref()
                .and_then(|d| d.get("amount"))
                .and_then(|v| v.as_str())
                .unwrap_or("0")
                .to_string();
            Some(DomainEvent::PaymentReceived {
                payment_id: cdc.id,
                invoice_id,
                tenant_id: cdc.tenant_id,
                amount,
            })
        }
        ("stock_movements", CdcOperation::Insert) => {
            let product_id = cdc
                .new_data
                .as_ref()
                .and_then(|d| d.get("product_id"))
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let warehouse_id = cdc
                .new_data
                .as_ref()
                .and_then(|d| d.get("warehouse_id"))
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let quantity = cdc
                .new_data
                .as_ref()
                .and_then(|d| d.get("quantity"))
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let direction = cdc
                .new_data
                .as_ref()
                .and_then(|d| d.get("movement_type"))
                .and_then(|v| v.as_str())
                .unwrap_or("In")
                .to_string();
            Some(DomainEvent::StockMoved {
                movement_id: cdc.id,
                tenant_id: cdc.tenant_id,
                product_id,
                warehouse_id,
                quantity,
                direction,
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_cdc_event_insert() {
        let payload = json!({
            "table": "invoices",
            "operation": "INSERT",
            "id": 1,
            "tenant_id": 42,
            "old": null,
            "new": {"total_amount": "100.00", "currency": "TRY"}
        });
        let event = parse_cdc_event(payload).unwrap();
        assert_eq!(event.table, "invoices");
        assert_eq!(event.operation, CdcOperation::Insert);
        assert_eq!(event.id, 1);
        assert_eq!(event.tenant_id, 42);
        assert!(event.old_data.is_none());
        assert!(event.new_data.is_some());
    }

    #[test]
    fn test_parse_cdc_event_update() {
        let payload = json!({
            "table": "products",
            "operation": "UPDATE",
            "id": 5,
            "tenant_id": 2,
            "old": {"name": "Old Name"},
            "new": {"name": "New Name"}
        });
        let event = parse_cdc_event(payload).unwrap();
        assert_eq!(event.operation, CdcOperation::Update);
        assert_eq!(event.old_data.as_ref().unwrap()["name"], "Old Name");
        assert_eq!(event.new_data.as_ref().unwrap()["name"], "New Name");
    }

    #[test]
    fn test_parse_cdc_event_delete() {
        let payload = json!({
            "table": "cari",
            "operation": "DELETE",
            "id": 10,
            "tenant_id": 3,
            "old": {"name": "Deleted"},
            "new": null
        });
        let event = parse_cdc_event(payload).unwrap();
        assert_eq!(event.operation, CdcOperation::Delete);
        assert!(event.new_data.is_none());
    }

    #[test]
    fn test_parse_cdc_event_missing_tenant_id_fails() {
        let payload = json!({
            "table": "stock_movements",
            "operation": "INSERT",
            "id": 5,
            "old": null,
            "new": {"product_id": 1}
        });
        assert!(parse_cdc_event(payload).is_err());
    }

    #[test]
    fn test_parse_cdc_event_invalid_operation() {
        let payload = json!({
            "table": "invoices",
            "operation": "TRUNCATE",
            "id": 1,
            "tenant_id": 1,
            "old": null,
            "new": null
        });
        assert!(parse_cdc_event(payload).is_err());
    }

    #[test]
    fn test_convert_invoice_created() {
        let cdc = CdcEvent {
            table: "invoices".to_string(),
            operation: CdcOperation::Insert,
            id: 1,
            tenant_id: 1,
            old_data: None,
            new_data: Some(json!({"total_amount": "500.00", "currency": "USD"})),
            timestamp: Utc::now(),
        };
        let domain = convert_to_domain_event(&cdc).unwrap();
        let DomainEvent::InvoiceCreated {
            invoice_id,
            tenant_id,
            amount,
            currency,
        } = domain
        else {
            panic!("Expected InvoiceCreated, got {:?}", domain);
        };
        assert_eq!(invoice_id, 1);
        assert_eq!(tenant_id, 1);
        assert_eq!(amount, "500.00");
        assert_eq!(currency, "USD");
    }

    #[test]
    fn test_convert_payment_received() {
        let cdc = CdcEvent {
            table: "payments".to_string(),
            operation: CdcOperation::Insert,
            id: 2,
            tenant_id: 1,
            old_data: None,
            new_data: Some(json!({"invoice_id": 10, "amount": "250.00"})),
            timestamp: Utc::now(),
        };
        let domain = convert_to_domain_event(&cdc).unwrap();
        let DomainEvent::PaymentReceived {
            payment_id,
            invoice_id,
            tenant_id,
            amount,
        } = domain
        else {
            panic!("Expected PaymentReceived, got {:?}", domain);
        };
        assert_eq!(payment_id, 2);
        assert_eq!(invoice_id, 10);
        assert_eq!(tenant_id, 1);
        assert_eq!(amount, "250.00");
    }

    #[test]
    fn test_convert_stock_moved() {
        let cdc = CdcEvent {
            table: "stock_movements".to_string(),
            operation: CdcOperation::Insert,
            id: 3,
            tenant_id: 1,
            old_data: None,
            new_data: Some(
                json!({"product_id": 5, "warehouse_id": 2, "quantity": 100, "movement_type": "In"}),
            ),
            timestamp: Utc::now(),
        };
        let domain = convert_to_domain_event(&cdc).unwrap();
        let DomainEvent::StockMoved {
            movement_id,
            tenant_id,
            product_id,
            warehouse_id,
            quantity,
            direction,
        } = domain
        else {
            panic!("Expected StockMoved, got {:?}", domain);
        };
        assert_eq!(movement_id, 3);
        assert_eq!(tenant_id, 1);
        assert_eq!(product_id, 5);
        assert_eq!(warehouse_id, 2);
        assert_eq!(quantity, 100);
        assert_eq!(direction, "In");
    }

    #[test]
    fn test_convert_skip_unsupported_table() {
        let cdc = CdcEvent {
            table: "journal_entries".to_string(),
            operation: CdcOperation::Insert,
            id: 1,
            tenant_id: 1,
            old_data: None,
            new_data: Some(json!({})),
            timestamp: Utc::now(),
        };
        assert!(convert_to_domain_event(&cdc).is_none());
    }

    #[test]
    fn test_convert_skip_update() {
        let cdc = CdcEvent {
            table: "invoices".to_string(),
            operation: CdcOperation::Update,
            id: 1,
            tenant_id: 1,
            old_data: None,
            new_data: Some(json!({})),
            timestamp: Utc::now(),
        };
        assert!(convert_to_domain_event(&cdc).is_none());
    }

    #[tokio::test]
    async fn test_channel_subscription_logic() {
        let pool = Arc::new(
            sqlx::postgres::PgPoolOptions::new()
                .max_connections(1)
                .connect_lazy("postgres://localhost/dummy")
                .expect("lazy pool"),
        );
        let config = CdcConfig {
            enabled: true,
            channels: vec!["invoice_changes".to_string(), "payment_changes".to_string()],
        };
        let listener = CdcListener::new(pool, &config);
        assert_eq!(listener.active_channels().len(), 2);
        assert!(listener
            .active_channels()
            .contains(&"invoice_changes".to_string()));
        assert!(!listener.is_active());
        assert!(listener.last_event_time().is_none());
    }
}
