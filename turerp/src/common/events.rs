//! Event-driven architecture with EventBus, Outbox pattern, and Dead Letter Queue
//!
//! Provides an `EventBus` trait for publishing/subscribing to domain events,
//! an outbox table for reliable event delivery, and a DLQ for failed events.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Domain event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum DomainEvent {
    /// Invoice was created
    InvoiceCreated {
        invoice_id: i64,
        tenant_id: i64,
        amount: String,
        currency: String,
    },
    /// Payment was received for an invoice
    PaymentReceived {
        payment_id: i64,
        invoice_id: i64,
        tenant_id: i64,
        amount: String,
    },
    /// Stock was moved (in/out/transfer)
    StockMoved {
        movement_id: i64,
        tenant_id: i64,
        product_id: i64,
        warehouse_id: i64,
        quantity: i64,
        direction: String,
    },
    /// Employee was hired
    EmployeeHired {
        employee_id: i64,
        tenant_id: i64,
        department: Option<String>,
    },
    /// Sales order was created
    SalesOrderCreated {
        order_id: i64,
        tenant_id: i64,
        customer_id: i64,
        total: String,
    },
    /// Purchase order was approved
    PurchaseOrderApproved {
        order_id: i64,
        tenant_id: i64,
        supplier_id: i64,
    },
    // --- P0 cross-module integration events ---
    /// e-Fatura document was created (from an invoice)
    EFaturaCreated {
        tenant_id: i64,
        fatura_id: i64,
        uuid: String,
    },
    /// e-Fatura document was sent to GIB
    EFaturaSent {
        tenant_id: i64,
        fatura_id: i64,
        uuid: String,
    },
    /// e-Fatura document was cancelled
    EFaturaCancelled {
        tenant_id: i64,
        fatura_id: i64,
        reason: String,
    },
    /// e-Defter ledger period was created
    EDefterPeriodCreated {
        tenant_id: i64,
        period_id: i64,
        year: i32,
        month: u32,
    },
    /// e-Defter ledger period was signed (berat)
    EDefterPeriodSigned { tenant_id: i64, period_id: i64 },
    /// e-Defter ledger period was sent to saklayici
    EDefterPeriodSent { tenant_id: i64, period_id: i64 },
    /// Tax period was calculated
    TaxPeriodCalculated {
        tenant_id: i64,
        period_id: i64,
        tax_type: String,
    },
    /// Tax period was filed
    TaxPeriodFiled {
        tenant_id: i64,
        period_id: i64,
        tax_type: String,
    },

    /// Custom event for extensibility
    Custom {
        name: String,
        tenant_id: i64,
        payload: String,
    },
}

impl DomainEvent {
    pub fn tenant_id(&self) -> i64 {
        match self {
            DomainEvent::InvoiceCreated { tenant_id, .. } => *tenant_id,
            DomainEvent::PaymentReceived { tenant_id, .. } => *tenant_id,
            DomainEvent::StockMoved { tenant_id, .. } => *tenant_id,
            DomainEvent::EmployeeHired { tenant_id, .. } => *tenant_id,
            DomainEvent::SalesOrderCreated { tenant_id, .. } => *tenant_id,
            DomainEvent::PurchaseOrderApproved { tenant_id, .. } => *tenant_id,
            DomainEvent::EFaturaCreated { tenant_id, .. } => *tenant_id,
            DomainEvent::EFaturaSent { tenant_id, .. } => *tenant_id,
            DomainEvent::EFaturaCancelled { tenant_id, .. } => *tenant_id,
            DomainEvent::EDefterPeriodCreated { tenant_id, .. } => *tenant_id,
            DomainEvent::EDefterPeriodSigned { tenant_id, .. } => *tenant_id,
            DomainEvent::EDefterPeriodSent { tenant_id, .. } => *tenant_id,
            DomainEvent::TaxPeriodCalculated { tenant_id, .. } => *tenant_id,
            DomainEvent::TaxPeriodFiled { tenant_id, .. } => *tenant_id,
            DomainEvent::Custom { tenant_id, .. } => *tenant_id,
        }
    }

