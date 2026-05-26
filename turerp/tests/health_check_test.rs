//! Health check endpoint integration tests

use actix_web::{http::StatusCode, test, web, App};

mod common;
use common::*;

/// Build a minimal app with only health endpoints (no auth middleware)
fn build_health_app(
    state: &turerp::app::AppState,
) -> App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse<actix_web::body::BoxBody>,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    App::new()
        .app_data(web::Data::new(state.clone()))
        .app_data(state.infra.cache_service.clone())
        .route("/health", web::get().to(health_check_handler))
        .route("/health/live", web::get().to(health_live_handler))
        .route("/health/ready", web::get().to(health_ready_handler))
}

async fn health_live_handler() -> actix_web::Result<actix_web::HttpResponse> {
    Ok(actix_web::HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "service": "turerp-erp",
        "version": env!("CARGO_PKG_VERSION")
    })))
}

async fn health_ready_handler(
    app_state: web::Data<turerp::app::AppState>,
) -> actix_web::Result<actix_web::HttpResponse> {
    let cache = app_state.infra.cache_service.get_ref();
    let cache_result = cache.health_check().await;

    match cache_result {
        Ok(_) => Ok(actix_web::HttpResponse::Ok().json(serde_json::json!({
            "status": "ok",
            "service": "turerp-erp",
            "version": env!("CARGO_PKG_VERSION"),
            "storage": "in-memory",
            "cache": "healthy"
        }))),
        Err(e) => {
            tracing::error!("Cache health check failed: {}", e);
            Ok(
                actix_web::HttpResponse::ServiceUnavailable().json(serde_json::json!({
                    "status": "unhealthy",
                    "service": "turerp-erp",
                    "version": env!("CARGO_PKG_VERSION"),
                    "storage": "in-memory",
                    "cache": "unhealthy"
                })),
            )
        }
    }
}

async fn health_check_handler(
    app_state: web::Data<turerp::app::AppState>,
) -> actix_web::Result<actix_web::HttpResponse> {
    health_ready_handler(app_state).await
}

#[actix_web::test]
async fn test_health_live_returns_ok() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_health_app(&state)).await;

    let req = test::TestRequest::get().uri("/health/live").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = test::read_body(resp).await;
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ok");
    assert_eq!(json["service"], "turerp-erp");
    assert!(json["version"].is_string());
}

#[actix_web::test]
async fn test_health_ready_in_memory_returns_ok() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_health_app(&state)).await;

    let req = test::TestRequest::get().uri("/health/ready").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = test::read_body(resp).await;
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ok");
    assert_eq!(json["storage"], "in-memory");
    assert_eq!(json["cache"], "healthy");
}

#[actix_web::test]
async fn test_health_root_returns_ok() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_health_app(&state)).await;

    let req = test::TestRequest::get().uri("/health").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = test::read_body(resp).await;
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ok");
}

#[actix_web::test]
async fn test_health_does_not_require_auth() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_health_app(&state)).await;

    // No Authorization header - should still succeed
    let req = test::TestRequest::get().uri("/health/live").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}
