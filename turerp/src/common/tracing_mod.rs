//! Distributed tracing integration with OpenTelemetry
//!
//! Provides a `TracingService` trait for span creation, propagation, and export.
//! The in-memory backend stores spans locally; production backends can export
//! to Jaeger/Tempo via `opentelemetry` + `tracing-opentelemetry`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Span status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpanStatus {
    Ok,
    Error,
    Unset,
}

/// A traced span
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSpan {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub operation_name: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub status: SpanStatus,
    pub tenant_id: Option<i64>,
    pub attributes: HashMap<String, String>,
    pub service_name: String,
}

impl TraceSpan {
    pub fn duration_ms(&self) -> Option<i64> {
        self.end_time
            .map(|end| (end - self.start_time).num_milliseconds())
    }
}

/// Trace context for propagation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceContext {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub trace_flags: u8,
}

/// Trace search query
#[derive(Debug, Clone)]
pub struct TraceQuery {
    pub tenant_id: Option<i64>,
    pub operation_name: Option<String>,
    pub trace_id: Option<String>,
    pub min_duration_ms: Option<i64>,
    pub status: Option<SpanStatus>,
    pub limit: u32,
}

/// Tracing service trait
#[async_trait::async_trait]
pub trait TracingService: Send + Sync {
    /// Start a new span
    async fn start_span(
        &self,
        operation: &str,
        context: Option<TraceContext>,
        tenant_id: Option<i64>,
    ) -> TraceSpan;

    /// End a span
    async fn end_span(&self, span: TraceSpan, status: SpanStatus);

    /// Get spans for a trace
    async fn get_trace(&self, trace_id: &str) -> Result<Vec<TraceSpan>, String>;

    /// Search traces
    async fn search_traces(&self, query: TraceQuery) -> Result<Vec<TraceSpan>, String>;

    /// Extract trace context from carrier (e.g., HTTP headers)
    fn extract_context(&self, carrier: &HashMap<String, String>) -> Option<TraceContext>;

    /// Inject trace context into carrier
    fn inject_context(&self, context: &TraceContext, carrier: &mut HashMap<String, String>);
}

/// Type alias for boxed tracing service
pub type BoxTracingService = std::sync::Arc<dyn TracingService>;

/// In-memory tracing service
pub struct InMemoryTracingService {
    spans: parking_lot::RwLock<Vec<TraceSpan>>,
    service_name: String,
}

impl InMemoryTracingService {
    pub fn new(service_name: &str) -> Self {
        Self {
            spans: parking_lot::RwLock::new(Vec::new()),
            service_name: service_name.to_string(),
        }
    }

    fn generate_id() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let bytes: [u8; 8] = rng.gen();
        let mut s = String::with_capacity(16);
        for &b in &bytes {
            use std::fmt::Write;
            write!(&mut s, "{:02x}", b).unwrap();
        }
        s
    }

    fn generate_trace_id() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let bytes: [u8; 16] = rng.gen();
        let mut s = String::with_capacity(32);
        for &b in &bytes {
            use std::fmt::Write;
            write!(&mut s, "{:02x}", b).unwrap();
        }
        s
    }
}

impl Default for InMemoryTracingService {
    fn default() -> Self {
        Self::new("turerp-erp")
    }
}

#[async_trait::async_trait]
impl TracingService for InMemoryTracingService {
    async fn start_span(
        &self,
        operation: &str,
        context: Option<TraceContext>,
        tenant_id: Option<i64>,
    ) -> TraceSpan {
        let (trace_id, parent_span_id) = match context {
            Some(ctx) => (ctx.trace_id, Some(ctx.span_id)),
            None => (Self::generate_trace_id(), None),
        };

        TraceSpan {
            trace_id,
            span_id: Self::generate_id(),
            parent_span_id,
            operation_name: operation.to_string(),
            start_time: Utc::now(),
            end_time: None,
            status: SpanStatus::Unset,
            tenant_id,
            attributes: HashMap::new(),
            service_name: self.service_name.clone(),
        }
    }

    async fn end_span(&self, mut span: TraceSpan, status: SpanStatus) {
        span.end_time = Some(Utc::now());
        span.status = status;
        self.spans.write().push(span);
    }

    async fn get_trace(&self, trace_id: &str) -> Result<Vec<TraceSpan>, String> {
        let spans = self.spans.read();
        Ok(spans
            .iter()
            .filter(|s| s.trace_id == trace_id)
            .cloned()
            .collect())
    }

