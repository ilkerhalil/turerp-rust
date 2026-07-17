//! Prometheus metrics middleware
//!
//! Records HTTP request metrics:
//! - `http_requests_total` counter (labels: method, endpoint, status)
//! - `http_request_duration_seconds` histogram (labels: method, endpoint)
//! - `http_requests_in_flight` gauge (labels: method)
//!
//! Endpoint labels are normalized to avoid cardinality explosion:
//! numeric IDs in paths are replaced with `:id`.

use actix_web::body::MessageBody;
use actix_web::{dev::ServiceRequest, dev::ServiceResponse, Error};
use metrics::{counter, gauge, histogram};
use std::task::{Context, Poll};
use std::time::Instant;

use metrics_exporter_prometheus::PrometheusHandle;

/// Global handle for rendering Prometheus metrics.
///
/// Uses `Mutex` instead of `OnceLock` so tests can reset the handle
/// between runs, eliminating test-order dependency.
static PROMETHEUS_HANDLE: parking_lot::Mutex<Option<PrometheusHandle>> =
    parking_lot::Mutex::new(None);

/// Metrics middleware
pub struct MetricsMiddleware;

impl MetricsMiddleware {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MetricsMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

/// Normalize an HTTP path into a low-cardinality endpoint label.
///
/// Replaces trailing numeric segments with `:id` to prevent
/// unbounded cardinality from IDs in URLs like `/api/v1/invoices/12345`.
pub fn normalize_endpoint(path: &str) -> String {
    let mut parts: Vec<&str> = path.split('/').collect();
    for part in parts.iter_mut() {
        if part.parse::<u64>().is_ok() || part.parse::<i64>().is_ok() {
            *part = ":id";
        }
    }
    parts.join("/")
}

impl<S, B> actix_web::dev::Transform<S, ServiceRequest> for MetricsMiddleware
where
    S: actix_web::dev::Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = MetricsMiddlewareService<S>;
    type Future = std::future::Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std::future::ready(Ok(MetricsMiddlewareService { service }))
    }
}

/// The actual middleware service that records metrics
pub struct MetricsMiddlewareService<S> {
    service: S,
}

impl<S, B> actix_web::dev::Service<ServiceRequest> for MetricsMiddlewareService<S>
where
    S: actix_web::dev::Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = futures::future::LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&self, ctx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(ctx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let method = req.method().to_string();
        let endpoint = normalize_endpoint(req.path());

        let fut = self.service.call(req);

        Box::pin(async move {
            // Increment in-flight gauge inside the async block so that
            // dropped futures (e.g. IpWhitelistMiddleware dropping the
            // service future on the blocked path without polling it)
            // do not leak the gauge. The gauge is only incremented when
            // the future is actually polled, and decremented when it
            // completes — guaranteeing symmetric increment/decrement.
            gauge!("http_requests_in_flight", "method" => method.clone()).increment(1);

            // Record start time (inside async block — measures actual
            // request processing, not middleware chain construction)
            let start = Instant::now();

            let result = fut.await;

            // Decrement in-flight gauge
            gauge!("http_requests_in_flight", "method" => method.clone()).decrement(1);

            let elapsed = start.elapsed().as_secs_f64();

            // Record metrics based on response status
            let status = match &result {
                Ok(res) => res.status().as_u16(),
                Err(_) => 500,
            };

            counter!(
                "http_requests_total",
                "method" => method.clone(),
                "endpoint" => endpoint.clone(),
                "status" => status.to_string()
            )
            .increment(1);
            histogram!(
                "http_request_duration_seconds",
                "method" => method.clone(),
                "endpoint" => endpoint.clone()
            )
            .record(elapsed);

            result
        })
    }
}

/// Install the Prometheus metrics recorder globally and store the handle.
///
/// Uses `install_recorder()` which returns a `PrometheusHandle` that can
/// later be used to render metrics via `render_metrics()`.
pub fn install_metrics_exporter() -> Result<(), String> {
    use metrics_exporter_prometheus::PrometheusBuilder;

    let builder = PrometheusBuilder::new();
    let handle = builder
        .install_recorder()
        .map_err(|e| format!("Failed to install Prometheus recorder: {}", e))?;

    let mut guard = PROMETHEUS_HANDLE.lock();
    if guard.is_some() {
        return Err("Prometheus handle already initialized".to_string());
    }
    *guard = Some(handle);

    tracing::info!("Prometheus metrics exporter installed");
    Ok(())
}

/// Get the current Prometheus metrics in text format.
///
/// Uses the globally stored `PrometheusHandle` to render the current
/// metrics snapshot. Returns empty string if the recorder was never installed.
pub fn render_metrics() -> String {
    PROMETHEUS_HANDLE
        .lock()
        .as_ref()
        .map(|h| h.render())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_middleware_creation() {
        let _middleware = MetricsMiddleware::new();
    }

    #[test]
    fn test_render_metrics_without_install() {
        // Clear any previously installed handle so this test is independent.
        *PROMETHEUS_HANDLE.lock() = None;
        assert_eq!(render_metrics(), "");
    }

    #[test]
    fn test_render_metrics_after_install() {
        // Ensure a clean slate, then install, record a metric, and verify output.
        *PROMETHEUS_HANDLE.lock() = None;
        install_metrics_exporter().expect("install should succeed in test");
        metrics::counter!("test_metric_total").increment(1);
        let rendered = render_metrics();
        assert!(rendered.contains("test_metric_total"));
        // Clean up so other tests are not affected.
        *PROMETHEUS_HANDLE.lock() = None;
    }
}
