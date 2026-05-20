//! Observability service for health checks, SLI/SLO tracking, and alerting

use crate::cache::CacheService;
use crate::cache::{cache_get, cache_key, cache_set};
use crate::common::alert_duration_tracker::AlertDurationTracker;
use crate::common::notifications::{NotificationPriority, NotificationRequest};
use crate::common::NotificationService;
use crate::domain::observability::model::{
    Alert, AlertRule, AlertSeverity, AlertState, HealthCheckResult, HealthStatus, SliDefinition,
    SliMeasurement, SliMetricType, SloCompliance, SloDefinition, SloStatus, SparklineDataPoint,
    SystemHealthSummary,
};
use crate::domain::observability::repository::BoxObservabilityRepository;
use crate::error::ApiError;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use utoipa::ToSchema;

/// Cache TTL for observability data (seconds)
const OBSERVABILITY_CACHE_TTL: u64 = 30;

/// Observability service
#[derive(Clone)]
pub struct ObservabilityService {
    repo: BoxObservabilityRepository,
    cache: Arc<dyn CacheService>,
    notification_service: Option<Arc<dyn NotificationService>>,
    next_alert_id: Arc<Mutex<i64>>,
    next_rule_id: Arc<Mutex<i64>>,
    alert_tracker: Arc<Mutex<AlertDurationTracker>>,
}

impl ObservabilityService {
    /// Create a new observability service
    pub fn new(repo: BoxObservabilityRepository, cache: Arc<dyn CacheService>) -> Self {
        Self {
            repo,
            cache,
            notification_service: None,
            next_alert_id: Arc::new(Mutex::new(1)),
            next_rule_id: Arc::new(Mutex::new(1)),
            alert_tracker: Arc::new(Mutex::new(AlertDurationTracker::new())),
        }
    }

    /// Attach a notification service for alert dispatch
    pub fn with_notification(mut self, notification: Arc<dyn NotificationService>) -> Self {
        self.notification_service = Some(notification);
        self
    }

    // ── Health Checks ───────────────────────────────────────────────

    /// Run a system health check
    pub async fn run_health_check(
        &self,
        db_check: Option<&sqlx::PgPool>,
    ) -> Result<SystemHealthSummary, ApiError> {
        let mut checks = Vec::new();
        let now = chrono::Utc::now();

        // Application check
        checks.push(HealthCheckResult {
            component: "app".to_string(),
            status: HealthStatus::Healthy,
            latency_ms: 0,
            message: Some("Application is running".to_string()),
            checked_at: now,
        });

        // Database check
        if let Some(pool) = db_check {
            let start = std::time::Instant::now();
            let db_result = sqlx::query("SELECT 1").execute(pool).await;
            let latency_ms = start.elapsed().as_millis() as u64;
            match db_result {
                Ok(_) => {
                    checks.push(HealthCheckResult {
                        component: "database".to_string(),
                        status: HealthStatus::Healthy,
                        latency_ms,
                        message: Some("Database connection healthy".to_string()),
                        checked_at: now,
                    });
                }
                Err(e) => {
                    checks.push(HealthCheckResult {
                        component: "database".to_string(),
                        status: HealthStatus::Unhealthy,
                        latency_ms,
                        message: Some(format!("Database error: {}", e)),
                        checked_at: now,
                    });
                }
            }
        }

        // Cache check
        {
            let start = std::time::Instant::now();
            let cache_key_str = cache_key(0, "health", "ping");
            let cache_result = cache_set(&*self.cache, &cache_key_str, &"pong", Some(5)).await;
            let latency_ms = start.elapsed().as_millis() as u64;
            match cache_result {
                Ok(_) => {
                    checks.push(HealthCheckResult {
                        component: "cache".to_string(),
                        status: HealthStatus::Healthy,
                        latency_ms,
                        message: Some("Cache service healthy".to_string()),
                        checked_at: now,
                    });
                }
                Err(e) => {
                    checks.push(HealthCheckResult {
                        component: "cache".to_string(),
                        status: HealthStatus::Degraded,
                        latency_ms,
                        message: Some(format!("Cache error: {}", e)),
                        checked_at: now,
                    });
                }
            }
        }

        let overall = checks
            .iter()
            .fold(HealthStatus::Healthy, |acc, c| match (acc, c.status) {
                (HealthStatus::Unhealthy, _) => HealthStatus::Unhealthy,
                (_, HealthStatus::Unhealthy) => HealthStatus::Unhealthy,
                (HealthStatus::Degraded, _) => HealthStatus::Degraded,
                (_, HealthStatus::Degraded) => HealthStatus::Degraded,
                _ => HealthStatus::Healthy,
            });

        let summary = SystemHealthSummary {
            overall,
            version: env!("CARGO_PKG_VERSION").to_string(),
            checks,
            checked_at: now,
        };

        for check in &summary.checks {
            self.repo.record_health_check(check.clone()).await?;
        }

        Ok(summary)
    }

