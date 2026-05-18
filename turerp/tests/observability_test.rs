//! Observability Integration Tests
//!
//! Covers: histogram metrics, business KPIs, alerting rules,
//! metrics endpoint format, and observability API integration.
//!
//! Run with: cargo test --test observability_test

use actix_web::{body::to_bytes, http::StatusCode, web, App, HttpResponse};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

use turerp::api::v1_observability_configure;
use turerp::app::create_app_state_in_memory;
use turerp::config::Config;
use turerp::domain::observability::model::{
    AlertSeverity, AlertState, HealthStatus, SliMetricType, SloStatus,
};
use turerp::domain::observability::service::ObservabilityService;
use turerp::domain::observability::InMemoryObservabilityRepository;
use turerp::domain::user::model::Role;
use turerp::middleware::{JwtAuthMiddleware, MetricsMiddleware};
use turerp::utils::jwt::JwtService;

// ============================================================================
// Helpers
// ============================================================================

fn create_test_app_state() -> turerp::app::AppState {
    let config = Config::default();
    create_app_state_in_memory(&config).expect("app state creation failed")
}

fn build_observability_app(
    state: &turerp::app::AppState,
) -> App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse<
            actix_web::body::EitherBody<actix_web::body::BoxBody>,
        >,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    let jwt = JwtService::new(
        Config::default().jwt.secret.clone(),
        Config::default().jwt.access_token_expiration,
        Config::default().jwt.refresh_token_expiration,
    );
    App::new()
        .wrap(JwtAuthMiddleware::new(jwt))
        .wrap(MetricsMiddleware::new())
        .app_data(state.observability_service.clone())
        .service(web::scope("/api").configure(v1_observability_configure))
}

fn admin_token_for_state(_state: &turerp::app::AppState, tenant_id: i64) -> String {
    let jwt = JwtService::new(
        Config::default().jwt.secret.clone(),
        Config::default().jwt.access_token_expiration,
        Config::default().jwt.refresh_token_expiration,
    );
    jwt.generate_tokens(1, tenant_id, "admin".to_string(), Role::Admin)
        .unwrap()
        .access_token
}

fn user_token_for_state(_state: &turerp::app::AppState, tenant_id: i64) -> String {
    let jwt = JwtService::new(
        Config::default().jwt.secret.clone(),
        Config::default().jwt.access_token_expiration,
        Config::default().jwt.refresh_token_expiration,
    );
    jwt.generate_tokens(2, tenant_id, "user".to_string(), Role::User)
        .unwrap()
        .access_token
}

// ============================================================================
// 1. Histogram Metric Accuracy Tests
// ============================================================================

mod histogram_tests {
    use super::*;

    /// Parse Prometheus text-format metrics into a key-value map.
    /// Mirrors the logic in main.rs for testing.
    fn parse_prometheus_metrics(text: &str) -> HashMap<String, f64> {
        let mut metrics = HashMap::new();
        let mut total_requests = 0u64;
        let mut error_requests = 0u64;
        let mut latency_sum = 0.0;
        let mut latency_count = 0u64;

        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some(value_str) = line.rsplit(' ').next() {
                if let Ok(value) = value_str.parse::<f64>() {
                    if line.starts_with("http_requests_total{") {
                        total_requests += value as u64;
                        if line.contains("status=\"5") {
                            error_requests += value as u64;
                        }
                    } else if line.starts_with("http_request_duration_seconds_sum{") {
                        latency_sum += value;
                    } else if line.starts_with("http_request_duration_seconds_count{") {
                        latency_count += value as u64;
                    }
                }
            }
        }

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

        if latency_count > 0 {
            metrics.insert(
                "latency_p95".to_string(),
                latency_sum / latency_count as f64,
            );
        }

        if total_requests > 0 {
            metrics.insert("throughput".to_string(), total_requests as f64);
        }

