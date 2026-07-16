//! Forecasting Integration Tests

use actix_web::{body::to_bytes, http::StatusCode, test, web, App};
use rust_decimal_macros::dec;
use serde_json::json;
use std::sync::Arc;

use crate::common::*;

use turerp::api::v1_forecasting_configure;
use turerp::domain::forecasting::repository::InMemoryForecastingRepository;
use turerp::domain::forecasting::service::ForecastingService;
use turerp::middleware::JwtAuthMiddleware;

fn build_test_app_with_forecasting(
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
    let jwt = create_test_jwt_service();
    App::new()
        .wrap(JwtAuthMiddleware::new(jwt))
        .app_data(web::Data::new(state.clone()))
        .app_data(state.auth.auth_service.clone())
        .app_data(state.auth.user_service.clone())
        .app_data(state.auth.jwt_service.clone())
        .app_data(state.commerce.cari_service.clone())
        .app_data(state.commerce.stock_service.clone())
        .app_data(state.commerce.invoice_service.clone())
        .app_data(state.commerce.sales_service.clone())
        .app_data(state.hr.hr_service.clone())
        .app_data(state.finance.accounting_service.clone())
        .app_data(state.project.project_service.clone())
        .app_data(state.project.manufacturing_service.clone())
        .app_data(state.project.crm_service.clone())
        .app_data(state.admin.tenant_service.clone())
        .app_data(state.admin.tenant_config_service.clone())
        .app_data(state.i18n.clone())
        .app_data(state.assets_service.clone())
        .app_data(state.feature_service.clone())
        .app_data(state.commerce.product_service.clone())
        .app_data(state.commerce.purchase_service.clone())
        .app_data(state.chart_of_accounts_service.clone())
        .app_data(state.finance.tax_service.clone())
        .app_data(state.integration.customer_portal_service.clone())
        .app_data(state.integration.webhook_service.clone())
        .app_data(state.infra.search_service.clone())
        .app_data(state.infra.report_engine.clone())
        .app_data(state.infra.job_scheduler.clone())
        .app_data(state.infra.notification_service.clone())
        .app_data(state.analytics.audit_service.clone())
        .app_data(state.finance.bank_service.clone())
        .app_data(state.finance.cost_center_service.clone())
        .app_data(state.document.dashboard_service.clone())
        .app_data(state.document.file_storage.clone())
        .app_data(state.project.qc_service.clone())
        .app_data(state.admin.settings_service.clone())
        .app_data(state.admin.api_key_service.clone())
        .app_data(state.analytics.subscription_service.clone())
        .app_data(state.integration.workflow_service.clone())
        .app_data(state.finance.currency_service.clone())
        .app_data(state.infra.import_service.clone())
        .app_data(state.commerce.inter_company_service.clone())
        .app_data(state.integration.efatura_service.clone())
        .app_data(state.integration.edefter_service.clone())
        .app_data(state.commerce.company_service.clone())
        .app_data(state.analytics.forecasting_service.clone())
        .app_data(state.hr.shift_service.clone())
        .service(
            web::scope("/api")
                .configure(configure_all_routes)
                .configure(configure_v1_routes)
                .configure(v1_forecasting_configure),
        )
}

async fn create_seeded_app_state() -> turerp::app::AppState {
    let mut state = create_test_app_state().await;

    let repo = InMemoryForecastingRepository::new();
    let today = chrono::Utc::now();

    repo.seed_product(1, 1, "Widget A");
    repo.seed_product(2, 1, "Widget B");

    for i in 0..10 {
        repo.seed_sale(1, 1, dec!(5.0), today - chrono::Duration::days(i));
    }
    for i in 0..5 {
        repo.seed_sale(2, 1, dec!(10.0), today - chrono::Duration::days(i));
    }

    repo.seed_stock_level(1, 1, dec!(100.0), dec!(10.0));
    repo.seed_stock_level(1, 2, dec!(5.0), dec!(1.0));

    let forecasting_service = ForecastingService::new(Arc::new(repo));
    state.analytics.forecasting_service = web::Data::new(forecasting_service);

    state
}

// ============================================================================
// Forecasting Tests
// ============================================================================

#[actix_web::test]
async fn test_forecast_demand_success() {
    let state = create_seeded_app_state().await;
    let app = test::init_service(build_test_app_with_forecasting(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/forecasting/demand",
        &token,
    )
    .set_json(json!({
        "product_id": 1,
        "warehouse_id": 1,
        "periods": 4,
        "period_type": "Daily",
        "history_days": 30
    }))
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["product_id"], 1);
    assert_eq!(json["product_name"], "Widget A");
    assert_eq!(json["periods_ahead"], 4);
    assert!(json["forecasted_quantity"].as_str().is_some());
}

#[actix_web::test]
async fn test_forecast_demand_not_found() {
    let state = create_seeded_app_state().await;
    let app = test::init_service(build_test_app_with_forecasting(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/forecasting/demand",
        &token,
    )
    .set_json(json!({
        "product_id": 999,
        "warehouse_id": 1,
        "periods": 4,
        "period_type": "Daily",
        "history_days": 30
    }))
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_forecast_demand_validation_error() {
    let state = create_seeded_app_state().await;
    let app = test::init_service(build_test_app_with_forecasting(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/forecasting/demand",
        &token,
    )
    .set_json(json!({
        "product_id": 1,
        "warehouse_id": 1,
        "periods": 0,
        "period_type": "Daily",
        "history_days": 800
    }))
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn test_reorder_suggestions_success() {
    let state = create_seeded_app_state().await;
    let app = test::init_service(build_test_app_with_forecasting(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/forecasting/reorder",
        &token,
    )
    .set_json(json!({
        "warehouse_id": 1,
        "lead_time_days": 7,
        "safety_factor": "0.5"
    }))
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json.as_array().unwrap();
    assert!(!items.is_empty());
    let critical = items.iter().find(|i| i["product_id"] == 2);
    assert!(critical.is_some());
}

#[actix_web::test]
async fn test_stock_alerts_success() {
    let state = create_seeded_app_state().await;
    let app = test::init_service(build_test_app_with_forecasting(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/forecasting/alerts",
        &token,
    )
    .set_json(json!({
        "warehouse_id": 1,
        "alert_types": ["BelowSafetyStock"]
    }))
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json.as_array().unwrap();
    assert!(!items.is_empty());
    let alert = items.iter().find(|i| i["product_id"] == 2);
    assert!(alert.is_some());
    assert_eq!(alert.unwrap()["alert_type"], "BelowSafetyStock");
}

#[actix_web::test]
async fn test_forecast_report_success() {
    let state = create_seeded_app_state().await;
    let app = test::init_service(build_test_app_with_forecasting(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/forecasting/report?warehouse_id=1",
        &token,
    )
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json.as_array().unwrap();
    assert!(!items.is_empty());
    let report = items.iter().find(|i| i["product_id"] == 2);
    assert!(report.is_some());
    assert!(report.unwrap()["reorder_suggestion"].is_object());
}

#[actix_web::test]
async fn test_forecast_unauthorized() {
    let state = create_seeded_app_state().await;
    let app = test::init_service(build_test_app_with_forecasting(&state)).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/forecasting/demand")
        .set_json(json!({
            "product_id": 1,
            "warehouse_id": 1,
            "periods": 4,
            "period_type": "Daily",
            "history_days": 30
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
