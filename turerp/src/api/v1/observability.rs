//! Observability API endpoints (v1)

use actix_web::{web, HttpResponse};
use std::collections::HashMap;

use crate::domain::observability::model::{AlertSeverity, SliMetricType};
use crate::domain::observability::service::ObservabilityService;
use crate::error::ApiResult;
use crate::middleware::AdminUser;
use actix_web::ResponseError;

/// Create SLI request
#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct CreateSliRequest {
    pub name: String,
    pub metric_type: SliMetricType,
    pub source: String,
    pub window_minutes: i64,
}

/// Create SLO request
#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct CreateSloRequest {
    pub name: String,
    pub sli_id: String,
    pub target_value: f64,
    pub error_budget: f64,
    pub window_days: i64,
}

/// Create alert rule request
#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct CreateAlertRuleRequest {
    pub name: String,
    pub metric: String,
    pub condition: String,
    pub threshold: f64,
    pub severity: AlertSeverity,
    pub duration_sec: i64,
}

/// Evaluate alerts request
#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct EvaluateAlertsRequest {
    pub metrics: HashMap<String, f64>,
}

/// Health check endpoint
#[utoipa::path(
    get,
    path = "/api/v1/observability/health",
    tag = "Observability",
    responses(
        (status = 200, description = "System health summary", body = crate::domain::observability::model::SystemHealthSummary),
    ),
)]
pub async fn health(
    observability_service: web::Data<ObservabilityService>,
) -> ApiResult<HttpResponse> {
    match observability_service.get_liveness().await {
        Ok(summary) => Ok(HttpResponse::Ok().json(summary)),
        Err(e) => Ok(e.error_response()),
    }
}

/// Liveness probe
#[utoipa::path(
    get,
    path = "/api/v1/observability/health/live",
    tag = "Observability",
    responses(
        (status = 200, description = "Liveness status", body = crate::domain::observability::model::SystemHealthSummary),
    ),
)]
pub async fn health_live(
    observability_service: web::Data<ObservabilityService>,
) -> ApiResult<HttpResponse> {
    let summary = observability_service.get_liveness().await?;
    Ok(HttpResponse::Ok().json(summary))
}

/// Readiness probe
#[utoipa::path(
    get,
    path = "/api/v1/observability/health/ready",
    tag = "Observability",
    responses(
        (status = 200, description = "Readiness status", body = crate::domain::observability::model::SystemHealthSummary),
        (status = 503, description = "Not ready"),
    ),
)]
pub async fn health_ready(
    observability_service: web::Data<ObservabilityService>,
    app_state: web::Data<crate::app::AppState>,
) -> ApiResult<HttpResponse> {
    let pool = app_state
        .infra
        .db_pool
        .as_ref()
        .map(|p| p.get_ref().as_ref());
    match observability_service.run_health_check(pool).await {
        Ok(summary) => {
            if summary.overall == crate::domain::observability::model::HealthStatus::Healthy {
                Ok(HttpResponse::Ok().json(summary))
            } else {
                Ok(HttpResponse::ServiceUnavailable().json(summary))
            }
        }
        Err(e) => Ok(e.error_response()),
    }
}