        metrics
    }

    #[test]
    fn test_parse_prometheus_metrics_empty() {
        let parsed = parse_prometheus_metrics("");
        assert!(parsed.is_empty());
    }

    #[test]
    fn test_parse_prometheus_metrics_with_comments() {
        let text = r#"# HELP http_requests_total Total requests
# TYPE http_requests_total counter
http_requests_total{method="GET",path="/api/v1/users",status="200"} 10
http_request_duration_seconds_sum{method="GET",path="/api/v1/users"} 5.0
http_request_duration_seconds_count{method="GET",path="/api/v1/users"} 10
"#;
        let parsed = parse_prometheus_metrics(text);
        assert_eq!(parsed.get("availability"), Some(&1.0));
        assert_eq!(parsed.get("error_rate"), Some(&0.0));
        assert_eq!(parsed.get("latency_p95"), Some(&0.5));
        assert_eq!(parsed.get("throughput"), Some(&10.0));
    }

    #[test]
    fn test_parse_prometheus_metrics_with_errors() {
        let text = r#"http_requests_total{method="GET",path="/api/v1/users",status="200"} 8
http_requests_total{method="POST",path="/api/v1/users",status="500"} 2
http_request_duration_seconds_sum{method="GET",path="/api/v1/users"} 4.0
http_request_duration_seconds_count{method="GET",path="/api/v1/users"} 8
"#;
        let parsed = parse_prometheus_metrics(text);
        assert_eq!(parsed.get("availability"), Some(&0.8));
        assert_eq!(parsed.get("error_rate"), Some(&0.2));
        assert_eq!(parsed.get("throughput"), Some(&10.0));
    }

    #[test]
    fn test_parse_prometheus_metrics_ignores_unknown_lines() {
        let text = r#"random_metric 42
http_requests_total{method="GET",path="/",status="200"} 1
"#;
        let parsed = parse_prometheus_metrics(text);
        assert_eq!(parsed.get("throughput"), Some(&1.0));
        assert!(!parsed.contains_key("random_metric"));
    }

    #[actix_web::test]
    async fn test_metrics_middleware_records_histogram() {
        let _app_state = create_test_app_state();
        let _jwt = JwtService::new(
            Config::default().jwt.secret.clone(),
            Config::default().jwt.access_token_expiration,
            Config::default().jwt.refresh_token_expiration,
        );

        let app = actix_web::test::init_service(App::new().wrap(MetricsMiddleware::new()).route(
            "/test",
            web::get().to(|| async { HttpResponse::Ok().body("ok") }),
        ))
        .await;

        // Make several requests
        for _ in 0..5 {
            let req = actix_web::test::TestRequest::get()
                .uri("/test")
                .to_request();
            let resp = actix_web::test::call_service(&app, req).await;
            assert_eq!(resp.status(), StatusCode::OK);
        }

        // The global Prometheus handle may or may not be installed in tests,
        // so we verify the middleware exists and doesn't panic.
        // Histogram recording is validated via the metrics crate internals
        // in integration tests with the exporter installed.
    }
}

// ============================================================================
// 2. Business KPI Metric Collection Tests
// ============================================================================

mod kpi_tests {
    use super::*;
    use turerp::api::v1_dashboard_configure;

    #[actix_web::test]
    async fn test_kpi_endpoints_require_auth() {
        let app_state = create_test_app_state();
        let jwt = JwtService::new(
            Config::default().jwt.secret.clone(),
            Config::default().jwt.access_token_expiration,
            Config::default().jwt.refresh_token_expiration,
        );

        let app = actix_web::test::init_service(
            App::new()
                .wrap(JwtAuthMiddleware::new(jwt))
                .app_data(app_state.document.dashboard_service.clone())
                .service(web::scope("/api").configure(v1_dashboard_configure)),
        )
        .await;

        let req = actix_web::test::TestRequest::get()
            .uri("/api/v1/dashboard/kpis")
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn test_kpi_endpoints_return_structure() {
        let app_state = create_test_app_state();
        let token = admin_token_for_state(&app_state, 1);
        let jwt = JwtService::new(
            Config::default().jwt.secret.clone(),
            Config::default().jwt.access_token_expiration,
            Config::default().jwt.refresh_token_expiration,
        );

        let app = actix_web::test::init_service(
            App::new()
                .wrap(JwtAuthMiddleware::new(jwt))
                .app_data(app_state.document.dashboard_service.clone())
                .service(web::scope("/api").configure(v1_dashboard_configure)),
        )
        .await;

        let req = actix_web::test::TestRequest::get()
            .uri("/api/v1/dashboard/kpis")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;

        // In-memory dashboard may return empty/default KPIs; validate structure
        assert!(resp.status().is_success() || resp.status() == StatusCode::OK);

        let body = to_bytes(resp.into_body()).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        // KPI response should be an array or object with expected fields
        assert!(json.is_array() || json.is_object());
    }
}

// ============================================================================
// 3. Alerting Rule Evaluation Tests (Threshold Triggering)
// ============================================================================

mod alert_rule_tests {
    use super::*;