    /// Get recent health check history
    pub async fn get_health_history(&self, limit: i64) -> Result<Vec<HealthCheckResult>, ApiError> {
        self.repo.get_recent_health_checks(limit).await
    }

    /// Get liveness status (always healthy if running)
    pub async fn get_liveness(&self) -> Result<SystemHealthSummary, ApiError> {
        let now = chrono::Utc::now();
        Ok(SystemHealthSummary {
            overall: HealthStatus::Healthy,
            version: env!("CARGO_PKG_VERSION").to_string(),
            checks: vec![HealthCheckResult {
                component: "app".to_string(),
                status: HealthStatus::Healthy,
                latency_ms: 0,
                message: Some("Application is alive".to_string()),
                checked_at: now,
            }],
            checked_at: now,
        })
    }

    // ── SLI / SLO ───────────────────────────────────────────────────

    /// Create an SLI definition
    pub async fn create_sli(
        &self,
        name: String,
        metric_type: SliMetricType,
        source: String,
        window_minutes: i64,
    ) -> Result<SliDefinition, ApiError> {
        let id = format!("sli-{}", uuid::Uuid::new_v4());
        let sli = SliDefinition {
            id,
            name,
            metric_type,
            source,
            window_minutes,
            created_at: chrono::Utc::now(),
        };
        self.repo.create_sli(sli).await
    }

    /// List all SLIs
    pub async fn list_slis(&self) -> Result<Vec<SliDefinition>, ApiError> {
        self.repo.list_slis().await
    }

    /// Record an SLI measurement
    pub async fn record_sli(&self, sli_id: String, value: f64) -> Result<(), ApiError> {
        let measurement = SliMeasurement {
            sli_id,
            value,
            recorded_at: chrono::Utc::now(),
        };
        self.repo.record_sli_measurement(measurement).await
    }

    /// Create an SLO definition
    pub async fn create_slo(
        &self,
        name: String,
        sli_id: String,
        target_value: f64,
        error_budget: f64,
        window_days: i64,
    ) -> Result<SloDefinition, ApiError> {
        let id = format!("slo-{}", uuid::Uuid::new_v4());
        let slo = SloDefinition {
            id,
            name,
            sli_id,
            target_value,
            target_operator: crate::domain::observability::model::SloTarget::Gte,
            error_budget,
            window_days,
            created_at: chrono::Utc::now(),
        };
        self.repo.create_slo(slo).await
    }

    /// List all SLOs
    pub async fn list_slos(&self) -> Result<Vec<SloDefinition>, ApiError> {
        self.repo.list_slos().await
    }

    /// Evaluate SLO compliance for all SLOs
    pub async fn evaluate_slo_compliance(&self) -> Result<Vec<SloCompliance>, ApiError> {
        let slos = self.repo.list_slos().await?;
        let mut compliance_list = Vec::with_capacity(slos.len());

        for slo in slos {
            let measurements = self
                .repo
                .get_sli_measurements(&slo.sli_id, slo.window_days * 24 * 60)
                .await?;

            if measurements.is_empty() {
                continue;
            }

            let current_value =
                measurements.iter().map(|m| m.value).sum::<f64>() / measurements.len() as f64;

            let status = if current_value >= slo.target_value {
                SloStatus::Compliant
            } else if current_value >= slo.target_value - slo.error_budget {
                SloStatus::AtRisk
            } else {
                SloStatus::Breached
            };

            let error_budget_remaining = (slo.target_value - current_value).max(0.0);

            let compliance = SloCompliance {
                slo_id: slo.id.clone(),
                slo_name: slo.name.clone(),
                current_value,
                target_value: slo.target_value,
                status,
                error_budget_remaining,
                measured_at: chrono::Utc::now(),
            };

            self.repo.record_slo_compliance(compliance.clone()).await?;
            compliance_list.push(compliance);
        }

        Ok(compliance_list)
    }

    /// Get latest SLO compliance
    pub async fn get_slo_compliance(&self) -> Result<Vec<SloCompliance>, ApiError> {
        let cache_key = cache_key(0, "observability", "slo_compliance");
        if let Some(cached) = cache_get::<Vec<SloCompliance>>(&*self.cache, &cache_key).await? {
            return Ok(cached);
        }

        let compliance = self.repo.get_slo_compliance().await?;
        cache_set(
            &*self.cache,
            &cache_key,
            &compliance,
            Some(OBSERVABILITY_CACHE_TTL),
        )
        .await?;
        Ok(compliance)
    }

    // ── Alerting ────────────────────────────────────────────────────