    pub fn event_type(&self) -> &str {
        match self {
            DomainEvent::InvoiceCreated { .. } => "invoice_created",
            DomainEvent::PaymentReceived { .. } => "payment_received",
            DomainEvent::StockMoved { .. } => "stock_moved",
            DomainEvent::EmployeeHired { .. } => "employee_hired",
            DomainEvent::SalesOrderCreated { .. } => "sales_order_created",
            DomainEvent::PurchaseOrderApproved { .. } => "purchase_order_approved",
            DomainEvent::EFaturaCreated { .. } => "efatura_created",
            DomainEvent::EFaturaSent { .. } => "efatura_sent",
            DomainEvent::EFaturaCancelled { .. } => "efatura_cancelled",
            DomainEvent::EDefterPeriodCreated { .. } => "edefter_period_created",
            DomainEvent::EDefterPeriodSigned { .. } => "edefter_period_signed",
            DomainEvent::EDefterPeriodSent { .. } => "edefter_period_sent",
            DomainEvent::TaxPeriodCalculated { .. } => "tax_period_calculated",
            DomainEvent::TaxPeriodFiled { .. } => "tax_period_filed",
            DomainEvent::Custom { name, .. } => name,
        }
    }
}

/// Outbox event record for reliable delivery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboxEvent {
    pub id: i64,
    pub event: DomainEvent,
    pub aggregate_type: String,
    pub aggregate_id: i64,
    pub tenant_id: i64,
    pub created_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
    pub status: EventStatus,
    pub attempts: u32,
    pub last_error: Option<String>,
}

/// Status of an outbox event
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventStatus {
    Pending,
    Published,
    Failed,
    DeadLettered,
}

/// Dead letter queue entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadLetterEntry {
    pub id: i64,
    pub original_event: DomainEvent,
    pub aggregate_type: String,
    pub aggregate_id: i64,
    pub tenant_id: i64,
    pub error: String,
    pub original_attempts: u32,
    pub dead_lettered_at: DateTime<Utc>,
}

/// Event subscriber trait for handling domain events
#[async_trait::async_trait]
pub trait EventSubscriber: Send + Sync {
    /// Handle a domain event
    async fn handle(&self, event: &DomainEvent) -> Result<(), String>;

    /// List event types this subscriber is interested in
    fn subscribed_to(&self) -> Vec<String>;

    /// Subscriber name for debugging
    fn name(&self) -> &str;
}

/// Event bus trait for publishing and subscribing to events
#[async_trait::async_trait]
pub trait EventBus: Send + Sync {
    /// Publish an event to the bus (writes to outbox)
    async fn publish(&self, event: DomainEvent) -> Result<i64, String>;

    /// Publish multiple events atomically
    async fn publish_batch(&self, events: Vec<DomainEvent>) -> Result<Vec<i64>, String>;

    /// Subscribe a handler to specific event types
    async fn subscribe(&self, subscriber: Arc<dyn EventSubscriber>) -> Result<(), String>;

    /// Process pending outbox events (called by background worker)
    async fn process_outbox(&self, batch_size: u32) -> Result<u64, String>;

    /// Get pending outbox events
    async fn get_pending(&self, limit: u32) -> Result<Vec<OutboxEvent>, String>;

    /// Mark an outbox event as published
    async fn mark_published(&self, event_id: i64) -> Result<(), String>;

    /// Mark an outbox event as failed (moves to DLQ after max attempts)
    async fn mark_failed(&self, event_id: i64, error: &str) -> Result<(), String>;

    /// Get dead lettered events for a tenant
    async fn get_dead_letters(&self, tenant_id: i64) -> Result<Vec<DeadLetterEntry>, String>;

    /// Retry a dead-lettered event
    async fn retry_dead_letter(&self, entry_id: i64) -> Result<(), String>;
}

/// Type alias for boxed event bus
pub type BoxEventBus = Arc<dyn EventBus>;