/// List SLIs
#[utoipa::path(
    get,
    path = "/api/v1/observability/slis",
    tag = "Observability",
    responses(
        (status = 200, description = "List of SLIs", body = Vec<crate::domain::observability::model::SliDefinition>),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_slis(
    observability_service: web::Data<ObservabilityService>,
) -> ApiResult<HttpResponse> {
    match observability_service.list_slis().await {
        Ok(slis) => Ok(HttpResponse::Ok().json(slis)),
        Err(e) => Ok(e.error_response()),
    }
}

/// Create an SLI
#[utoipa::path(
    post,
    path = "/api/v1/observability/slis",
    tag = "Observability",
    request_body = CreateSliRequest,
    responses(
        (status = 201, description = "SLI created", body = crate::domain::observability::model::SliDefinition),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_sli(
    _admin: AdminUser,
    observability_service: web::Data<ObservabilityService>,
    payload: web::Json<CreateSliRequest>,
) -> ApiResult<HttpResponse> {
    let req = payload.into_inner();
    match observability_service
        .create_sli(req.name, req.metric_type, req.source, req.window_minutes)
        .await
    {
        Ok(sli) => Ok(HttpResponse::Created().json(sli)),
        Err(e) => Ok(e.error_response()),
    }
}

/// Record an SLI measurement
#[utoipa::path(
    post,
    path = "/api/v1/observability/slis/{sli_id}/measurements",
    tag = "Observability",
    params(("sli_id" = String, Path, description = "SLI ID")),
    request_body = f64,
    responses(
        (status = 204, description = "Measurement recorded"),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn record_sli(
    _admin: AdminUser,
    observability_service: web::Data<ObservabilityService>,
    path: web::Path<String>,
    payload: web::Json<f64>,
) -> ApiResult<HttpResponse> {
    match observability_service
        .record_sli(path.into_inner(), payload.into_inner())
        .await
    {
        Ok(()) => Ok(HttpResponse::NoContent().finish()),
        Err(e) => Ok(e.error_response()),
    }
}

/// List SLOs
#[utoipa::path(
    get,
    path = "/api/v1/observability/slos",
    tag = "Observability",
    responses(
        (status = 200, description = "List of SLOs", body = Vec<crate::domain::observability::model::SloDefinition>),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_slos(
    observability_service: web::Data<ObservabilityService>,
) -> ApiResult<HttpResponse> {
    match observability_service.list_slos().await {
        Ok(slos) => Ok(HttpResponse::Ok().json(slos)),
        Err(e) => Ok(e.error_response()),
    }
}

/// Create an SLO
#[utoipa::path(
    post,
    path = "/api/v1/observability/slos",
    tag = "Observability",
    request_body = CreateSloRequest,
    responses(
        (status = 201, description = "SLO created", body = crate::domain::observability::model::SloDefinition),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_slo(
    _admin: AdminUser,
    observability_service: web::Data<ObservabilityService>,
    payload: web::Json<CreateSloRequest>,
) -> ApiResult<HttpResponse> {
    let req = payload.into_inner();
    match observability_service
        .create_slo(
            req.name,
            req.sli_id,
            req.target_value,
            req.error_budget,
            req.window_days,
        )
        .await
    {
        Ok(slo) => Ok(HttpResponse::Created().json(slo)),
        Err(e) => Ok(e.error_response()),
    }
}

/// Get SLO compliance
#[utoipa::path(
    get,
    path = "/api/v1/observability/slos/compliance",
    tag = "Observability",
    responses(
        (status = 200, description = "SLO compliance", body = Vec<crate::domain::observability::model::SloCompliance>),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_slo_compliance(
    observability_service: web::Data<ObservabilityService>,
) -> ApiResult<HttpResponse> {
    match observability_service.get_slo_compliance().await {
        Ok(compliance) => Ok(HttpResponse::Ok().json(compliance)),
        Err(e) => Ok(e.error_response()),
    }
}

/// Evaluate SLO compliance
#[utoipa::path(
    post,
    path = "/api/v1/observability/slos/compliance/evaluate",
    tag = "Observability",
    responses(
        (status = 200, description = "SLO compliance evaluated", body = Vec<crate::domain::observability::model::SloCompliance>),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn evaluate_slo_compliance(
    _admin: AdminUser,
    observability_service: web::Data<ObservabilityService>,
) -> ApiResult<HttpResponse> {
    match observability_service.evaluate_slo_compliance().await {
        Ok(compliance) => Ok(HttpResponse::Ok().json(compliance)),
        Err(e) => Ok(e.error_response()),
    }
}

/// List alert rules
#[utoipa::path(
    get,
    path = "/api/v1/observability/alert-rules",
    tag = "Observability",
    responses(
        (status = 200, description = "List of alert rules", body = Vec<crate::domain::observability::model::AlertRule>),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_alert_rules(
    observability_service: web::Data<ObservabilityService>,
) -> ApiResult<HttpResponse> {
    match observability_service.list_alert_rules().await {
        Ok(rules) => Ok(HttpResponse::Ok().json(rules)),
        Err(e) => Ok(e.error_response()),
    }
}

/// Create an alert rule
#[utoipa::path(
    post,
    path = "/api/v1/observability/alert-rules",
    tag = "Observability",
    request_body = CreateAlertRuleRequest,
    responses(
        (status = 201, description = "Alert rule created", body = crate::domain::observability::model::AlertRule),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_alert_rule(
    _admin: AdminUser,
    observability_service: web::Data<ObservabilityService>,
    payload: web::Json<CreateAlertRuleRequest>,
) -> ApiResult<HttpResponse> {
    let req = payload.into_inner();
    match observability_service
        .create_alert_rule(
            req.name,
            req.metric,
            req.condition,
            req.threshold,
            req.severity,
            req.duration_sec,
        )
        .await
    {
        Ok(rule) => Ok(HttpResponse::Created().json(rule)),
        Err(e) => Ok(e.error_response()),
    }
}

/// Delete an alert rule
#[utoipa::path(
    delete,
    path = "/api/v1/observability/alert-rules/{id}",
    tag = "Observability",
    params(("id" = String, Path, description = "Alert rule ID")),
    responses(
        (status = 204, description = "Alert rule deleted"),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_alert_rule(
    _admin: AdminUser,
    observability_service: web::Data<ObservabilityService>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    match observability_service
        .delete_alert_rule(&path.into_inner())
        .await
    {
        Ok(()) => Ok(HttpResponse::NoContent().finish()),
        Err(e) => Ok(e.error_response()),
    }
}

/// Evaluate alert rules
#[utoipa::path(
    post,
    path = "/api/v1/observability/alerts/evaluate",
    tag = "Observability",
    request_body = EvaluateAlertsRequest,
    responses(
        (status = 200, description = "Alerts evaluated", body = Vec<crate::domain::observability::model::Alert>),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn evaluate_alerts(
    _admin: AdminUser,
    observability_service: web::Data<ObservabilityService>,
    payload: web::Json<EvaluateAlertsRequest>,
) -> ApiResult<HttpResponse> {
    match observability_service
        .evaluate_alert_rules(&payload.into_inner().metrics)
        .await
    {
        Ok(alerts) => Ok(HttpResponse::Ok().json(alerts)),
        Err(e) => Ok(e.error_response()),
    }
}

/// List recent alerts
#[utoipa::path(
    get,
    path = "/api/v1/observability/alerts",
    tag = "Observability",
    params(("limit" = i64, Query, description = "Max alerts to return")),
    responses(
        (status = 200, description = "List of alerts", body = Vec<crate::domain::observability::model::Alert>),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_alerts(
    observability_service: web::Data<ObservabilityService>,
    query: web::Query<HashMap<String, String>>,
) -> ApiResult<HttpResponse> {
    let limit = query
        .get("limit")
        .and_then(|v| v.parse().ok())
        .unwrap_or(50i64);
    match observability_service.list_alerts(limit).await {
        Ok(alerts) => Ok(HttpResponse::Ok().json(alerts)),
        Err(e) => Ok(e.error_response()),
    }
}

/// Resolve an alert
#[utoipa::path(
    post,
    path = "/api/v1/observability/alerts/{id}/resolve",
    tag = "Observability",
    params(("id" = String, Path, description = "Alert ID")),
    responses(
        (status = 204, description = "Alert resolved"),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn resolve_alert(
    _admin: AdminUser,
    observability_service: web::Data<ObservabilityService>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    match observability_service
        .resolve_alert(&path.into_inner())
        .await
    {
        Ok(()) => Ok(HttpResponse::NoContent().finish()),
        Err(e) => Ok(e.error_response()),
    }
}

/// Get observability dashboard summary
#[utoipa::path(
    get,
    path = "/api/v1/observability/dashboard",
    tag = "Observability",
    responses(
        (status = 200, description = "Dashboard summary", body = crate::domain::observability::service::DashboardSummary),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_dashboard(
    observability_service: web::Data<ObservabilityService>,
) -> ApiResult<HttpResponse> {
    match observability_service.get_dashboard_summary().await {
        Ok(summary) => Ok(HttpResponse::Ok().json(summary)),
        Err(e) => Ok(e.error_response()),
    }
}

/// Get sparkline data for a metric
#[utoipa::path(
    get,
    path = "/api/v1/observability/sparklines/{metric}",
    tag = "Observability",
    params(
        ("metric" = String, Path, description = "Metric name"),
        ("minutes" = i64, Query, description = "Time window in minutes")
    ),
    responses(
        (status = 200, description = "Sparkline data", body = Vec<crate::domain::observability::model::SparklineDataPoint>),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_sparkline(
    observability_service: web::Data<ObservabilityService>,
    path: web::Path<String>,
    query: web::Query<HashMap<String, String>>,
) -> ApiResult<HttpResponse> {
    let minutes = query
        .get("minutes")
        .and_then(|v| v.parse().ok())
        .unwrap_or(60i64);
    match observability_service
        .get_sparkline(&path.into_inner(), minutes)
        .await
    {
        Ok(points) => Ok(HttpResponse::Ok().json(points)),
        Err(e) => Ok(e.error_response()),
    }
}

/// Aspire Dashboard URL response.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct AspireDashboardUrlResponse {
    pub url: String,
}

/// Get the Aspire Dashboard URL.
///
/// Returns the URL of the Aspire Dashboard where traces, metrics, and logs
/// can be viewed in real time.
#[utoipa::path(
    get,
    path = "/api/v1/observability/dashboard-url",
    tag = "Observability",
    responses(
        (status = 200, description = "Aspire Dashboard URL", body = AspireDashboardUrlResponse),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_aspire_dashboard_url(_admin: AdminUser) -> ApiResult<HttpResponse> {
    let url = std::env::var("ASPIRE_DASHBOARD_URL")
        .unwrap_or_else(|_| "http://localhost:18888".to_string());
    Ok(HttpResponse::Ok().json(AspireDashboardUrlResponse { url }))
}

/// Percentiles response for current metric snapshots.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct PercentilesResponse {
    pub p95: f64,
    pub p99: f64,
    pub computed_at: chrono::DateTime<chrono::Utc>,
}

/// Get current P95/P99 percentiles for HTTP request duration.
#[utoipa::path(
    get,
    path = "/api/v1/observability/metrics/percentiles",
    tag = "Observability",
    responses(
        (status = 200, description = "Current P95/P99 percentiles", body = PercentilesResponse),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_percentiles() -> ApiResult<HttpResponse> {
    let metrics_text = crate::middleware::metrics::render_metrics();
    let percentiles = crate::common::prometheus_percentile::compute_percentiles(&metrics_text);

    // Find the first http_request_duration_seconds P95/P99
    let mut p95 = 0.0;
    let mut p99 = 0.0;

    for (key, value) in &percentiles {
        if key.starts_with("http_request_duration_seconds") && key.contains("quantile=p95") {
            p95 = *value;
        }
        if key.starts_with("http_request_duration_seconds") && key.contains("quantile=p99") {
            p99 = *value;
        }
    }

    Ok(HttpResponse::Ok().json(PercentilesResponse {
        p95,
        p99,
        computed_at: chrono::Utc::now(),
    }))
}

/// Configure observability routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/v1/observability/health").route(web::get().to(health)))
        .service(web::resource("/v1/observability/health/live").route(web::get().to(health_live)))
        .service(web::resource("/v1/observability/health/ready").route(web::get().to(health_ready)))
        .service(
            web::resource("/v1/observability/slis")
                .route(web::get().to(list_slis))
                .route(web::post().to(create_sli)),
        )
        .service(
            web::resource("/v1/observability/slis/{sli_id}/measurements")
                .route(web::post().to(record_sli)),
        )
        .service(
            web::resource("/v1/observability/slos")
                .route(web::get().to(list_slos))
                .route(web::post().to(create_slo)),
        )
        .service(
            web::resource("/v1/observability/slos/compliance")
                .route(web::get().to(get_slo_compliance)),
        )
        .service(
            web::resource("/v1/observability/slos/compliance/evaluate")
                .route(web::post().to(evaluate_slo_compliance)),
        )
        .service(
            web::resource("/v1/observability/alert-rules")
                .route(web::get().to(list_alert_rules))
                .route(web::post().to(create_alert_rule)),
        )
        .service(
            web::resource("/v1/observability/alert-rules/{id}")
                .route(web::delete().to(delete_alert_rule)),
        )
        .service(web::resource("/v1/observability/alerts").route(web::get().to(list_alerts)))
        .service(
            web::resource("/v1/observability/alerts/evaluate")
                .route(web::post().to(evaluate_alerts)),
        )
        .service(
            web::resource("/v1/observability/alerts/{id}/resolve")
                .route(web::post().to(resolve_alert)),
        )
        .service(web::resource("/v1/observability/dashboard").route(web::get().to(get_dashboard)))
        .service(
            web::resource("/v1/observability/sparklines/{metric}")
                .route(web::get().to(get_sparkline)),
        )
        .service(
            web::resource("/v1/observability/dashboard-url")
                .route(web::get().to(get_aspire_dashboard_url)),
        )
        .service(
            web::resource("/v1/observability/metrics/percentiles")
                .route(web::get().to(get_percentiles)),
        );
}