    /// Create an alert rule
    pub async fn create_alert_rule(
        &self,
        name: String,
        metric: String,
        condition: String,
        threshold: f64,
        severity: AlertSeverity,
        duration_sec: i64,
    ) -> Result<AlertRule, ApiError> {
        let id_num = {
            let mut id_guard = self.next_rule_id.lock();
            let num = *id_guard;
            *id_guard += 1;
            num
        };

        let id = format!("rule-{}", id_num);
        let rule = AlertRule {
            id,
            name,
            metric,
            condition,
            threshold,
            severity,
            duration_sec,
            enabled: true,
            created_at: chrono::Utc::now(),
        };
        self.repo.create_alert_rule(rule).await
    }

    /// List all alert rules
    pub async fn list_alert_rules(&self) -> Result<Vec<AlertRule>, ApiError> {
        self.repo.list_alert_rules().await
    }

    /// Delete an alert rule
    pub async fn delete_alert_rule(&self, id: &str) -> Result<(), ApiError> {
        self.repo.delete_alert_rule(id).await
    }

    /// Evaluate alert rules and create alerts
    pub async fn evaluate_alert_rules(
        &self,
        metrics: &HashMap<String, f64>,
    ) -> Result<Vec<Alert>, ApiError> {
        let rules = self.repo.list_alert_rules().await?;
        let mut new_alerts = Vec::with_capacity(rules.len());

        for rule in rules {
            if !rule.enabled {
                continue;
            }

            let value = metrics.get(&rule.metric).copied().unwrap_or(0.0);
            let triggered = match rule.condition.as_str() {
                "gt" => value > rule.threshold,
                "gte" => value >= rule.threshold,
                "lt" => value < rule.threshold,
                "lte" => value <= rule.threshold,
                "eq" => (value - rule.threshold).abs() < f64::EPSILON,
                _ => false,
            };

            let should_fire = if triggered {
                if rule.duration_sec <= 0 {
                    true
                } else {
                    let mut tracker = self.alert_tracker.lock();
                    tracker
                        .record(&rule.id, &rule.metric, true, value, rule.duration_sec)
                        .is_some()
                }
            } else {
                // Condition cleared — reset tracker state
                if rule.duration_sec > 0 {
                    let mut tracker = self.alert_tracker.lock();
                    tracker.record(&rule.id, &rule.metric, false, value, rule.duration_sec);
                }
                false
            };

            if should_fire {
                let alert_id_num = {
                    let mut id_guard = self.next_alert_id.lock();
                    let num = *id_guard;
                    *id_guard += 1;
                    num
                };

                let alert = Alert {
                    id: format!("alert-{}", alert_id_num),
                    rule_id: rule.id.clone(),
                    rule_name: rule.name.clone(),
                    severity: rule.severity,
                    state: AlertState::Firing,
                    message: format!(
                        "{}: {} is {} (threshold: {}, duration: {}s)",
                        rule.severity, rule.metric, value, rule.threshold, rule.duration_sec
                    ),
                    value,
                    fired_at: chrono::Utc::now(),
                    resolved_at: None,
                };

                self.repo.create_alert(alert.clone()).await?;

                // Dispatch notification if service is available
                if let Some(ref notifier) = self.notification_service {
                    let priority = match alert.severity {
                        AlertSeverity::Critical => NotificationPriority::Urgent,
                        AlertSeverity::Warning => NotificationPriority::High,
                        AlertSeverity::Info => NotificationPriority::Normal,
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
                            "metric": rule.metric,
                            "value": alert.value,
                            "threshold": rule.threshold,
                            "message": alert.message,
                        }),
                        recipient: "admin".to_string(),
                    };
                    notifier.send(request).await.ok();
                }

                new_alerts.push(alert);
            }
        }

        Ok(new_alerts)
    }

    /// List recent alerts
    pub async fn list_alerts(&self, limit: i64) -> Result<Vec<Alert>, ApiError> {
        self.repo.list_alerts(limit).await
    }

    /// Resolve an alert
    pub async fn resolve_alert(&self, id: &str) -> Result<(), ApiError> {
        self.repo.resolve_alert(id).await
    }

    // ── Dashboard ─────────────────────────────────────────────────

    /// Get dashboard summary data
    pub async fn get_dashboard_summary(&self) -> Result<DashboardSummary, ApiError> {
        let slos = self.get_slo_compliance().await?;
        let alerts = self.list_alerts(10).await?;
        let history = self.get_health_history(5).await?;

        Ok(DashboardSummary {
            slo_compliance: slos,
            recent_alerts: alerts,
            health_history: history,
            generated_at: chrono::Utc::now(),
        })
    }

    /// Get sparkline data for a metric
    pub async fn get_sparkline(
        &self,
        metric: &str,
        minutes: i64,
    ) -> Result<Vec<SparklineDataPoint>, ApiError> {
        self.repo.get_sparkline(metric, minutes).await
    }
}

/// Dashboard summary response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DashboardSummary {
    pub slo_compliance: Vec<SloCompliance>,
    pub recent_alerts: Vec<Alert>,
    pub health_history: Vec<HealthCheckResult>,
    pub generated_at: chrono::DateTime<chrono::Utc>,
}