/// In-memory event bus with outbox and DLQ support
pub struct InMemoryEventBus {
    outbox: parking_lot::RwLock<Vec<OutboxEvent>>,
    dead_letters: parking_lot::RwLock<Vec<DeadLetterEntry>>,
    subscribers: parking_lot::RwLock<Vec<Arc<dyn EventSubscriber>>>,
    next_id: parking_lot::RwLock<i64>,
    next_dlq_id: parking_lot::RwLock<i64>,
    max_attempts: u32,
}

impl InMemoryEventBus {
    pub fn new() -> Self {
        Self {
            outbox: parking_lot::RwLock::new(Vec::new()),
            dead_letters: parking_lot::RwLock::new(Vec::new()),
            subscribers: parking_lot::RwLock::new(Vec::new()),
            next_id: parking_lot::RwLock::new(1),
            next_dlq_id: parking_lot::RwLock::new(1),
            max_attempts: 5,
        }
    }

    pub fn with_max_attempts(max_attempts: u32) -> Self {
        Self {
            outbox: parking_lot::RwLock::new(Vec::new()),
            dead_letters: parking_lot::RwLock::new(Vec::new()),
            subscribers: parking_lot::RwLock::new(Vec::new()),
            next_id: parking_lot::RwLock::new(1),
            next_dlq_id: parking_lot::RwLock::new(1),
            max_attempts,
        }
    }

    fn allocate_id(&self) -> i64 {
        let mut id = self.next_id.write();
        let event_id = *id;
        *id += 1;
        event_id
    }

    fn allocate_dlq_id(&self) -> i64 {
        let mut id = self.next_dlq_id.write();
        let entry_id = *id;
        *id += 1;
        entry_id
    }

    fn dispatch_to_subscribers(&self, event: &DomainEvent) -> Result<(), String> {
        let subscribers = self.subscribers.read();
        let event_type = event.event_type();

        for subscriber in subscribers.iter() {
            let interested = subscriber.subscribed_to();
            if interested.is_empty() || interested.iter().any(|t| t == event_type || t == "*") {
                // Fire-and-forget: spawn task for each subscriber
                // In production this would use tokio::spawn, but for in-memory
                // we call directly to keep things synchronous for testing
                if let Err(e) = futures::executor::block_on(subscriber.handle(event)) {
                    tracing::warn!(
                        "Subscriber {} failed for event {}: {}",
                        subscriber.name(),
                        event_type,
                        e
                    );
                }
            }
        }
        Ok(())
    }
}

impl Default for InMemoryEventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl EventBus for InMemoryEventBus {
    async fn publish(&self, event: DomainEvent) -> Result<i64, String> {
        let id = self.allocate_id();
        let tenant_id = event.tenant_id();
        let event_type = event.event_type().to_string();

        let outbox_event = OutboxEvent {
            id,
            event: event.clone(),
            aggregate_type: event_type.clone(),
            aggregate_id: id,
            tenant_id,
            created_at: Utc::now(),
            published_at: None,
            status: EventStatus::Pending,
            attempts: 0,
            last_error: None,
        };

        self.outbox.write().push(outbox_event);

        // Immediately dispatch to subscribers (in-memory only)
        self.dispatch_to_subscribers(&event)?;

        // Mark as published
        let mut outbox = self.outbox.write();
        if let Some(e) = outbox.iter_mut().find(|e| e.id == id) {
            e.status = EventStatus::Published;
            e.published_at = Some(Utc::now());
            e.attempts = 1;
        }

        Ok(id)
    }

    async fn publish_batch(&self, events: Vec<DomainEvent>) -> Result<Vec<i64>, String> {
        let mut ids = Vec::new();
        for event in events {
            let id = self.publish(event).await?;
            ids.push(id);
        }
        Ok(ids)
    }

    async fn subscribe(&self, subscriber: Arc<dyn EventSubscriber>) -> Result<(), String> {
        self.subscribers.write().push(subscriber);
        Ok(())
    }