    async fn search_traces(&self, query: TraceQuery) -> Result<Vec<TraceSpan>, String> {
        let spans = self.spans.read();
        Ok(spans
            .iter()
            .filter(|s| {
                if let Some(tid) = query.tenant_id {
                    if s.tenant_id != Some(tid) {
                        return false;
                    }
                }
                if let Some(ref op) = query.operation_name {
                    if !s.operation_name.contains(op) {
                        return false;
                    }
                }
                if let Some(ref tid) = query.trace_id {
                    if s.trace_id != *tid {
                        return false;
                    }
                }
                if let Some(min_ms) = query.min_duration_ms {
                    if s.duration_ms().is_none_or(|d| d < min_ms) {
                        return false;
                    }
                }
                if let Some(status) = query.status {
                    if s.status != status {
                        return false;
                    }
                }
                true
            })
            .take(query.limit as usize)
            .cloned()
            .collect())
    }

    fn extract_context(&self, carrier: &HashMap<String, String>) -> Option<TraceContext> {
        // W3C Trace Context: traceparent header
        let traceparent = carrier.get("traceparent")?;
        let parts: Vec<&str> = traceparent.split('-').collect();
        if parts.len() < 3 {
            return None;
        }
        Some(TraceContext {
            trace_id: parts[1].to_string(),
            span_id: parts[2].to_string(),
            parent_span_id: None,
            trace_flags: u8::from_str_radix(parts.get(3).unwrap_or(&"01"), 16).unwrap_or(1),
        })
    }

    fn inject_context(&self, context: &TraceContext, carrier: &mut HashMap<String, String>) {
        let traceparent = format!(
            "00-{}-{}-{:02x}",
            context.trace_id, context.span_id, context.trace_flags
        );
        carrier.insert("traceparent".to_string(), traceparent);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_service() -> InMemoryTracingService {
        InMemoryTracingService::new("test-service")
    }

    #[tokio::test]
    async fn test_start_and_end_span() {
        let service = create_service();
        let span = service.start_span("GET /api/v1/cari", None, Some(1)).await;
        assert_eq!(span.operation_name, "GET /api/v1/cari");
        assert!(span.end_time.is_none());

        service.end_span(span, SpanStatus::Ok).await;

        let traces = service
            .get_trace(&service.spans.read()[0].trace_id)
            .await
            .unwrap();
        assert_eq!(traces.len(), 1);
        assert!(traces[0].end_time.is_some());
        assert_eq!(traces[0].status, SpanStatus::Ok);
    }

    #[tokio::test]
    async fn test_span_duration() {
        let service = create_service();
        let span = service.start_span("operation", None, None).await;
        service.end_span(span, SpanStatus::Ok).await;

        let stored = &service.spans.read()[0];
        assert!(stored.duration_ms().unwrap_or(0) >= 0);
    }

    #[tokio::test]
    async fn test_child_span_propagation() {
        let service = create_service();
        let parent = service.start_span("parent", None, Some(1)).await;
        let ctx = TraceContext {
            trace_id: parent.trace_id.clone(),
            span_id: parent.span_id.clone(),
            parent_span_id: None,
            trace_flags: 1,
        };
        let child = service.start_span("child", Some(ctx), Some(1)).await;
        assert_eq!(child.trace_id, parent.trace_id);
        assert_eq!(child.parent_span_id, Some(parent.span_id));
    }

    #[tokio::test]
    async fn test_search_traces() {
        let service = create_service();
        let span = service
            .start_span("GET /api/v1/invoices", None, Some(1))
            .await;
        service.end_span(span, SpanStatus::Ok).await;

        let results = service
            .search_traces(TraceQuery {
                tenant_id: Some(1),
                operation_name: Some("invoices".to_string()),
                trace_id: None,
                min_duration_ms: None,
                status: Some(SpanStatus::Ok),
                limit: 10,
            })
            .await
            .unwrap();
        assert_eq!(results.len(), 1);

        let no_results = service
            .search_traces(TraceQuery {
                tenant_id: Some(99),
                operation_name: None,
                trace_id: None,
                min_duration_ms: None,
                status: None,
                limit: 10,
            })
            .await
            .unwrap();
        assert!(no_results.is_empty());
    }

    #[tokio::test]
    async fn test_w3c_trace_context_propagation() {
        let service = create_service();
        let mut carrier = HashMap::new();
        carrier.insert(
            "traceparent".to_string(),
            "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01".to_string(),
        );

        let ctx = service.extract_context(&carrier).unwrap();
        assert_eq!(ctx.trace_id, "4bf92f3577b34da6a3ce929d0e0e4736");
        assert_eq!(ctx.span_id, "00f067aa0ba902b7");

        let mut out = HashMap::new();
        service.inject_context(&ctx, &mut out);
        let traceparent = out.get("traceparent").unwrap();
        assert!(traceparent.starts_with("00-"));
    }
}