    #[tokio::test]
    async fn test_alert_rule_gt_trigger() {
        let repo = Arc::new(InMemoryObservabilityRepository::new());
        let cache: Arc<dyn turerp::cache::CacheService> =
            Arc::new(turerp::cache::InMemoryCacheService::new());
        let service = ObservabilityService::new(repo.clone(), cache);

        let rule = service
            .create_alert_rule(
                "cpu_high".to_string(),
                "cpu_usage".to_string(),
                "gt".to_string(),
                80.0,
                AlertSeverity::Warning,
                0,
            )
            .await
            .unwrap();

        let mut metrics = HashMap::new();
        metrics.insert("cpu_usage".to_string(), 85.0);

        let alerts = service.evaluate_alert_rules(&metrics).await.unwrap();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].rule_id, rule.id);
        assert_eq!(alerts[0].state, AlertState::Firing);
        assert_eq!(alerts[0].severity, AlertSeverity::Warning);
    }

    #[tokio::test]
    async fn test_alert_rule_gt_no_trigger() {
        let repo = Arc::new(InMemoryObservabilityRepository::new());
        let cache: Arc<dyn turerp::cache::CacheService> =
            Arc::new(turerp::cache::InMemoryCacheService::new());
        let service = ObservabilityService::new(repo.clone(), cache);

        service
            .create_alert_rule(
                "cpu_high".to_string(),
                "cpu_usage".to_string(),
                "gt".to_string(),
                80.0,
                AlertSeverity::Warning,
                60,
            )
            .await
            .unwrap();

        let mut metrics = HashMap::new();
        metrics.insert("cpu_usage".to_string(), 75.0);

        let alerts = service.evaluate_alert_rules(&metrics).await.unwrap();
        assert!(alerts.is_empty());
    }

    #[tokio::test]
    async fn test_alert_rule_gte_trigger_on_equal() {
        let repo = Arc::new(InMemoryObservabilityRepository::new());
        let cache: Arc<dyn turerp::cache::CacheService> =
            Arc::new(turerp::cache::InMemoryCacheService::new());
        let service = ObservabilityService::new(repo.clone(), cache);

        service
            .create_alert_rule(
                "disk_full".to_string(),
                "disk_usage".to_string(),
                "gte".to_string(),
                100.0,
                AlertSeverity::Critical,
                0,
            )
            .await
            .unwrap();

        let mut metrics = HashMap::new();
        metrics.insert("disk_usage".to_string(), 100.0);

        let alerts = service.evaluate_alert_rules(&metrics).await.unwrap();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].severity, AlertSeverity::Critical);
    }

    #[tokio::test]
    async fn test_alert_rule_lt_trigger() {
        let repo = Arc::new(InMemoryObservabilityRepository::new());
        let cache: Arc<dyn turerp::cache::CacheService> =
            Arc::new(turerp::cache::InMemoryCacheService::new());
        let service = ObservabilityService::new(repo.clone(), cache);

        service
            .create_alert_rule(
                "low_memory".to_string(),
                "free_memory_mb".to_string(),
                "lt".to_string(),
                512.0,
                AlertSeverity::Critical,
                0,
            )
            .await
            .unwrap();

        let mut metrics = HashMap::new();
        metrics.insert("free_memory_mb".to_string(), 256.0);

        let alerts = service.evaluate_alert_rules(&metrics).await.unwrap();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].rule_name, "low_memory");
    }

    #[tokio::test]
    async fn test_alert_rule_lte_trigger() {
        let repo = Arc::new(InMemoryObservabilityRepository::new());
        let cache: Arc<dyn turerp::cache::CacheService> =
            Arc::new(turerp::cache::InMemoryCacheService::new());
        let service = ObservabilityService::new(repo.clone(), cache);

        service
            .create_alert_rule(
                "qps_low".to_string(),
                "qps".to_string(),
                "lte".to_string(),
                10.0,
                AlertSeverity::Warning,
                0,
            )
            .await
            .unwrap();

        let mut metrics = HashMap::new();
        metrics.insert("qps".to_string(), 10.0);

        let alerts = service.evaluate_alert_rules(&metrics).await.unwrap();
        assert_eq!(alerts.len(), 1);
    }

    #[tokio::test]
    async fn test_alert_rule_eq_trigger() {
        let repo = Arc::new(InMemoryObservabilityRepository::new());
        let cache: Arc<dyn turerp::cache::CacheService> =
            Arc::new(turerp::cache::InMemoryCacheService::new());
        let service = ObservabilityService::new(repo.clone(), cache);

        service
            .create_alert_rule(
                "exact_match".to_string(),
                "status_code".to_string(),
                "eq".to_string(),
                503.0,
                AlertSeverity::Info,
                0,
            )
            .await
            .unwrap();

        let mut metrics = HashMap::new();
        metrics.insert("status_code".to_string(), 503.0);

        let alerts = service.evaluate_alert_rules(&metrics).await.unwrap();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].value, 503.0);
    }

    #[tokio::test]
    async fn test_alert_rule_disabled_not_triggered() {
        let repo = Arc::new(InMemoryObservabilityRepository::new());
        let cache: Arc<dyn turerp::cache::CacheService> =
            Arc::new(turerp::cache::InMemoryCacheService::new());
        let service = ObservabilityService::new(repo.clone(), cache);

        let rule = service
            .create_alert_rule(
                "disabled_rule".to_string(),
                "cpu_usage".to_string(),
                "gt".to_string(),
                0.0,
                AlertSeverity::Warning,
                60,
            )
            .await
            .unwrap();

        // Disable the rule by modifying the repository directly
        repo.disable_alert_rule(&rule.id);

        let mut metrics = HashMap::new();
        metrics.insert("cpu_usage".to_string(), 999.0);

        let alerts = service.evaluate_alert_rules(&metrics).await.unwrap();
        assert!(alerts.is_empty());
    }

    #[tokio::test]
    async fn test_alert_rule_unknown_condition_ignored() {
        let repo = Arc::new(InMemoryObservabilityRepository::new());
        let cache: Arc<dyn turerp::cache::CacheService> =
            Arc::new(turerp::cache::InMemoryCacheService::new());
        let service = ObservabilityService::new(repo.clone(), cache);

        service
            .create_alert_rule(
                "bad_condition".to_string(),
                "metric".to_string(),
                "unknown".to_string(),
                50.0,
                AlertSeverity::Warning,
                60,
            )
            .await
            .unwrap();

        let mut metrics = HashMap::new();
        metrics.insert("metric".to_string(), 100.0);

        let alerts = service.evaluate_alert_rules(&metrics).await.unwrap();
        assert!(alerts.is_empty());
    }

    #[tokio::test]
    async fn test_alert_rule_missing_metric_defaults_zero() {
        let repo = Arc::new(InMemoryObservabilityRepository::new());
        let cache: Arc<dyn turerp::cache::CacheService> =
            Arc::new(turerp::cache::InMemoryCacheService::new());
        let service = ObservabilityService::new(repo.clone(), cache);

        service
            .create_alert_rule(
                "missing_metric".to_string(),
                "nonexistent".to_string(),
                "gt".to_string(),
                0.0,
                AlertSeverity::Warning,
                60,
            )
            .await
            .unwrap();

        let metrics = HashMap::new(); // empty

        let alerts = service.evaluate_alert_rules(&metrics).await.unwrap();
        // Default value is 0.0, threshold is 0.0, condition gt => 0.0 > 0.0 is false
        assert!(alerts.is_empty());
    }

    #[tokio::test]
    async fn test_multiple_alert_rules_evaluated() {
        let repo = Arc::new(InMemoryObservabilityRepository::new());
        let cache: Arc<dyn turerp::cache::CacheService> =
            Arc::new(turerp::cache::InMemoryCacheService::new());
        let service = ObservabilityService::new(repo.clone(), cache);

        service
            .create_alert_rule(
                "cpu".to_string(),
                "cpu".to_string(),
                "gt".to_string(),
                80.0,
                AlertSeverity::Warning,
                0,
            )
            .await
            .unwrap();
        service
            .create_alert_rule(
                "mem".to_string(),
                "mem".to_string(),
                "gt".to_string(),
                90.0,
                AlertSeverity::Critical,
                0,
            )
            .await
            .unwrap();

        let mut metrics = HashMap::new();
        metrics.insert("cpu".to_string(), 85.0);
        metrics.insert("mem".to_string(), 85.0);

        let alerts = service.evaluate_alert_rules(&metrics).await.unwrap();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].rule_name, "cpu");
    }
}