    async fn process_outbox(&self, batch_size: u32) -> Result<u64, String> {
        let pending: Vec<OutboxEvent> = {
            let outbox = self.outbox.read();
            outbox
                .iter()
                .filter(|e| e.status == EventStatus::Pending)
                .take(batch_size as usize)
                .cloned()
                .collect()
        };

        let mut processed = 0u64;
        for event in pending {
            // Dispatch to subscribers
            self.dispatch_to_subscribers(&event.event)?;

            // Mark as published
            let mut outbox = self.outbox.write();
            if let Some(e) = outbox.iter_mut().find(|o| o.id == event.id) {
                e.status = EventStatus::Published;
                e.published_at = Some(Utc::now());
                e.attempts += 1;
            }
            processed += 1;
        }

        Ok(processed)
    }

    async fn get_pending(&self, limit: u32) -> Result<Vec<OutboxEvent>, String> {
        let outbox = self.outbox.read();
        Ok(outbox
            .iter()
            .filter(|e| e.status == EventStatus::Pending)
            .take(limit as usize)
            .cloned()
            .collect())
    }

    async fn mark_published(&self, event_id: i64) -> Result<(), String> {
        let mut outbox = self.outbox.write();
        let event = outbox
            .iter_mut()
            .find(|e| e.id == event_id)
            .ok_or_else(|| format!("Outbox event {} not found", event_id))?;
        event.status = EventStatus::Published;
        event.published_at = Some(Utc::now());
        Ok(())
    }

    async fn mark_failed(&self, event_id: i64, error: &str) -> Result<(), String> {
        let mut outbox = self.outbox.write();
        let event = outbox
            .iter_mut()
            .find(|e| e.id == event_id)
            .ok_or_else(|| format!("Outbox event {} not found", event_id))?;

        event.attempts += 1;
        event.last_error = Some(error.to_string());

        if event.attempts >= self.max_attempts {
            // Move to DLQ
            let dlq_id = self.allocate_dlq_id();
            let dlq_entry = DeadLetterEntry {
                id: dlq_id,
                original_event: event.event.clone(),
                aggregate_type: event.aggregate_type.clone(),
                aggregate_id: event.aggregate_id,
                tenant_id: event.tenant_id,
                error: error.to_string(),
                original_attempts: event.attempts,
                dead_lettered_at: Utc::now(),
            };
            self.dead_letters.write().push(dlq_entry);
            event.status = EventStatus::DeadLettered;
        } else {
            event.status = EventStatus::Failed;
        }

        Ok(())
    }

