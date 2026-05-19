//! PostgreSQL observability repository implementation

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::db::error::map_sqlx_error;
use crate::domain::observability::model::{
    Alert, AlertRule, AlertSeverity, AlertState, HealthCheckResult, HealthStatus, SliDefinition,
    SliMeasurement, SliMetricType, SloCompliance, SloDefinition, SloStatus, SloTarget,
    SparklineDataPoint,
};
use crate::domain::observability::repository::ObservabilityRepository;
use crate::error::ApiError;

/// PostgreSQL observability repository
pub struct PostgresObservabilityRepository {
    pool: Arc<PgPool>,
}

impl PostgresObservabilityRepository {
    /// Create a new PostgreSQL observability repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> Arc<dyn ObservabilityRepository> {
        Arc::new(self) as Arc<dyn ObservabilityRepository>
    }
}

/// Row type for health checks
#[derive(Debug, FromRow)]
struct HealthCheckRow {
    component: String,
    status: String,
    latency_ms: i64,
    message: Option<String>,
    checked_at: DateTime<Utc>,
}

impl From<HealthCheckRow> for HealthCheckResult {
    fn from(row: HealthCheckRow) -> Self {
        Self {
            component: row.component,
            status: parse_health_status(&row.status),
            latency_ms: row.latency_ms as u64,
            message: row.message,
            checked_at: row.checked_at,
        }
    }
}

fn parse_health_status(s: &str) -> HealthStatus {
    match s {
        "healthy" => HealthStatus::Healthy,
        "degraded" => HealthStatus::Degraded,
        _ => HealthStatus::Unhealthy,
    }
}

fn health_status_str(status: HealthStatus) -> &'static str {
    match status {
        HealthStatus::Healthy => "healthy",
        HealthStatus::Degraded => "degraded",
        HealthStatus::Unhealthy => "unhealthy",
    }
}

/// Row type for SLI definitions
#[derive(Debug, FromRow)]
struct SliRow {
    id: String,
    name: String,
    metric_type: String,
    source: String,
    window_minutes: i32,
    created_at: DateTime<Utc>,
}

impl From<SliRow> for SliDefinition {
    fn from(row: SliRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            metric_type: parse_sli_metric_type(&row.metric_type),
            source: row.source,
            window_minutes: row.window_minutes as i64,
            created_at: row.created_at,
        }
    }
}

fn parse_sli_metric_type(s: &str) -> SliMetricType {
    match s {
        "availability" => SliMetricType::Availability,
        "latency" => SliMetricType::Latency,
        "error_rate" => SliMetricType::ErrorRate,
        "throughput" => SliMetricType::Throughput,
        "invoice_creation_latency" => SliMetricType::InvoiceCreationLatency,
        "payment_success_rate" => SliMetricType::PaymentSuccessRate,
        "stock_update_latency" => SliMetricType::StockUpdateLatency,
        "sales_order_throughput" => SliMetricType::SalesOrderThroughput,
        _ => SliMetricType::Throughput,
    }
}

fn sli_metric_type_str(metric_type: SliMetricType) -> &'static str {
    match metric_type {
        SliMetricType::Availability => "availability",
        SliMetricType::Latency => "latency",
        SliMetricType::ErrorRate => "error_rate",
        SliMetricType::Throughput => "throughput",
        SliMetricType::InvoiceCreationLatency => "invoice_creation_latency",
        SliMetricType::PaymentSuccessRate => "payment_success_rate",
        SliMetricType::StockUpdateLatency => "stock_update_latency",
        SliMetricType::SalesOrderThroughput => "sales_order_throughput",
    }
}

/// Row type for SLI measurements
#[derive(Debug, FromRow)]
struct SliMeasurementRow {
    sli_id: String,
    value: f64,
    recorded_at: DateTime<Utc>,
}

impl From<SliMeasurementRow> for SliMeasurement {
    fn from(row: SliMeasurementRow) -> Self {
        Self {
            sli_id: row.sli_id,
            value: row.value,
            recorded_at: row.recorded_at,
        }
    }
}

/// Row type for SLO definitions
#[derive(Debug, FromRow)]
struct SloRow {
    id: String,
    name: String,
    sli_id: String,
    target_value: f64,
    target_operator: String,
    error_budget: f64,
    window_days: i32,
    created_at: DateTime<Utc>,
}

