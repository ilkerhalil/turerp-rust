//! Observability repository trait

use async_trait::async_trait;
use std::sync::Arc;

use crate::domain::observability::model::{
    Alert, AlertRule, HealthCheckResult, SliDefinition, SliMeasurement, SloCompliance,
    SloDefinition, SparklineDataPoint,
};
use crate::error::ApiError;

/// Repository trait for observability data
#[async_trait]
pub trait ObservabilityRepository: Send + Sync {
    /// Record a health check result
    async fn record_health_check(&self, result: HealthCheckResult) -> Result<(), ApiError>;

    /// Get recent health check results
    async fn get_recent_health_checks(
        &self,
        limit: i64,
    ) -> Result<Vec<HealthCheckResult>, ApiError>;

    /// Store an SLI definition
    async fn create_sli(&self, sli: SliDefinition) -> Result<SliDefinition, ApiError>;

    /// List all SLI definitions
    async fn list_slis(&self) -> Result<Vec<SliDefinition>, ApiError>;

    /// Record an SLI measurement
    async fn record_sli_measurement(&self, measurement: SliMeasurement) -> Result<(), ApiError>;

    /// Get SLI measurements within a time window
    async fn get_sli_measurements(
        &self,
        sli_id: &str,
        minutes: i64,
    ) -> Result<Vec<SliMeasurement>, ApiError>;

    /// Store an SLO definition
    async fn create_slo(&self, slo: SloDefinition) -> Result<SloDefinition, ApiError>;

    /// List all SLO definitions
    async fn list_slos(&self) -> Result<Vec<SloDefinition>, ApiError>;

    /// Store SLO compliance snapshot
    async fn record_slo_compliance(&self, compliance: SloCompliance) -> Result<(), ApiError>;

    /// Get latest SLO compliance for all SLOs
    async fn get_slo_compliance(&self) -> Result<Vec<SloCompliance>, ApiError>;

    /// Create an alert rule
    async fn create_alert_rule(&self, rule: AlertRule) -> Result<AlertRule, ApiError>;

    /// List all alert rules
    async fn list_alert_rules(&self) -> Result<Vec<AlertRule>, ApiError>;

    /// Get an alert rule by ID
    async fn get_alert_rule(&self, id: &str) -> Result<Option<AlertRule>, ApiError>;

    /// Delete an alert rule
    async fn delete_alert_rule(&self, id: &str) -> Result<(), ApiError>;

    /// Create an alert instance
    async fn create_alert(&self, alert: Alert) -> Result<Alert, ApiError>;

    /// List recent alerts
    async fn list_alerts(&self, limit: i64) -> Result<Vec<Alert>, ApiError>;

    /// Resolve an alert
    async fn resolve_alert(&self, id: &str) -> Result<(), ApiError>;

    /// Get sparkline data for a metric
    async fn get_sparkline(
        &self,
        metric: &str,
        minutes: i64,
    ) -> Result<Vec<SparklineDataPoint>, ApiError>;

    /// Record a sparkline data point for a metric
    async fn record_sparkline(&self, metric: &str, value: f64) -> Result<(), ApiError>;
}

/// Type alias for boxed observability repository
pub type BoxObservabilityRepository = Arc<dyn ObservabilityRepository>;

use parking_lot::Mutex;
use std::collections::HashMap;

/// In-memory observability repository
pub struct InMemoryObservabilityRepository {
    health_checks: Mutex<Vec<HealthCheckResult>>,
    slis: Mutex<HashMap<String, SliDefinition>>,
    sli_measurements: Mutex<HashMap<String, Vec<SliMeasurement>>>,
    slos: Mutex<HashMap<String, SloDefinition>>,
    slo_compliance: Mutex<HashMap<String, SloCompliance>>,
    alert_rules: Mutex<HashMap<String, AlertRule>>,
    alerts: Mutex<Vec<Alert>>,
    sparklines: Mutex<HashMap<String, Vec<SparklineDataPoint>>>,
}

impl InMemoryObservabilityRepository {
    pub fn new() -> Self {
        Self {
            health_checks: Mutex::new(Vec::new()),
            slis: Mutex::new(HashMap::new()),
            sli_measurements: Mutex::new(HashMap::new()),
            slos: Mutex::new(HashMap::new()),
            slo_compliance: Mutex::new(HashMap::new()),
            alert_rules: Mutex::new(HashMap::new()),
            alerts: Mutex::new(Vec::new()),
            sparklines: Mutex::new(HashMap::new()),
        }
    }

    /// Disable an alert rule by ID (test helper)
    pub fn disable_alert_rule(&self, id: &str) {
        let mut rules = self.alert_rules.lock();
        if let Some(r) = rules.get_mut(id) {
            r.enabled = false;
        }
    }
}