    async fn get_dead_letters(&self, tenant_id: i64) -> Result<Vec<DeadLetterEntry>, String> {
        let dlq = self.dead_letters.read();
        Ok(dlq
            .iter()
            .filter(|e| e.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn retry_dead_letter(&self, entry_id: i64) -> Result<(), String> {
        let entry = {
            let dlq = self.dead_letters.read();
            dlq.iter()
                .find(|e| e.id == entry_id)
                .cloned()
                .ok_or_else(|| format!("DLQ entry {} not found", entry_id))?
        };

        // Re-publish the event
        self.publish(entry.original_event.clone()).await?;

        // Remove from DLQ
        self.dead_letters.write().retain(|e| e.id != entry_id);

        Ok(())
    }
}

/// Stock decrement subscriber (InvoiceCreated → StockDecrement)
pub struct StockDecrementSubscriber;

#[async_trait::async_trait]
impl EventSubscriber for StockDecrementSubscriber {
    async fn handle(&self, event: &DomainEvent) -> Result<(), String> {
        match event {
            DomainEvent::InvoiceCreated { invoice_id, .. } => {
                tracing::info!(
                    "StockDecrementSubscriber: processing invoice {}",
                    invoice_id
                );
                // In production, this would call StockService to decrement stock
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn subscribed_to(&self) -> Vec<String> {
        vec!["invoice_created".to_string()]
    }

    fn name(&self) -> &str {
        "StockDecrementSubscriber"
    }
}

/// Accounting entry subscriber (InvoiceCreated → AccountingEntry)
pub struct AccountingEntrySubscriber;

#[async_trait::async_trait]
impl EventSubscriber for AccountingEntrySubscriber {
    async fn handle(&self, event: &DomainEvent) -> Result<(), String> {
        match event {
            DomainEvent::InvoiceCreated {
                invoice_id, amount, ..
            } => {
                tracing::info!(
                    "AccountingEntrySubscriber: creating entry for invoice {} amount {}",
                    invoice_id,
                    amount
                );
                // In production, this would call AccountingService to create journal entry
                Ok(())
            }
            DomainEvent::PaymentReceived {
                payment_id, amount, ..
            } => {
                tracing::info!(
                    "AccountingEntrySubscriber: recording payment {} amount {}",
                    payment_id,
                    amount
                );
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn subscribed_to(&self) -> Vec<String> {
        vec![
            "invoice_created".to_string(),
            "payment_received".to_string(),
        ]
    }

    fn name(&self) -> &str {
        "AccountingEntrySubscriber"
    }
}

/// Subscriber that creates e-Defter period entries when
/// accounting journal entries trigger e-Defter integration.
pub struct EDefterAccountingSubscriber;

#[async_trait::async_trait]
impl EventSubscriber for EDefterAccountingSubscriber {
    async fn handle(&self, event: &DomainEvent) -> Result<(), String> {
        match event {
            DomainEvent::InvoiceCreated {
                invoice_id, amount, ..
            } => {
                tracing::info!(
                    "EDefterAccountingSubscriber: invoice {} amount {} — will populate e-Defter",
                    invoice_id,
                    amount
                );
                // In production, this would call EDefterService::populate_from_accounting
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn subscribed_to(&self) -> Vec<String> {
        vec!["invoice_created".to_string()]
    }

    fn name(&self) -> &str {
        "EDefterAccountingSubscriber"
    }
}

/// Subscriber that creates an e-Fatura draft when an invoice is created
/// and fires EFaturaCreated when e-Fatura is sent to GIB.
pub struct EFaturaIntegrationSubscriber;

#[async_trait::async_trait]
impl EventSubscriber for EFaturaIntegrationSubscriber {
    async fn handle(&self, event: &DomainEvent) -> Result<(), String> {
        match event {
            DomainEvent::InvoiceCreated {
                invoice_id, amount, ..
            } => {
                tracing::info!(
                    "EFaturaIntegrationSubscriber: invoice {} amount {} — eligible for e-Fatura creation",
                    invoice_id,
                    amount
                );
                // In production, this would call EFaturaService::create_from_invoice
                Ok(())
            }
            DomainEvent::EFaturaCreated {
                fatura_id, uuid, ..
            } => {
                tracing::info!(
                    "EFaturaIntegrationSubscriber: e-Fatura {} created with UUID {}",
                    fatura_id,
                    uuid
                );
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn subscribed_to(&self) -> Vec<String> {
        vec!["invoice_created".to_string(), "efatura_created".to_string()]
    }

    fn name(&self) -> &str {
        "EFaturaIntegrationSubscriber"
    }
}

/// Subscriber for tax period lifecycle events.
/// Fires TaxPeriodCalculated after calculation, TaxPeriodFiled after filing.
pub struct TaxPeriodSubscriber;

#[async_trait::async_trait]
impl EventSubscriber for TaxPeriodSubscriber {
    async fn handle(&self, event: &DomainEvent) -> Result<(), String> {
        match event {
            DomainEvent::TaxPeriodCalculated {
                period_id,
                tax_type,
                ..
            } => {
                tracing::info!(
                    "TaxPeriodSubscriber: tax period {} ({}) calculated",
                    period_id,
                    tax_type
                );
                Ok(())
            }
            DomainEvent::TaxPeriodFiled {
                period_id,
                tax_type,
                ..
            } => {
                tracing::info!(
                    "TaxPeriodSubscriber: tax period {} ({}) filed",
                    period_id,
                    tax_type
                );
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn subscribed_to(&self) -> Vec<String> {
        vec![
            "tax_period_calculated".to_string(),
            "tax_period_filed".to_string(),
        ]
    }

    fn name(&self) -> &str {
        "TaxPeriodSubscriber"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_publish_event() {
        let bus = InMemoryEventBus::new();
        let id = bus
            .publish(DomainEvent::InvoiceCreated {
                invoice_id: 1,
                tenant_id: 1,
                amount: "1000.00".to_string(),
                currency: "TRY".to_string(),
            })
            .await
            .unwrap();

        assert!(id > 0);

        // Should be marked as published immediately in in-memory mode
        let outbox = bus.outbox.read();
        assert_eq!(outbox.len(), 1);
        assert_eq!(outbox[0].status, EventStatus::Published);
    }

    #[tokio::test]
    async fn test_publish_batch() {
        let bus = InMemoryEventBus::new();
        let events = vec![
            DomainEvent::InvoiceCreated {
                invoice_id: 1,
                tenant_id: 1,
                amount: "1000.00".to_string(),
                currency: "TRY".to_string(),
            },
            DomainEvent::PaymentReceived {
                payment_id: 1,
                invoice_id: 1,
                tenant_id: 1,
                amount: "500.00".to_string(),
            },
        ];

        let ids = bus.publish_batch(events).await.unwrap();
        assert_eq!(ids.len(), 2);
    }

    #[tokio::test]
    async fn test_subscriber_dispatch() {
        let bus = InMemoryEventBus::new();
        bus.subscribe(Arc::new(StockDecrementSubscriber))
            .await
            .unwrap();
        bus.subscribe(Arc::new(AccountingEntrySubscriber))
            .await
            .unwrap();

        // Publish event — subscribers should receive it
        bus.publish(DomainEvent::InvoiceCreated {
            invoice_id: 1,
            tenant_id: 1,
            amount: "1000.00".to_string(),
            currency: "TRY".to_string(),
        })
        .await
        .unwrap();

        // Event should be published with subscriber processing
        let outbox = bus.outbox.read();
        assert_eq!(outbox[0].status, EventStatus::Published);
    }

    #[tokio::test]
    async fn test_mark_failed_and_dlq() {
        let bus = InMemoryEventBus::with_max_attempts(2);

        // Manually add a pending event
        let id = bus.allocate_id();
        {
            let mut outbox = bus.outbox.write();
            outbox.push(OutboxEvent {
                id,
                event: DomainEvent::Custom {
                    name: "test".to_string(),
                    tenant_id: 1,
                    payload: "{}".to_string(),
                },
                aggregate_type: "test".to_string(),
                aggregate_id: id,
                tenant_id: 1,
                created_at: Utc::now(),
                published_at: None,
                status: EventStatus::Pending,
                attempts: 0,
                last_error: None,
            });
        }

        // First failure
        bus.mark_failed(id, "Connection timeout").await.unwrap();
        {
            let outbox = bus.outbox.read();
            assert_eq!(outbox[0].status, EventStatus::Failed);
            assert_eq!(outbox[0].attempts, 1);
        }

        // Second failure — should go to DLQ
        bus.mark_failed(id, "Connection timeout").await.unwrap();
        {
            let outbox = bus.outbox.read();
            assert_eq!(outbox[0].status, EventStatus::DeadLettered);
        }

        let dlq = bus.get_dead_letters(1).await.unwrap();
        assert_eq!(dlq.len(), 1);
        assert_eq!(dlq[0].error, "Connection timeout");
    }

    #[tokio::test]
    async fn test_retry_dead_letter() {
        let bus = InMemoryEventBus::with_max_attempts(1);

        // Add a pending event
        let id = bus.allocate_id();
        {
            let mut outbox = bus.outbox.write();
            outbox.push(OutboxEvent {
                id,
                event: DomainEvent::Custom {
                    name: "retry_test".to_string(),
                    tenant_id: 1,
                    payload: "{}".to_string(),
                },
                aggregate_type: "test".to_string(),
                aggregate_id: id,
                tenant_id: 1,
                created_at: Utc::now(),
                published_at: None,
                status: EventStatus::Pending,
                attempts: 0,
                last_error: None,
            });
        }

        // Fail it to DLQ
        bus.mark_failed(id, "Fatal error").await.unwrap();

        let dlq = bus.get_dead_letters(1).await.unwrap();
        assert_eq!(dlq.len(), 1);
        let dlq_id = dlq[0].id;

        // Retry from DLQ
        bus.retry_dead_letter(dlq_id).await.unwrap();

        // DLQ should be empty, new event in outbox
        let dlq = bus.get_dead_letters(1).await.unwrap();
        assert!(dlq.is_empty());
    }

    #[tokio::test]
    async fn test_event_tenant_id() {
        let event = DomainEvent::InvoiceCreated {
            invoice_id: 1,
            tenant_id: 42,
            amount: "100".to_string(),
            currency: "TRY".to_string(),
        };
        assert_eq!(event.tenant_id(), 42);
        assert_eq!(event.event_type(), "invoice_created");
    }

    #[tokio::test]
    async fn test_process_outbox() {
        let bus = InMemoryEventBus::new();

        // Manually add pending events
        for i in 0..5 {
            let id = bus.allocate_id();
            let mut outbox = bus.outbox.write();
            outbox.push(OutboxEvent {
                id,
                event: DomainEvent::Custom {
                    name: format!("test_{}", i),
                    tenant_id: 1,
                    payload: "{}".to_string(),
                },
                aggregate_type: "test".to_string(),
                aggregate_id: id,
                tenant_id: 1,
                created_at: Utc::now(),
                published_at: None,
                status: EventStatus::Pending,
                attempts: 0,
                last_error: None,
            });
        }

        let processed = bus.process_outbox(3).await.unwrap();
        assert_eq!(processed, 3);

        // 2 remaining
        let pending = bus.get_pending(10).await.unwrap();
        assert_eq!(pending.len(), 2);
    }

    #[tokio::test]
    async fn test_efatura_events() {
        let bus = InMemoryEventBus::new();
        bus.subscribe(Arc::new(EFaturaIntegrationSubscriber))
            .await
            .unwrap();

        // Publish EFaturaCreated event
        let id = bus
            .publish(DomainEvent::EFaturaCreated {
                tenant_id: 1,
                fatura_id: 42,
                uuid: "uuid-123".to_string(),
            })
            .await
            .unwrap();
        assert!(id > 0);

        // Publish EFaturaSent event
        let id2 = bus
            .publish(DomainEvent::EFaturaSent {
                tenant_id: 1,
                fatura_id: 42,
                uuid: "uuid-123".to_string(),
            })
            .await
            .unwrap();
        assert!(id2 > 0);

        // Publish EFaturaCancelled event
        let id3 = bus
            .publish(DomainEvent::EFaturaCancelled {
                tenant_id: 1,
                fatura_id: 42,
                reason: "Mistake".to_string(),
            })
            .await
            .unwrap();
        assert!(id3 > 0);

        // All should be published
        let outbox = bus.outbox.read();
        assert_eq!(outbox.len(), 3);
        assert_eq!(outbox[0].status, EventStatus::Published);
    }

    #[tokio::test]
    async fn test_edefter_events() {
        let bus = InMemoryEventBus::new();

        // Publish EDefterPeriodCreated
        let id = bus
            .publish(DomainEvent::EDefterPeriodCreated {
                tenant_id: 1,
                period_id: 10,
                year: 2024,
                month: 6,
            })
            .await
            .unwrap();
        assert!(id > 0);

        // Publish EDefterPeriodSigned
        let id2 = bus
            .publish(DomainEvent::EDefterPeriodSigned {
                tenant_id: 1,
                period_id: 10,
            })
            .await
            .unwrap();
        assert!(id2 > 0);

        // Publish EDefterPeriodSent
        let id3 = bus
            .publish(DomainEvent::EDefterPeriodSent {
                tenant_id: 1,
                period_id: 10,
            })
            .await
            .unwrap();
        assert!(id3 > 0);

        let outbox = bus.outbox.read();
        assert_eq!(outbox.len(), 3);
    }

    #[tokio::test]
    async fn test_tax_events() {
        let bus = InMemoryEventBus::new();
        bus.subscribe(Arc::new(TaxPeriodSubscriber)).await.unwrap();

        // Publish TaxPeriodCalculated
        let id = bus
            .publish(DomainEvent::TaxPeriodCalculated {
                tenant_id: 1,
                period_id: 5,
                tax_type: "KDV".to_string(),
            })
            .await
            .unwrap();
        assert!(id > 0);

        // Publish TaxPeriodFiled
        let id2 = bus
            .publish(DomainEvent::TaxPeriodFiled {
                tenant_id: 1,
                period_id: 5,
                tax_type: "KDV".to_string(),
            })
            .await
            .unwrap();
        assert!(id2 > 0);

        let outbox = bus.outbox.read();
        assert_eq!(outbox.len(), 2);
    }

    #[tokio::test]
    async fn test_new_event_types_and_tenant_ids() {
        // Verify tenant_id() and event_type() for all new event variants
        let efatura_created = DomainEvent::EFaturaCreated {
            tenant_id: 10,
            fatura_id: 1,
            uuid: "abc".to_string(),
        };
        assert_eq!(efatura_created.tenant_id(), 10);
        assert_eq!(efatura_created.event_type(), "efatura_created");

        let efatura_sent = DomainEvent::EFaturaSent {
            tenant_id: 20,
            fatura_id: 2,
            uuid: "def".to_string(),
        };
        assert_eq!(efatura_sent.tenant_id(), 20);
        assert_eq!(efatura_sent.event_type(), "efatura_sent");

        let efatura_cancelled = DomainEvent::EFaturaCancelled {
            tenant_id: 30,
            fatura_id: 3,
            reason: "error".to_string(),
        };
        assert_eq!(efatura_cancelled.tenant_id(), 30);
        assert_eq!(efatura_cancelled.event_type(), "efatura_cancelled");

        let edefter_created = DomainEvent::EDefterPeriodCreated {
            tenant_id: 40,
            period_id: 1,
            year: 2024,
            month: 6,
        };
        assert_eq!(edefter_created.tenant_id(), 40);
        assert_eq!(edefter_created.event_type(), "edefter_period_created");

        let edefter_signed = DomainEvent::EDefterPeriodSigned {
            tenant_id: 50,
            period_id: 2,
        };
        assert_eq!(edefter_signed.tenant_id(), 50);
        assert_eq!(edefter_signed.event_type(), "edefter_period_signed");

        let edefter_sent = DomainEvent::EDefterPeriodSent {
            tenant_id: 60,
            period_id: 3,
        };
        assert_eq!(edefter_sent.tenant_id(), 60);
        assert_eq!(edefter_sent.event_type(), "edefter_period_sent");

        let tax_calculated = DomainEvent::TaxPeriodCalculated {
            tenant_id: 70,
            period_id: 4,
            tax_type: "KDV".to_string(),
        };
        assert_eq!(tax_calculated.tenant_id(), 70);
        assert_eq!(tax_calculated.event_type(), "tax_period_calculated");

        let tax_filed = DomainEvent::TaxPeriodFiled {
            tenant_id: 80,
            period_id: 5,
            tax_type: "BSMV".to_string(),
        };
        assert_eq!(tax_filed.tenant_id(), 80);
        assert_eq!(tax_filed.event_type(), "tax_period_filed");
    }

    #[tokio::test]
    async fn test_edefter_accounting_subscriber() {
        let bus = InMemoryEventBus::new();
        bus.subscribe(Arc::new(EDefterAccountingSubscriber))
            .await
            .unwrap();

        // InvoiceCreated should be received by EDefterAccountingSubscriber
        let id = bus
            .publish(DomainEvent::InvoiceCreated {
                invoice_id: 1,
                tenant_id: 1,
                amount: "5000.00".to_string(),
                currency: "TRY".to_string(),
            })
            .await
            .unwrap();
        assert!(id > 0);

        let outbox = bus.outbox.read();
        assert_eq!(outbox.len(), 1);
        assert_eq!(outbox[0].status, EventStatus::Published);
    }
}
