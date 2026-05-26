//! Background evaluator for SLI/SLO and alert rule auto-evaluation
//!
//! Spawns a tokio task on startup that polls every 60 seconds:
//! 1. Reads current Prometheus metric values from the recorder handle.
//! 2. Computes P95/P99 from histogram buckets via linear interpolation.
//! 3. Evaluates SLO compliance using `ObservabilityService`.
//! 4. Evaluates alert rules with current metric snapshots.
//! 5. Fires in-app notifications for triggered alerts.
//! 6. Records SLI measurements into the repository.

use crate::common::notifications::{NotificationPriority, NotificationRequest};
use crate::common::prometheus_percentile::compute_percentiles;
use crate::common::NotificationService;
use crate::domain::observability::model::SloStatus;
use crate::domain::observability::service::ObservabilityService;
use crate::error::ApiError;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{interval, Duration};

/// Background task that continuously evaluates observability rules.
#[derive(Clone)]
pub struct BackgroundEvaluator {
    observability_service: ObservabilityService,
    notification_service: Arc<dyn NotificationService>,
    interval_secs: u64,
}

impl BackgroundEvaluator {
    /// Create a new background evaluator.
    pub fn new(
        observability_service: ObservabilityService,
        notification_service: Arc<dyn NotificationService>,
    ) -> Self {
        Self {
            observability_service,
            notification_service,
            interval_secs: 60,
        }
    }

