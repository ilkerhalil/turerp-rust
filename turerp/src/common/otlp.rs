//! OpenTelemetry OTLP pipeline for Aspire Dashboard integration
//!
//! Exports traces, metrics, and logs via OTLP to the Aspire Dashboard.
//!
//! # Security Note
//! OTLP exporters use plain HTTP by default. In production, ensure the OTLP
//! endpoint is on an encrypted channel (e.g., mTLS sidecar, encrypted overlay
//! network, or HTTPS with `with_tls_config`).

use opentelemetry::trace::TracerProvider;
use opentelemetry::{global, KeyValue};
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::{LogExporter, MetricExporter, SpanExporter, WithExportConfig};
use opentelemetry_sdk::logs::SdkLoggerProvider;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::resource::Resource;
use opentelemetry_sdk::trace::{BatchSpanProcessor, SdkTracerProvider};
use opentelemetry_semantic_conventions::resource::SERVICE_NAME;

fn build_resource() -> Resource {
    Resource::builder_empty()
        .with_attributes([KeyValue::new(SERVICE_NAME, "turerp")])
        .build()
}

/// Create a `tracing-opentelemetry` layer for the tracing subscriber.
///
/// The returned layer must be added to the subscriber **before** `.init()` is called.
/// This function also installs the global tracer provider.
pub fn create_otlp_trace_layer(
    endpoint: &str,
) -> Result<
    tracing_opentelemetry::OpenTelemetryLayer<
        tracing_subscriber::Registry,
        opentelemetry_sdk::trace::SdkTracer,
    >,
    String,
> {
    let span_exporter = SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint.to_string())
        .build()
        .map_err(|e| format!("Failed to build OTLP span exporter: {}", e))?;

    let span_processor = BatchSpanProcessor::builder(span_exporter).build();

    let tracer_provider = SdkTracerProvider::builder()
        .with_span_processor(span_processor)
        .with_resource(build_resource())
        .build();

    global::set_tracer_provider(tracer_provider.clone());

    let tracer = tracer_provider.tracer("turerp");
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    Ok(telemetry)
}

/// Create the OTLP log layer.
pub fn create_otlp_log_layer(
    endpoint: &str,
) -> Result<
    opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge<
        opentelemetry_sdk::logs::SdkLoggerProvider,
        opentelemetry_sdk::logs::SdkLogger,
    >,
    String,
> {
    let log_exporter = LogExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint.to_string())
        .build()
        .map_err(|e| format!("Failed to build OTLP log exporter: {}", e))?;

    let logger_provider = SdkLoggerProvider::builder()
        .with_resource(build_resource())
        .with_batch_exporter(log_exporter)
        .build();

    let layer = OpenTelemetryTracingBridge::new(&logger_provider);

    // SAFETY: Intentionally leak the logger provider so it outlives the tracing
    // layer, which borrows it. This is acceptable for a long-running server
    // process where the provider lives until process termination.
    let _ = Box::leak(Box::new(logger_provider));

    Ok(layer)
}

/// Install the OTLP metric pipeline.
///
/// Returns a `SdkMeterProvider` that must be kept alive for the lifetime of the process.
pub fn install_otlp_metrics(endpoint: &str) -> Result<SdkMeterProvider, String> {
    let metric_exporter = MetricExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint.to_string())
        .build()
        .map_err(|e| format!("Failed to build OTLP metric exporter: {}", e))?;

    let provider = opentelemetry_sdk::metrics::SdkMeterProvider::builder()
        .with_periodic_exporter(metric_exporter)
        .with_resource(build_resource())
        .build();

    global::set_meter_provider(provider.clone());

    Ok(provider)
}