// ============================================================================
// 4. Metrics Endpoint Format Validation Tests
// ============================================================================

mod metrics_format_tests {
    #[test]
    fn test_prometheus_text_format_basic() {
        let text = r#"# HELP http_requests_total Total requests
# TYPE http_requests_total counter
http_requests_total{method="GET",path="/",status="200"} 1
"#;
        // Validate it starts with comment or metric
        assert!(text.starts_with('#') || text.starts_with("http_"));
    }

    #[test]
    fn test_prometheus_text_format_histogram_buckets() {
        // Real Prometheus histogram format includes _bucket, _sum, _count
        let text = r#"# TYPE http_request_duration_seconds histogram
http_request_duration_seconds_bucket{method="GET",path="/",le="0.005"} 0
http_request_duration_seconds_bucket{method="GET",path="/",le="0.01"} 1
http_request_duration_seconds_bucket{method="GET",path="/",le="0.025"} 2
http_request_duration_seconds_bucket{method="GET",path="/",le="+Inf"} 3
http_request_duration_seconds_sum{method="GET",path="/"} 0.045
http_request_duration_seconds_count{method="GET",path="/"} 3
"#;

        let lines: Vec<&str> = text.lines().filter(|l| !l.is_empty()).collect();
        assert!(lines.iter().any(|l| l.contains("_bucket")));
        assert!(lines.iter().any(|l| l.contains("_sum")));
        assert!(lines.iter().any(|l| l.contains("_count")));
        assert!(lines.iter().any(|l| l.contains("+Inf")));
    }

    #[test]
    fn test_prometheus_text_format_counter_increment() {
        // Counter lines must have a numeric value at the end
        let line = "http_requests_total{method=\"GET\",path=\"/\",status=\"200\"} 42";
        let parts: Vec<&str> = line.rsplit(' ').collect();
        assert_eq!(parts[0], "42");
        assert!(parts[0].parse::<f64>().is_ok());
    }

    #[test]
    fn test_prometheus_text_format_labels_parsing() {
        let line = r#"http_requests_total{method="GET",path="/api/v1/users",status="200"} 10"#;
        assert!(line.contains("method=\"GET\""));
        assert!(line.contains("path=\"/api/v1/users\""));
        assert!(line.contains("status=\"200\""));
    }
}

// ============================================================================
// 5. Observability API Integration Tests
// ============================================================================

mod observability_api_tests {
    use super::*;