    /// Start the background evaluation loop.
    ///
    /// The loop runs until the provided `shutdown_rx` fires.
    pub fn start(
        &self,
        mut shutdown_rx: tokio::sync::mpsc::Receiver<()>,
    ) -> tokio::task::JoinHandle<()> {
        let service = self.observability_service.clone();
        let notifier = self.notification_service.clone();
        let interval_secs = self.interval_secs;

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(interval_secs));

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        if let Err(e) = Self::evaluate_tick(
                            &service,
                            &*notifier,
                        ).await {
                            tracing::error!("Background evaluator tick failed: {}", e);
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        tracing::info!("Background evaluator shutting down");
                        break;
                    }
                }
            }
        })
    }

    /// Single evaluation tick: SLOs + alerts + SLI recording.
    async fn evaluate_tick(
        service: &ObservabilityService,
        notifier: &dyn NotificationService,
    ) -> Result<(), ApiError> {
        // 1. Render Prometheus metrics and compute percentiles
        let metrics_text = crate::middleware::metrics::render_metrics();
        let percentiles = compute_percentiles(&metrics_text);
        let mut metrics = Self::read_prometheus_metrics().await;

        // Merge computed percentiles into the metrics map
        for (key, value) in &percentiles {
            metrics.insert(key.clone(), *value);
        }

        // Aggregate P95/P99 across all http_request_duration_seconds histograms
        let mut p95_values = Vec::new();
        let mut p99_values = Vec::new();
        for (key, value) in &percentiles {
            if key.starts_with("http_request_duration_seconds") {
                if key.contains("quantile=p95") {
                    p95_values.push(*value);
                } else if key.contains("quantile=p99") {
                    p99_values.push(*value);
                }
            }
        }
        if !p95_values.is_empty() {
            let avg_p95 = p95_values.iter().sum::<f64>() / p95_values.len() as f64;
            metrics.insert("latency_p95".to_string(), avg_p95);
        }
        if !p99_values.is_empty() {
            let avg_p99 = p99_values.iter().sum::<f64>() / p99_values.len() as f64;
            metrics.insert("latency_p99".to_string(), avg_p99);
        }

        // 2. Evaluate SLO compliance
        let slos = service.evaluate_slo_compliance().await?;
        for slo in &slos {
            if let SloStatus::Breached = slo.status {
                let request = NotificationRequest {
                    tenant_id: 0,
                    user_id: None,
                    channel: crate::common::notifications::NotificationChannel::InApp,
                    priority: NotificationPriority::Urgent,
                    template_key: "slo_breached".to_string(),
                    template_vars: serde_json::json!({
                        "slo_id": slo.slo_id,
                        "slo_name": slo.slo_name,
                        "current_value": slo.current_value,
                        "target_value": slo.target_value,
                    }),
                    recipient: "admin".to_string(),
                };
                notifier.send(request).await.ok();
            }
        }

        // 3. Evaluate alert rules
        let alerts = service.evaluate_alert_rules(&metrics).await?;
        for alert in &alerts {
            let priority = match alert.severity {
                crate::domain::observability::model::AlertSeverity::Critical => {
                    NotificationPriority::Urgent
                }
                crate::domain::observability::model::AlertSeverity::Warning => {
                    NotificationPriority::High
                }
                crate::domain::observability::model::AlertSeverity::Info => {
                    NotificationPriority::Normal
                }
            };
            let request = NotificationRequest {
                tenant_id: 0,
                user_id: None,
                channel: crate::common::notifications::NotificationChannel::InApp,
                priority,
                template_key: "alert_fired".to_string(),
                template_vars: serde_json::json!({
                    "alert_id": alert.id,
                    "rule_name": alert.rule_name,
                    "severity": alert.severity.to_string(),
                    "value": alert.value,
                    "message": alert.message,
                }),
                recipient: "admin".to_string(),
            };
            notifier.send(request).await.ok();
        }

        // 4. Record SLI measurements from metric snapshots + percentiles
        for (name, value) in &metrics {
            if name.contains("_p95") || name.contains("_p99") || name.contains("_duration_seconds")
            {
                let sli_id = format!("auto-sli-{}", name.replace('.', "_"));
                if let Err(e) = service.record_sli(sli_id, *value).await {
                    tracing::warn!("Failed to record SLI for {}: {}", name, e);
                }
            }
        }

        Ok(())
    }

    /// Read flat gauges/counters from Prometheus text output.
    async fn read_prometheus_metrics() -> HashMap<String, f64> {
        let rendered = crate::middleware::metrics::render_metrics();
        let mut metrics = HashMap::new();
        let mut total_requests = 0u64;
        let mut error_requests = 0u64;

        for line in rendered.lines() {
            if line.starts_with('#') || line.is_empty() {
                continue;
            }

            if let Some(val_start) = line.rfind(' ') {
                let name_and_labels = &line[..val_start];
                let value_str = &line[val_start + 1..];

                if let Ok(value) = value_str.parse::<f64>() {
                    let name = if let Some(brace) = name_and_labels.find('{') {
                        &name_and_labels[..brace]
                    } else {
                        name_and_labels
                    };

                    if name == "http_requests_total" {
                        total_requests += value as u64;
                        if line.contains("status=\"5") {
                            error_requests += value as u64;
                        }
                    } else if !name.ends_with("_bucket")
                        && !name.ends_with("_sum")
                        && !name.ends_with("_count")
                    {
                        // Simple gauge/counter - record directly
                        metrics.insert(name.to_string(), value);
                    }
                }
            }
        }

        // Derive availability, error_rate, throughput
        if total_requests > 0 {
            metrics.insert(
                "availability".to_string(),
                (total_requests.saturating_sub(error_requests)) as f64 / total_requests as f64,
            );
            metrics.insert(
                "error_rate".to_string(),
                error_requests as f64 / total_requests as f64,
            );
        }
        metrics.insert("throughput".to_string(), total_requests as f64);

        metrics
    }

    /// Extract label key/value pairs from a Prometheus metric line fragment.
    #[cfg(test)]
    fn extract_labels(fragment: &str) -> HashMap<String, String> {
        let mut labels = HashMap::new();
        if let Some(start) = fragment.find('{') {
            if let Some(end) = fragment.rfind('}') {
                let inner = &fragment[start + 1..end];
                for part in inner.split(',') {
                    if let Some(eq) = part.find('=') {
                        let key = part[..eq].trim().to_string();
                        let value = part[eq + 1..].trim().trim_matches('"').to_string();
                        labels.insert(key, value);
                    }
                }
            }
        }
        labels
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_labels() {
        let fragment = r#"metric{method="GET",endpoint="/api/v1/invoices/:id"}"#;
        let labels = BackgroundEvaluator::extract_labels(fragment);
        assert_eq!(labels.get("method"), Some(&"GET".to_string()));
        assert_eq!(
            labels.get("endpoint"),
            Some(&"/api/v1/invoices/:id".to_string())
        );
    }

    #[tokio::test]
    async fn test_read_prometheus_metrics_parses_lines() {
        // This function reads from the global metrics registry.
        // The exact contents depend on which metrics are registered,
        // so we only verify it does not panic and returns a map.
        let metrics = BackgroundEvaluator::read_prometheus_metrics().await;
        // Throughput is always inserted even when zero.
        assert!(metrics.contains_key("throughput"));
    }
}