impl From<SloRow> for SloDefinition {
    fn from(row: SloRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            sli_id: row.sli_id,
            target_value: row.target_value,
            target_operator: parse_slo_target(&row.target_operator),
            error_budget: row.error_budget,
            window_days: row.window_days as i64,
            created_at: row.created_at,
        }
    }
}

fn parse_slo_target(s: &str) -> SloTarget {
    match s {
        "Lte" => SloTarget::Lte,
        _ => SloTarget::Gte,
    }
}

fn slo_target_str(target: SloTarget) -> &'static str {
    match target {
        SloTarget::Gte => "Gte",
        SloTarget::Lte => "Lte",
    }
}

/// Row type for SLO compliance
#[derive(Debug, FromRow)]
struct SloComplianceRow {
    slo_id: String,
    slo_name: String,
    current_value: f64,
    target_value: f64,
    status: String,
    error_budget_remaining: f64,
    measured_at: DateTime<Utc>,
}

impl From<SloComplianceRow> for SloCompliance {
    fn from(row: SloComplianceRow) -> Self {
        Self {
            slo_id: row.slo_id,
            slo_name: row.slo_name,
            current_value: row.current_value,
            target_value: row.target_value,
            status: parse_slo_status(&row.status),
            error_budget_remaining: row.error_budget_remaining,
            measured_at: row.measured_at,
        }
    }
}

fn parse_slo_status(s: &str) -> SloStatus {
    match s {
        "compliant" => SloStatus::Compliant,
        "at_risk" => SloStatus::AtRisk,
        _ => SloStatus::Breached,
    }
}

fn slo_status_str(status: SloStatus) -> &'static str {
    match status {
        SloStatus::Compliant => "compliant",
        SloStatus::AtRisk => "at_risk",
        SloStatus::Breached => "breached",
    }
}

/// Row type for alert rules
#[derive(Debug, FromRow)]
struct AlertRuleRow {
    id: String,
    name: String,
    metric: String,
    condition: String,
    threshold: f64,
    severity: String,
    duration_sec: i32,
    enabled: bool,
    created_at: DateTime<Utc>,
}

impl From<AlertRuleRow> for AlertRule {
    fn from(row: AlertRuleRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            metric: row.metric,
            condition: row.condition,
            threshold: row.threshold,
            severity: parse_alert_severity(&row.severity),
            duration_sec: row.duration_sec as i64,
            enabled: row.enabled,
            created_at: row.created_at,
        }
    }
}

fn parse_alert_severity(s: &str) -> AlertSeverity {
    match s {
        "warning" => AlertSeverity::Warning,
        "critical" => AlertSeverity::Critical,
        _ => AlertSeverity::Info,
    }
}

fn alert_severity_str(severity: AlertSeverity) -> &'static str {
    match severity {
        AlertSeverity::Info => "info",
        AlertSeverity::Warning => "warning",
        AlertSeverity::Critical => "critical",
    }
}

/// Row type for alerts
#[derive(Debug, FromRow)]
struct AlertRow {
    id: String,
    rule_id: String,
    rule_name: String,
    severity: String,
    state: String,
    message: String,
    value: Option<f64>,
    fired_at: DateTime<Utc>,
    resolved_at: Option<DateTime<Utc>>,
}

impl From<AlertRow> for Alert {
    fn from(row: AlertRow) -> Self {
        Self {
            id: row.id,
            rule_id: row.rule_id,
            rule_name: row.rule_name,
            severity: parse_alert_severity(&row.severity),
            state: parse_alert_state(&row.state),
            message: row.message,
            value: row.value.unwrap_or(0.0),
            fired_at: row.fired_at,
            resolved_at: row.resolved_at,
        }
    }
}

fn parse_alert_state(s: &str) -> AlertState {
    match s {
        "resolved" => AlertState::Resolved,
        "silenced" => AlertState::Silenced,
        _ => AlertState::Firing,
    }
}

fn alert_state_str(state: AlertState) -> &'static str {
    match state {
        AlertState::Firing => "firing",
        AlertState::Resolved => "resolved",
        AlertState::Silenced => "silenced",
    }
}