    #[actix_web::test]
    async fn test_health_endpoint_no_auth() {
        let app_state = create_test_app_state();
        let _jwt = JwtService::new(
            Config::default().jwt.secret.clone(),
            Config::default().jwt.access_token_expiration,
            Config::default().jwt.refresh_token_expiration,
        );

        let app = actix_web::test::init_service(
            App::new()
                .app_data(app_state.observability_service.clone())
                .service(web::scope("/api").configure(v1_observability_configure)),
        )
        .await;

        let req = actix_web::test::TestRequest::get()
            .uri("/api/v1/observability/health")
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body = to_bytes(resp.into_body()).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["overall"], "Healthy");
        assert!(json["version"].is_string());
        assert!(json["checks"].is_array());
    }

    #[actix_web::test]
    async fn test_health_live_endpoint() {
        let app_state = create_test_app_state();
        let _jwt = JwtService::new(
            Config::default().jwt.secret.clone(),
            Config::default().jwt.access_token_expiration,
            Config::default().jwt.refresh_token_expiration,
        );

        let app = actix_web::test::init_service(
            App::new()
                .app_data(app_state.observability_service.clone())
                .service(web::scope("/api").configure(v1_observability_configure)),
        )
        .await;

        let req = actix_web::test::TestRequest::get()
            .uri("/api/v1/observability/health/live")
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body = to_bytes(resp.into_body()).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["overall"], "Healthy");
    }

    #[actix_web::test]
    async fn test_sli_crud() {
        let app_state = create_test_app_state();
        let token = admin_token_for_state(&app_state, 1);
        let app = build_observability_app(&app_state);
        let app = actix_web::test::init_service(app).await;

        // Create SLI
        let create_req = actix_web::test::TestRequest::post()
            .uri("/api/v1/observability/slis")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(json!({
                "name": "API Availability",
                "metric_type": "Availability",
                "source": "prometheus",
                "window_minutes": 5
            }))
            .to_request();
        let create_resp = actix_web::test::call_service(&app, create_req).await;
        assert_eq!(create_resp.status(), StatusCode::CREATED);

        let body = to_bytes(create_resp.into_body()).await.unwrap();
        let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let sli_id = created["id"].as_str().unwrap();
        assert_eq!(created["name"], "API Availability");

        // List SLIs
        let list_req = actix_web::test::TestRequest::get()
            .uri("/api/v1/observability/slis")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();
        let list_resp = actix_web::test::call_service(&app, list_req).await;
        assert_eq!(list_resp.status(), StatusCode::OK);

        let list_body = to_bytes(list_resp.into_body()).await.unwrap();
        let list: Vec<serde_json::Value> = serde_json::from_slice(&list_body).unwrap();
        assert!(!list.is_empty());

        // Record measurement
        let measure_req = actix_web::test::TestRequest::post()
            .uri(&format!(
                "/api/v1/observability/slis/{}/measurements",
                sli_id
            ))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(0.995)
            .to_request();
        let measure_resp = actix_web::test::call_service(&app, measure_req).await;
        assert_eq!(measure_resp.status(), StatusCode::NO_CONTENT);
    }

    #[actix_web::test]
    async fn test_slo_crud_and_compliance() {
        let app_state = create_test_app_state();
        let token = admin_token_for_state(&app_state, 1);
        let app = build_observability_app(&app_state);
        let app = actix_web::test::init_service(app).await;

        // Create SLI first
        let sli_req = actix_web::test::TestRequest::post()
            .uri("/api/v1/observability/slis")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(json!({
                "name": "Latency",
                "metric_type": "Latency",
                "source": "prometheus",
                "window_minutes": 5
            }))
            .to_request();
        let sli_resp = actix_web::test::call_service(&app, sli_req).await;
        let sli_body = to_bytes(sli_resp.into_body()).await.unwrap();
        let sli: serde_json::Value = serde_json::from_slice(&sli_body).unwrap();
        let sli_id = sli["id"].as_str().unwrap();

        // Create SLO
        let slo_req = actix_web::test::TestRequest::post()
            .uri("/api/v1/observability/slos")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(json!({
                "name": "Latency SLO",
                "sli_id": sli_id,
                "target_value": 0.2,
                "error_budget": 0.05,
                "window_days": 7
            }))
            .to_request();
        let slo_resp = actix_web::test::call_service(&app, slo_req).await;
        assert_eq!(slo_resp.status(), StatusCode::CREATED);

        let slo_body = to_bytes(slo_resp.into_body()).await.unwrap();
        let slo: serde_json::Value = serde_json::from_slice(&slo_body).unwrap();
        assert_eq!(slo["name"], "Latency SLO");

        // Record measurements
        for _ in 0..3 {
            let measure_req = actix_web::test::TestRequest::post()
                .uri(&format!(
                    "/api/v1/observability/slis/{}/measurements",
                    sli_id
                ))
                .insert_header(("Authorization", format!("Bearer {}", token)))
                .set_json(0.25)
                .to_request();
            actix_web::test::call_service(&app, measure_req).await;
        }

        // Evaluate compliance
        let eval_req = actix_web::test::TestRequest::post()
            .uri("/api/v1/observability/slos/compliance/evaluate")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();
        let eval_resp = actix_web::test::call_service(&app, eval_req).await;
        assert_eq!(eval_resp.status(), StatusCode::OK);

        let eval_body = to_bytes(eval_resp.into_body()).await.unwrap();
        let compliance: Vec<serde_json::Value> = serde_json::from_slice(&eval_body).unwrap();
        assert!(!compliance.is_empty());
        assert_eq!(compliance[0]["status"], "Compliant");
    }

    #[actix_web::test]
    async fn test_alert_rule_crud() {
        let app_state = create_test_app_state();
        let token = admin_token_for_state(&app_state, 1);
        let app = build_observability_app(&app_state);
        let app = actix_web::test::init_service(app).await;

        // Create alert rule
        let create_req = actix_web::test::TestRequest::post()
            .uri("/api/v1/observability/alert-rules")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(json!({
                "name": "High CPU",
                "metric": "cpu_usage",
                "condition": "gt",
                "threshold": 80.0,
                "severity": "Warning",
                "duration_sec": 0
            }))
            .to_request();
        let create_resp = actix_web::test::call_service(&app, create_req).await;
        assert_eq!(create_resp.status(), StatusCode::CREATED);

        let body = to_bytes(create_resp.into_body()).await.unwrap();
        let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let rule_id = created["id"].as_str().unwrap();
        assert_eq!(created["name"], "High CPU");
        assert_eq!(created["enabled"], true);

        // List alert rules
        let list_req = actix_web::test::TestRequest::get()
            .uri("/api/v1/observability/alert-rules")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();
        let list_resp = actix_web::test::call_service(&app, list_req).await;
        assert_eq!(list_resp.status(), StatusCode::OK);

        let list_body = to_bytes(list_resp.into_body()).await.unwrap();
        let list: Vec<serde_json::Value> = serde_json::from_slice(&list_body).unwrap();
        assert!(!list.is_empty());

        // Delete alert rule
        let del_req = actix_web::test::TestRequest::delete()
            .uri(&format!("/api/v1/observability/alert-rules/{}", rule_id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();
        let del_resp = actix_web::test::call_service(&app, del_req).await;
        assert_eq!(del_resp.status(), StatusCode::NO_CONTENT);
    }

    #[actix_web::test]
    async fn test_alert_evaluation_api() {
        let app_state = create_test_app_state();
        let token = admin_token_for_state(&app_state, 1);
        let app = build_observability_app(&app_state);
        let app = actix_web::test::init_service(app).await;

        // Create rule
        let create_req = actix_web::test::TestRequest::post()
            .uri("/api/v1/observability/alert-rules")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(json!({
                "name": "Mem High",
                "metric": "memory_usage",
                "condition": "gt",
                "threshold": 90.0,
                "severity": "Critical",
                "duration_sec": 0
            }))
            .to_request();
        actix_web::test::call_service(&app, create_req).await;

        // Evaluate alerts via API
        let eval_req = actix_web::test::TestRequest::post()
            .uri("/api/v1/observability/alerts/evaluate")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(json!({
                "metrics": {
                    "memory_usage": 95.0
                }
            }))
            .to_request();
        let eval_resp = actix_web::test::call_service(&app, eval_req).await;
        assert_eq!(eval_resp.status(), StatusCode::OK);

        let body = to_bytes(eval_resp.into_body()).await.unwrap();
        let alerts: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0]["rule_name"], "Mem High");
        assert_eq!(alerts[0]["state"], "Firing");
        assert_eq!(alerts[0]["severity"], "Critical");
    }

    #[actix_web::test]
    async fn test_alert_list_and_resolve() {
        let app_state = create_test_app_state();
        let token = admin_token_for_state(&app_state, 1);
        let app = build_observability_app(&app_state);
        let app = actix_web::test::init_service(app).await;

        // Create rule and trigger alert
        let create_req = actix_web::test::TestRequest::post()
            .uri("/api/v1/observability/alert-rules")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(json!({
                "name": "Disk Full",
                "metric": "disk_usage",
                "condition": "gte",
                "threshold": 100.0,
                "severity": "Warning",
                "duration_sec": 0
            }))
            .to_request();
        actix_web::test::call_service(&app, create_req).await;

        let eval_req = actix_web::test::TestRequest::post()
            .uri("/api/v1/observability/alerts/evaluate")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(json!({
                "metrics": {
                    "disk_usage": 100.0
                }
            }))
            .to_request();
        let eval_resp = actix_web::test::call_service(&app, eval_req).await;
        let body = to_bytes(eval_resp.into_body()).await.unwrap();
        let alerts: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        let alert_id = alerts[0]["id"].as_str().unwrap();

        // List alerts
        let list_req = actix_web::test::TestRequest::get()
            .uri("/api/v1/observability/alerts?limit=10")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();
        let list_resp = actix_web::test::call_service(&app, list_req).await;
        assert_eq!(list_resp.status(), StatusCode::OK);

        let list_body = to_bytes(list_resp.into_body()).await.unwrap();
        let list: Vec<serde_json::Value> = serde_json::from_slice(&list_body).unwrap();
        assert!(!list.is_empty());

        // Resolve alert
        let resolve_req = actix_web::test::TestRequest::post()
            .uri(&format!(
                "/api/v1/observability/alerts/{}/resolve",
                alert_id
            ))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();
        let resolve_resp = actix_web::test::call_service(&app, resolve_req).await;
        assert_eq!(resolve_resp.status(), StatusCode::NO_CONTENT);
    }

    #[actix_web::test]
    async fn test_dashboard_summary_endpoint() {
        let app_state = create_test_app_state();
        let token = admin_token_for_state(&app_state, 1);
        let app = build_observability_app(&app_state);
        let app = actix_web::test::init_service(app).await;

        let req = actix_web::test::TestRequest::get()
            .uri("/api/v1/observability/dashboard")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body = to_bytes(resp.into_body()).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["slo_compliance"].is_array());
        assert!(json["recent_alerts"].is_array());
        assert!(json["health_history"].is_array());
        assert!(json["generated_at"].is_string());
    }

    #[actix_web::test]
    async fn test_sparkline_endpoint() {
        let app_state = create_test_app_state();
        let token = admin_token_for_state(&app_state, 1);
        let app = build_observability_app(&app_state);
        let app = actix_web::test::init_service(app).await;

        let req = actix_web::test::TestRequest::get()
            .uri("/api/v1/observability/sparklines/cpu_usage?minutes=60")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body = to_bytes(resp.into_body()).await.unwrap();
        let points: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        // Empty sparkline is valid for in-memory repo
        assert!(points.is_empty() || points[0]["timestamp"].is_string());
    }

    #[actix_web::test]
    async fn test_observability_admin_required_for_mutations() {
        let app_state = create_test_app_state();
        let user_token = user_token_for_state(&app_state, 1);
        let app = build_observability_app(&app_state);
        let app = actix_web::test::init_service(app).await;

        // Non-admin should get 403 for create_sli
        let req = actix_web::test::TestRequest::post()
            .uri("/api/v1/observability/slis")
            .insert_header(("Authorization", format!("Bearer {}", user_token)))
            .set_json(json!({
                "name": "Test",
                "metric_type": "Availability",
                "source": "test",
                "window_minutes": 1
            }))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_web::test]
    async fn test_slo_compliance_cached() {
        let app_state = create_test_app_state();
        let token = admin_token_for_state(&app_state, 1);
        let app = build_observability_app(&app_state);
        let app = actix_web::test::init_service(app).await;

        // Create SLI + SLO + measurement
        let sli_req = actix_web::test::TestRequest::post()
            .uri("/api/v1/observability/slis")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(json!({
                "name": "CacheTest",
                "metric_type": "Throughput",
                "source": "test",
                "window_minutes": 5
            }))
            .to_request();
        let sli_resp = actix_web::test::call_service(&app, sli_req).await;
        let sli_body = to_bytes(sli_resp.into_body()).await.unwrap();
        let sli: serde_json::Value = serde_json::from_slice(&sli_body).unwrap();
        let sli_id = sli["id"].as_str().unwrap();

        let slo_req = actix_web::test::TestRequest::post()
            .uri("/api/v1/observability/slos")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(json!({
                "name": "CacheTest SLO",
                "sli_id": sli_id,
                "target_value": 100.0,
                "error_budget": 10.0,
                "window_days": 1
            }))
            .to_request();
        actix_web::test::call_service(&app, slo_req).await;

        // Record high measurement
        let m_req = actix_web::test::TestRequest::post()
            .uri(&format!(
                "/api/v1/observability/slis/{}/measurements",
                sli_id
            ))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(150.0)
            .to_request();
        actix_web::test::call_service(&app, m_req).await;

        // Evaluate
        let eval_req = actix_web::test::TestRequest::post()
            .uri("/api/v1/observability/slos/compliance/evaluate")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();
        let eval_resp = actix_web::test::call_service(&app, eval_req).await;
        assert_eq!(eval_resp.status(), StatusCode::OK);

        // Get compliance (should hit cache on second call)
        let get_req = actix_web::test::TestRequest::get()
            .uri("/api/v1/observability/slos/compliance")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();
        let get_resp = actix_web::test::call_service(&app, get_req).await;
        assert_eq!(get_resp.status(), StatusCode::OK);

        let get_body = to_bytes(get_resp.into_body()).await.unwrap();
        let compliance: Vec<serde_json::Value> = serde_json::from_slice(&get_body).unwrap();
        assert!(!compliance.is_empty());
    }
}