impl Default for InMemoryObservabilityRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ObservabilityRepository for InMemoryObservabilityRepository {
    async fn record_health_check(&self, result: HealthCheckResult) -> Result<(), ApiError> {
        let mut checks = self.health_checks.lock();
        checks.push(result);
        if checks.len() > 100 {
            checks.remove(0);
        }
        Ok(())
    }

    async fn get_recent_health_checks(
        &self,
        limit: i64,
    ) -> Result<Vec<HealthCheckResult>, ApiError> {
        let checks = self.health_checks.lock();
        let limit = limit as usize;
        Ok(checks.iter().rev().take(limit).cloned().collect())
    }

    async fn create_sli(&self, sli: SliDefinition) -> Result<SliDefinition, ApiError> {
        let mut slis = self.slis.lock();
        slis.insert(sli.id.clone(), sli.clone());
        Ok(sli)
    }

    async fn list_slis(&self) -> Result<Vec<SliDefinition>, ApiError> {
        let slis = self.slis.lock();
        Ok(slis.values().cloned().collect())
    }

    async fn record_sli_measurement(&self, measurement: SliMeasurement) -> Result<(), ApiError> {
        let mut measurements = self.sli_measurements.lock();
        let list = measurements.entry(measurement.sli_id.clone()).or_default();
        list.push(measurement);
        Ok(())
    }

    async fn get_sli_measurements(
        &self,
        sli_id: &str,
        _minutes: i64,
    ) -> Result<Vec<SliMeasurement>, ApiError> {
        let measurements = self.sli_measurements.lock();
        Ok(measurements.get(sli_id).cloned().unwrap_or_default())
    }

    async fn create_slo(&self, slo: SloDefinition) -> Result<SloDefinition, ApiError> {
        let mut slos = self.slos.lock();
        slos.insert(slo.id.clone(), slo.clone());
        Ok(slo)
    }

    async fn list_slos(&self) -> Result<Vec<SloDefinition>, ApiError> {
        let slos = self.slos.lock();
        Ok(slos.values().cloned().collect())
    }

    async fn record_slo_compliance(&self, compliance: SloCompliance) -> Result<(), ApiError> {
        let mut map = self.slo_compliance.lock();
        map.insert(compliance.slo_id.clone(), compliance);
        Ok(())
    }

    async fn get_slo_compliance(&self) -> Result<Vec<SloCompliance>, ApiError> {
        let map = self.slo_compliance.lock();
        Ok(map.values().cloned().collect())
    }

    async fn create_alert_rule(&self, rule: AlertRule) -> Result<AlertRule, ApiError> {
        let mut rules = self.alert_rules.lock();
        rules.insert(rule.id.clone(), rule.clone());
        Ok(rule)
    }

    async fn list_alert_rules(&self) -> Result<Vec<AlertRule>, ApiError> {
        let rules = self.alert_rules.lock();
        Ok(rules.values().cloned().collect())
    }

    async fn get_alert_rule(&self, id: &str) -> Result<Option<AlertRule>, ApiError> {
        let rules = self.alert_rules.lock();
        Ok(rules.get(id).cloned())
    }

    async fn delete_alert_rule(&self, id: &str) -> Result<(), ApiError> {
        let mut rules = self.alert_rules.lock();
        rules.remove(id);
        Ok(())
    }

    async fn create_alert(&self, alert: Alert) -> Result<Alert, ApiError> {
        let mut alerts = self.alerts.lock();
        alerts.push(alert.clone());
        Ok(alert)
    }

    async fn list_alerts(&self, limit: i64) -> Result<Vec<Alert>, ApiError> {
        let alerts = self.alerts.lock();
        let limit = limit as usize;
        Ok(alerts.iter().rev().take(limit).cloned().collect())
    }

    async fn resolve_alert(&self, id: &str) -> Result<(), ApiError> {
        let mut alerts = self.alerts.lock();
        for alert in alerts.iter_mut() {
            if alert.id == id {
                alert.state = crate::domain::observability::model::AlertState::Resolved;
                alert.resolved_at = Some(chrono::Utc::now());
                break;
            }
        }
        Ok(())
    }

    async fn get_sparkline(
        &self,
        metric: &str,
        _minutes: i64,
    ) -> Result<Vec<SparklineDataPoint>, ApiError> {
        let sparklines = self.sparklines.lock();
        Ok(sparklines.get(metric).cloned().unwrap_or_default())
    }

    async fn record_sparkline(&self, metric: &str, value: f64) -> Result<(), ApiError> {
        let mut sparklines = self.sparklines.lock();
        let list = sparklines.entry(metric.to_string()).or_default();
        list.push(SparklineDataPoint {
            timestamp: chrono::Utc::now(),
            value,
        });
        // Keep last 1000 data points per metric to bound memory
        if list.len() > 1000 {
            list.remove(0);
        }
        Ok(())
    }
}
