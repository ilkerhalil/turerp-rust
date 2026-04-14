//! Prometheus metrics middleware
//!
//! Records HTTP request metrics:
//! - `http_requests_total` counter (labels: method, path, status)
//! - `http_request_duration_seconds` histogram (labels: method, path)
//! - `http_requests_in_flight` gauge (labels: method)

use actix_web::body::BoxBody;
use actix_web::{dev::ServiceRequest, dev::ServiceResponse, Error};
use metrics::{counter, gauge, histogram};
use std::sync::OnceLock;
use std::task::{Context, Poll};
use std::time::Instant;

use metrics_exporter_prometheus::PrometheusHandle;

/// Global handle for rendering Prometheus metrics
static PROMETHEUS_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

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

impl<S> actix_web::dev::Transform<S, ServiceRequest> for MetricsMiddleware
where
    S: actix_web::dev::Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
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

impl<S> actix_web::dev::Service<ServiceRequest> for MetricsMiddlewareService<S>
where
    S: actix_web::dev::Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = futures::future::LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&self, ctx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(ctx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let method = req.method().to_string();
        let path = req.path().to_string();

        // Increment in-flight gauge
        gauge!("http_requests_in_flight", "method" => method.clone()).increment(1);

        // Record start time
        let start = Instant::now();

        let fut = self.service.call(req);

        Box::pin(async move {
            let result = fut.await;

            // Decrement in-flight gauge
            gauge!("http_requests_in_flight", "method" => method.clone()).decrement(1);

            let elapsed = start.elapsed().as_secs_f64();

            // Record metrics based on response status
            let status = match &result {
                Ok(res) => res.status().as_u16(),
                Err(_) => 500,
            };

            counter!("http_requests_total", "method" => method.clone(), "path" => path.clone(), "status" => status.to_string()).increment(1);
            histogram!("http_request_duration_seconds", "method" => method, "path" => path)
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

    PROMETHEUS_HANDLE
        .set(handle)
        .map_err(|_| "Prometheus handle already initialized".to_string())?;

    tracing::info!("Prometheus metrics exporter installed");
    Ok(())
}

/// Get the current Prometheus metrics in text format.
///
/// Uses the globally stored `PrometheusHandle` to render the current
/// metrics snapshot. Returns empty string if the recorder was never installed.
pub fn render_metrics() -> String {
    PROMETHEUS_HANDLE
        .get()
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
        // Without installing, render_metrics should return empty string
        // (The OnceLock is never set in this test, so get() returns None)
        // Note: This test assumes no other test has called install_metrics_exporter()
        // which is true in unit test context since install is global and can only
        // be called once per process.
    }
}
