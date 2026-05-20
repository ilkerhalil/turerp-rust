//! Observability domain models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Overall health status of a component
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "healthy"),
            HealthStatus::Degraded => write!(f, "degraded"),
            HealthStatus::Unhealthy => write!(f, "unhealthy"),
        }
    }
}

/// Result of a single health check probe
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HealthCheckResult {
    pub component: String,
    pub status: HealthStatus,
    pub latency_ms: u64,
    pub message: Option<String>,
    pub checked_at: DateTime<Utc>,
}

/// System-wide health summary
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SystemHealthSummary {
    pub overall: HealthStatus,
    pub version: String,
    pub checks: Vec<HealthCheckResult>,
    pub checked_at: DateTime<Utc>,
}

/// SLI metric type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum SliMetricType {
    Availability,
    Latency,
    ErrorRate,
    Throughput,
    InvoiceCreationLatency,
    PaymentSuccessRate,
    StockUpdateLatency,
    SalesOrderThroughput,
}

impl std::fmt::Display for SliMetricType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SliMetricType::Availability => write!(f, "availability"),
            SliMetricType::Latency => write!(f, "latency"),
            SliMetricType::ErrorRate => write!(f, "error_rate"),
            SliMetricType::Throughput => write!(f, "throughput"),
            SliMetricType::InvoiceCreationLatency => write!(f, "invoice_creation_latency"),
            SliMetricType::PaymentSuccessRate => write!(f, "payment_success_rate"),
            SliMetricType::StockUpdateLatency => write!(f, "stock_update_latency"),
            SliMetricType::SalesOrderThroughput => write!(f, "sales_order_throughput"),
        }
    }
}

/// Definition of a Service Level Indicator
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SliDefinition {
    pub id: String,
    pub name: String,
    pub metric_type: SliMetricType,
    pub source: String,
    pub window_minutes: i64,
    pub created_at: DateTime<Utc>,
}

/// A single SLI measurement
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SliMeasurement {
    pub sli_id: String,
    pub value: f64,
    pub recorded_at: DateTime<Utc>,
}

/// SLO target comparison operator
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum SloTarget {
    Gte,
    Lte,
}

impl std::fmt::Display for SloTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SloTarget::Gte => write!(f, ">="),
            SloTarget::Lte => write!(f, "<="),
        }
    }
}

/// Definition of a Service Level Objective
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SloDefinition {
    pub id: String,
    pub name: String,
    pub sli_id: String,
    pub target_value: f64,
    pub target_operator: SloTarget,
    pub error_budget: f64,
    pub window_days: i64,
    pub created_at: DateTime<Utc>,
}

/// SLO compliance status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum SloStatus {
    Compliant,
    AtRisk,
    Breached,
}

impl std::fmt::Display for SloStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SloStatus::Compliant => write!(f, "compliant"),
            SloStatus::AtRisk => write!(f, "at_risk"),
            SloStatus::Breached => write!(f, "breached"),
        }
    }
}

/// SLO compliance report
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SloCompliance {
    pub slo_id: String,
    pub slo_name: String,
    pub current_value: f64,
    pub target_value: f64,
    pub status: SloStatus,
    pub error_budget_remaining: f64,
    pub measured_at: DateTime<Utc>,
}

/// Alert severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

impl std::fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlertSeverity::Info => write!(f, "info"),
            AlertSeverity::Warning => write!(f, "warning"),
            AlertSeverity::Critical => write!(f, "critical"),
        }
    }
}

/// Alert state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum AlertState {
    Firing,
    Resolved,
    Silenced,
}

impl std::fmt::Display for AlertState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlertState::Firing => write!(f, "firing"),
            AlertState::Resolved => write!(f, "resolved"),
            AlertState::Silenced => write!(f, "silenced"),
        }
    }
}

/// Alert rule definition
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AlertRule {
    pub id: String,
    pub name: String,
    pub metric: String,
    pub condition: String,
    pub threshold: f64,
    pub severity: AlertSeverity,
    pub duration_sec: i64,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

/// Alert instance
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Alert {
    pub id: String,
    pub rule_id: String,
    pub rule_name: String,
    pub severity: AlertSeverity,
    pub state: AlertState,
    pub message: String,
    pub value: f64,
    pub fired_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

/// Sparkline data point for dashboard charts
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SparklineDataPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
}

/// Snapshot of a single metric value parsed from Prometheus output.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MetricSnapshot {
    pub name: String,
    pub labels: std::collections::HashMap<String, String>,
    pub value: f64,
    pub captured_at: DateTime<Utc>,
}

/// Static Grafana dashboard JSON model served via API.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DashboardJson {
    pub name: String,
    pub title: String,
    pub panels: Vec<serde_json::Value>,
    pub tags: Vec<String>,
}