/// Row type for sparklines
#[derive(Debug, FromRow)]
#[allow(dead_code)]
struct SparklineRow {
    metric: String,
    value: f64,
    recorded_at: DateTime<Utc>,
}

impl From<SparklineRow> for SparklineDataPoint {
    fn from(row: SparklineRow) -> Self {
        Self {
            timestamp: row.recorded_at,
            value: row.value,
        }
    }
}

#[async_trait]
impl ObservabilityRepository for PostgresObservabilityRepository {
    async fn record_health_check(&self, result: HealthCheckResult) -> Result<(), ApiError> {
        sqlx::query(
            r#"
            INSERT INTO health_checks (component, status, latency_ms, message, checked_at)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(&result.component)
        .bind(health_status_str(result.status))
        .bind(result.latency_ms as i64)
        .bind(result.message.as_deref())
        .bind(result.checked_at)
        .execute(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "HealthCheck"))?;

        Ok(())
    }

    async fn get_recent_health_checks(
        &self,
        limit: i64,
    ) -> Result<Vec<HealthCheckResult>, ApiError> {
        let rows: Vec<HealthCheckRow> = sqlx::query_as(
            r#"
            SELECT component, status, latency_ms, message, checked_at
            FROM health_checks
            ORDER BY checked_at DESC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "HealthCheck"))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn create_sli(&self, sli: SliDefinition) -> Result<SliDefinition, ApiError> {
        sqlx::query(
            r#"
            INSERT INTO sli_definitions (id, name, metric_type, source, window_minutes, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(&sli.id)
        .bind(&sli.name)
        .bind(sli_metric_type_str(sli.metric_type))
        .bind(&sli.source)
        .bind(sli.window_minutes as i32)
        .bind(sli.created_at)
        .execute(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "SLI"))?;

        Ok(sli)
    }

    async fn list_slis(&self) -> Result<Vec<SliDefinition>, ApiError> {
        let rows: Vec<SliRow> = sqlx::query_as(
            r#"
            SELECT id, name, metric_type, source, window_minutes, created_at
            FROM sli_definitions
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "SLI"))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn record_sli_measurement(&self, measurement: SliMeasurement) -> Result<(), ApiError> {
        sqlx::query(
            r#"
            INSERT INTO sli_measurements (sli_id, value, recorded_at)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(&measurement.sli_id)
        .bind(measurement.value)
        .bind(measurement.recorded_at)
        .execute(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "SLI Measurement"))?;

        Ok(())
    }

    async fn get_sli_measurements(
        &self,
        sli_id: &str,
        minutes: i64,
    ) -> Result<Vec<SliMeasurement>, ApiError> {
        let since = Utc::now() - chrono::Duration::minutes(minutes);

        let rows: Vec<SliMeasurementRow> = sqlx::query_as(
            r#"
            SELECT sli_id, value, recorded_at
            FROM sli_measurements
            WHERE sli_id = $1 AND recorded_at >= $2
            ORDER BY recorded_at DESC
            "#,
        )
        .bind(sli_id)
        .bind(since)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "SLI Measurement"))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn create_slo(&self, slo: SloDefinition) -> Result<SloDefinition, ApiError> {
        sqlx::query(
            r#"
            INSERT INTO slo_definitions (id, name, sli_id, target_value, target_operator, error_budget, window_days, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(&slo.id)
        .bind(&slo.name)
        .bind(&slo.sli_id)
        .bind(slo.target_value)
        .bind(slo_target_str(slo.target_operator))
        .bind(slo.error_budget)
        .bind(slo.window_days as i32)
        .bind(slo.created_at)
        .execute(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "SLO"))?;

        Ok(slo)
    }

    async fn list_slos(&self) -> Result<Vec<SloDefinition>, ApiError> {
        let rows: Vec<SloRow> = sqlx::query_as(
            r#"
            SELECT id, name, sli_id, target_value, target_operator, error_budget, window_days, created_at
            FROM slo_definitions
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "SLO"))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn record_slo_compliance(&self, compliance: SloCompliance) -> Result<(), ApiError> {
        sqlx::query(
            r#"
            INSERT INTO slo_compliance (slo_id, slo_name, current_value, target_value, status, error_budget_remaining, measured_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(&compliance.slo_id)
        .bind(&compliance.slo_name)
        .bind(compliance.current_value)
        .bind(compliance.target_value)
        .bind(slo_status_str(compliance.status))
        .bind(compliance.error_budget_remaining)
        .bind(compliance.measured_at)
        .execute(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "SLO Compliance"))?;

        Ok(())
    }

    async fn get_slo_compliance(&self) -> Result<Vec<SloCompliance>, ApiError> {
        let rows: Vec<SloComplianceRow> = sqlx::query_as(
            r#"
            SELECT DISTINCT ON (slo_id) slo_id, slo_name, current_value, target_value, status, error_budget_remaining, measured_at
            FROM slo_compliance
            ORDER BY slo_id, measured_at DESC
            "#,
        )
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "SLO Compliance"))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn create_alert_rule(&self, rule: AlertRule) -> Result<AlertRule, ApiError> {
        sqlx::query(
            r#"
            INSERT INTO alert_rules (id, name, metric, condition, threshold, severity, duration_sec, enabled, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $9)
            "#,
        )
        .bind(&rule.id)
        .bind(&rule.name)
        .bind(&rule.metric)
        .bind(&rule.condition)
        .bind(rule.threshold)
        .bind(alert_severity_str(rule.severity))
        .bind(rule.duration_sec as i32)
        .bind(rule.enabled)
        .bind(rule.created_at)
        .execute(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Alert Rule"))?;

        Ok(rule)
    }

    async fn list_alert_rules(&self) -> Result<Vec<AlertRule>, ApiError> {
        let rows: Vec<AlertRuleRow> = sqlx::query_as(
            r#"
            SELECT id, name, metric, condition, threshold, severity, duration_sec, enabled, created_at
            FROM alert_rules
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Alert Rule"))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn get_alert_rule(&self, id: &str) -> Result<Option<AlertRule>, ApiError> {
        let row: Option<AlertRuleRow> = sqlx::query_as(
            r#"
            SELECT id, name, metric, condition, threshold, severity, duration_sec, enabled, created_at
            FROM alert_rules
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Alert Rule"))?;

        Ok(row.map(Into::into))
    }

    async fn delete_alert_rule(&self, id: &str) -> Result<(), ApiError> {
        sqlx::query(
            r#"
            DELETE FROM alert_rules WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Alert Rule"))?;

        Ok(())
    }

    async fn create_alert(&self, alert: Alert) -> Result<Alert, ApiError> {
        sqlx::query(
            r#"
            INSERT INTO alerts (id, rule_id, rule_name, severity, state, message, value, fired_at, resolved_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(&alert.id)
        .bind(&alert.rule_id)
        .bind(&alert.rule_name)
        .bind(alert_severity_str(alert.severity))
        .bind(alert_state_str(alert.state))
        .bind(&alert.message)
        .bind(alert.value)
        .bind(alert.fired_at)
        .bind(alert.resolved_at)
        .execute(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Alert"))?;

        Ok(alert)
    }

    async fn list_alerts(&self, limit: i64) -> Result<Vec<Alert>, ApiError> {
        let rows: Vec<AlertRow> = sqlx::query_as(
            r#"
            SELECT id, rule_id, rule_name, severity, state, message, value, fired_at, resolved_at
            FROM alerts
            ORDER BY fired_at DESC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Alert"))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn resolve_alert(&self, id: &str) -> Result<(), ApiError> {
        sqlx::query(
            r#"
            UPDATE alerts
            SET state = 'resolved', resolved_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Alert"))?;

        Ok(())
    }

    async fn get_sparkline(
        &self,
        metric: &str,
        minutes: i64,
    ) -> Result<Vec<SparklineDataPoint>, ApiError> {
        let since = Utc::now() - chrono::Duration::minutes(minutes);

        let rows: Vec<SparklineRow> = sqlx::query_as(
            r#"
            SELECT metric, value, recorded_at
            FROM sparklines
            WHERE metric = $1 AND recorded_at >= $2
            ORDER BY recorded_at ASC
            "#,
        )
        .bind(metric)
        .bind(since)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Sparkline"))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn record_sparkline(&self, metric: &str, value: f64) -> Result<(), ApiError> {
        sqlx::query(
            r#"
            INSERT INTO sparklines (metric, value, recorded_at)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(metric)
        .bind(value)
        .bind(Utc::now())
        .execute(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Sparkline"))?;

        Ok(())
    }
}