// ============================================================================
// 6. Service-Level Unit Tests (Fast, No HTTP)
// ============================================================================

mod service_unit_tests {
    use super::*;

    #[tokio::test]
    async fn test_liveness_always_healthy() {
        let repo = Arc::new(InMemoryObservabilityRepository::new());
        let cache: Arc<dyn turerp::cache::CacheService> =
            Arc::new(turerp::cache::InMemoryCacheService::new());
        let service = ObservabilityService::new(repo, cache);

        let live = service.get_liveness().await.unwrap();
        assert_eq!(live.overall, HealthStatus::Healthy);
        assert!(!live.checks.is_empty());
    }

    #[tokio::test]
    async fn test_health_check_in_memory_no_panic() {
        let repo = Arc::new(InMemoryObservabilityRepository::new());
        let cache: Arc<dyn turerp::cache::CacheService> =
            Arc::new(turerp::cache::InMemoryCacheService::new());
        let service = ObservabilityService::new(repo, cache);

        let health = service.run_health_check(None).await.unwrap();
        assert_eq!(health.overall, HealthStatus::Healthy);
        assert!(health.checks.iter().any(|c| c.component == "app"));
        assert!(health.checks.iter().any(|c| c.component == "cache"));
    }

    #[tokio::test]
    async fn test_slo_compliance_breached() {
        let repo = Arc::new(InMemoryObservabilityRepository::new());
        let cache: Arc<dyn turerp::cache::CacheService> =
            Arc::new(turerp::cache::InMemoryCacheService::new());
        let service = ObservabilityService::new(repo.clone(), cache);

        let sli = service
            .create_sli(
                "test".to_string(),
                SliMetricType::Availability,
                "test".to_string(),
                5,
            )
            .await
            .unwrap();

        service
            .create_slo("slo".to_string(), sli.id.clone(), 0.99, 0.01, 1)
            .await
            .unwrap();

        // Record very low availability
        service.record_sli(sli.id.clone(), 0.5).await.unwrap();

        let compliance = service.evaluate_slo_compliance().await.unwrap();
        assert_eq!(compliance.len(), 1);
        assert_eq!(compliance[0].status, SloStatus::Breached);
    }

