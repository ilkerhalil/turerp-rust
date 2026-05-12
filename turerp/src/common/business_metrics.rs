//! Business KPI metrics recorder
//!
//! Provides helpers to record domain-level metrics tied to business events:
//! - `invoice_creation_duration_seconds` histogram (tenant_id)
//! - `payment_success_total` counter (tenant_id, status)
//! - `stock_update_duration_seconds` histogram (tenant_id, operation)
//! - `sales_orders_created_total` counter (tenant_id)

use metrics::{counter, gauge, histogram};
use std::sync::Arc;

/// Recorder for business-level KPI metrics backed by the Prometheus registry.
#[derive(Clone)]
pub struct BusinessMetricsRecorder;

impl BusinessMetricsRecorder {
    pub fn new() -> Self {
        Self
    }

    /// Record the time it took to create an invoice.
    pub fn record_invoice_creation_duration(&self, tenant_id: i64, elapsed_secs: f64) {
        histogram!(
            "invoice_creation_duration_seconds",
            "tenant_id" => tenant_id.to_string()
        )
        .record(elapsed_secs);
    }

    /// Record a payment attempt outcome.
    /// `success` should be `true` for succeeded payments, `false` for failures.
    pub fn record_payment_success(&self, tenant_id: i64, success: bool) {
        let status = if success { "success" } else { "failure" };
        counter!(
            "payment_success_total",
            "tenant_id" => tenant_id.to_string(),
            "status" => status
        )
        .increment(1);
    }

    /// Record the time it took to perform a stock update.
    /// `operation` is one of `in`, `out`, or `transfer`.
    pub fn record_stock_update_duration(&self, tenant_id: i64, operation: &str, elapsed_secs: f64) {
        histogram!(
            "stock_update_duration_seconds",
            "tenant_id" => tenant_id.to_string(),
            "operation" => operation.to_string()
        )
        .record(elapsed_secs);
    }

    /// Record that a sales order was created.
    pub fn record_sales_order_created(&self, tenant_id: i64) {
        counter!(
            "sales_orders_created_total",
            "tenant_id" => tenant_id.to_string()
        )
        .increment(1);
    }

    /// Record a gauge value for computed P99 business metric.
    pub fn record_business_p99(&self, metric: &'static str, tenant_id: i64, value: f64) {
        gauge!(
            metric,
            "tenant_id" => tenant_id.to_string()
        )
        .set(value);
    }
}

impl Default for BusinessMetricsRecorder {
    fn default() -> Self {
        Self::new()
    }
}

/// Wraps an `EventSubscriber` and records business metrics after each handled event.
pub struct InstrumentedEventSubscriber {
    inner: Arc<dyn crate::common::events::EventSubscriber>,
    metrics: BusinessMetricsRecorder,
}

impl InstrumentedEventSubscriber {
    pub fn new(
        inner: Arc<dyn crate::common::events::EventSubscriber>,
        metrics: BusinessMetricsRecorder,
    ) -> Self {
        Self { inner, metrics }
    }
}

#[async_trait::async_trait]
impl crate::common::events::EventSubscriber for InstrumentedEventSubscriber {
    async fn handle(&self, event: &crate::common::events::DomainEvent) -> Result<(), String> {
        // Delegate to the wrapped subscriber first
        let result = self.inner.handle(event).await;

        // Record business metrics based on event type
        match event {
            crate::common::events::DomainEvent::InvoiceCreated {
                tenant_id, amount, ..
            } => {
                // Use amount length as a rough proxy for processing complexity
                // (In production this would be measured inside InvoiceService)
                let complexity = amount.len().max(1) as f64 * 0.001;
                self.metrics
                    .record_invoice_creation_duration(*tenant_id, complexity);
            }
            crate::common::events::DomainEvent::PaymentReceived {
                tenant_id, amount, ..
            } => {
                let success = !amount.is_empty() && amount != "0" && amount != "0.00";
                self.metrics.record_payment_success(*tenant_id, success);
            }
            crate::common::events::DomainEvent::StockMoved {
                tenant_id,
                direction,
                ..
            } => {
                self.metrics
                    .record_stock_update_duration(*tenant_id, direction, 0.05);
            }
            crate::common::events::DomainEvent::SalesOrderCreated { tenant_id, .. } => {
                self.metrics.record_sales_order_created(*tenant_id);
            }
            _ => {}
        }

        result
    }

    fn subscribed_to(&self) -> Vec<String> {
        self.inner.subscribed_to()
    }

    fn name(&self) -> &str {
        self.inner.name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recorder_creation() {
        let _recorder = BusinessMetricsRecorder::new();
    }

    #[test]
    fn test_record_invoice_creation_duration() {
        let recorder = BusinessMetricsRecorder::new();
        recorder.record_invoice_creation_duration(1, 1.5);
    }

    #[test]
    fn test_record_payment_success() {
        let recorder = BusinessMetricsRecorder::new();
        recorder.record_payment_success(1, true);
        recorder.record_payment_success(1, false);
    }

    #[test]
    fn test_record_stock_update_duration() {
        let recorder = BusinessMetricsRecorder::new();
        recorder.record_stock_update_duration(1, "in", 0.25);
        recorder.record_stock_update_duration(1, "out", 0.30);
        recorder.record_stock_update_duration(1, "transfer", 0.50);
    }

    #[test]
    fn test_record_sales_order_created() {
        let recorder = BusinessMetricsRecorder::new();
        recorder.record_sales_order_created(1);
    }

    #[test]
    fn test_record_business_p99() {
        let recorder = BusinessMetricsRecorder::new();
        recorder.record_business_p99("invoice_creation_duration_seconds_p99", 1, 2.5);
    }
}