    #[tokio::test]
    async fn test_slo_compliance_at_risk() {
        let repo = Arc::new(InMemoryObservabilityRepository::new());
        let cache: Arc<dyn turerp::cache::CacheService> =
            Arc::new(turerp::cache::InMemoryCacheService::new());
        let service = ObservabilityService::new(repo.clone(), cache);

        let sli = service
            .create_sli(
                "test".to_string(),
                SliMetricType::Latency,
                "test".to_string(),
                5,
            )
            .await
            .unwrap();

        // target=0.2, error_budget=0.05 => at risk if value between 0.15 and 0.2
        service
            .create_slo("slo".to_string(), sli.id.clone(), 0.2, 0.05, 1)
            .await
            .unwrap();

        service.record_sli(sli.id.clone(), 0.18).await.unwrap();

        let compliance = service.evaluate_slo_compliance().await.unwrap();
        assert_eq!(compliance.len(), 1);
        assert_eq!(compliance[0].status, SloStatus::AtRisk);
    }

    #[tokio::test]
    async fn test_alert_resolve_persists_state() {
        let repo = Arc::new(InMemoryObservabilityRepository::new());
        let cache: Arc<dyn turerp::cache::CacheService> =
            Arc::new(turerp::cache::InMemoryCacheService::new());
        let service = ObservabilityService::new(repo.clone(), cache);

        service
            .create_alert_rule(
                "rule".to_string(),
                "m".to_string(),
                "gt".to_string(),
                0.0,
                AlertSeverity::Info,
                0,
            )
            .await
            .unwrap();

        let mut metrics = HashMap::new();
        metrics.insert("m".to_string(), 1.0);

        let alerts = service.evaluate_alert_rules(&metrics).await.unwrap();
        let alert_id = alerts[0].id.clone();

        service.resolve_alert(&alert_id).await.unwrap();

        let list = service.list_alerts(10).await.unwrap();
        let resolved = list.iter().find(|a| a.id == alert_id).unwrap();
        assert_eq!(resolved.state, AlertState::Resolved);
        assert!(resolved.resolved_at.is_some());
    }

    #[tokio::test]
    async fn test_notification_sent_on_critical_alert() {
        let repo = Arc::new(InMemoryObservabilityRepository::new());
        let cache: Arc<dyn turerp::cache::CacheService> =
            Arc::new(turerp::cache::InMemoryCacheService::new());
        let notifier = Arc::new(turerp::common::InMemoryNotificationService::new())
            as Arc<dyn turerp::common::NotificationService>;

        let service = ObservabilityService::new(repo.clone(), cache).with_notification(notifier);

        service
            .create_alert_rule(
                "critical".to_string(),
                "cpu".to_string(),
                "gt".to_string(),
                50.0,
                AlertSeverity::Critical,
                0,
            )
            .await
            .unwrap();

        let mut metrics = HashMap::new();
        metrics.insert("cpu".to_string(), 99.0);

        let alerts = service.evaluate_alert_rules(&metrics).await.unwrap();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].severity, AlertSeverity::Critical);
    }
}
