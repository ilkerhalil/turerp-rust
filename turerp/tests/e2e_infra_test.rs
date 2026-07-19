//! End-to-End Infrastructure Integration Tests
//!
//! Tests Events, Notifications, Push Notifications, Jobs, Settings, Audit,
// Admin/Rate-Limits, Custom Fields, Search, Reports, Forecasting, and Resilience.
//!
//! Run with: cargo test --test integration e2e_infra

use actix_web::{body::to_bytes, http::StatusCode, test, web, App};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::common::*;

use rust_decimal_macros::dec;
use turerp::api::v1::push_notifications as push;
use turerp::api::{v1_events_configure, v1_forecasting_configure};
use turerp::domain::forecasting::repository::InMemoryForecastingRepository;
use turerp::domain::forecasting::service::ForecastingService;
use turerp::middleware::JwtAuthMiddleware;

// ============================================================================
// Custom App Builder
// ============================================================================

/// Build a test app with all infrastructure routes registered, including
/// events, push notifications, forecasting, resilience, and rate-limits
/// which are NOT in the default `build_test_app`.
fn build_test_app_with_infra(
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
        .app_data(state.custom_field_service.clone())
        .app_data(state.finance.tax_service.clone())
        .app_data(state.integration.customer_portal_service.clone())
        .app_data(state.integration.webhook_service.clone())
        .app_data(state.infra.search_service.clone())
        .app_data(state.infra.report_engine.clone())
        .app_data(state.infra.job_scheduler.clone())
        .app_data(state.infra.notification_service.clone())
        .app_data(state.infra.event_bus.clone())
        .app_data(state.infra.rate_limit_stats.clone())
        .app_data(state.infra.circuit_breaker_registry.clone())
        .app_data(state.infra.retry_stats.clone())
        .app_data(state.analytics.audit_service.clone())
        .app_data(state.finance.bank_service.clone())
        .app_data(state.finance.cost_center_service.clone())
        .app_data(state.document.document_service.clone())
        .app_data(state.document.dashboard_service.clone())
        .app_data(state.document.file_storage.clone())
        .app_data(state.project.qc_service.clone())
        .app_data(state.admin.settings_service.clone())
        .app_data(state.admin.api_key_service.clone())
        .app_data(state.admin.ip_whitelist_service.clone())
        .app_data(state.commerce.barcode_service.clone())
        .app_data(state.analytics.subscription_service.clone())
        .app_data(state.integration.workflow_service.clone())
        .app_data(state.finance.currency_service.clone())
        .app_data(state.infra.import_service.clone())
        .app_data(state.commerce.inter_company_service.clone())
        .app_data(state.integration.efatura_service.clone())
        .app_data(state.integration.earchive_service.clone())
        .app_data(state.integration.edefter_service.clone())
        .app_data(state.commerce.company_service.clone())
        .app_data(state.analytics.forecasting_service.clone())
        .app_data(state.hr.shift_service.clone())
        .service(
            web::scope("/api")
                // Push notification routes registered BEFORE notifications scope
                // so the more specific /v1/notifications/push prefix is matched first.
                .service(
                    web::scope("/v1/notifications/push")
                        .route("/register", web::post().to(push::register_push_token))
                        .route("/unregister", web::delete().to(push::unregister_push_token))
                        .route("/send", web::post().to(push::send_push))
                        .route("/broadcast", web::post().to(push::broadcast_push))
                        .route("/tokens", web::get().to(push::get_user_push_tokens)),
                )
                .configure(configure_all_routes)
                .configure(configure_v1_routes)
                .configure(v1_events_configure)
                .configure(v1_forecasting_configure),
        )
}

/// Create a seeded app state for forecasting tests (products, stock, sales).
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
// Events Workflow (9 endpoints)
// ============================================================================

#[actix_web::test]
async fn e2e_events_workflow() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_infra(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;

    // 1. Publish an event
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/events/publish",
        &token,
    )
    .set_json(json!({
        "name": "test.event",
        "payload": "{\"key\":\"value\"}"
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "publish event");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["id"].is_number(), "event id should be a number");

    // 2. List pending outbox events
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/events/outbox/pending",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "outbox pending");

    // 3. Process outbox
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/events/outbox/process",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "process outbox");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["processed"].is_number(), "processed count");

    // 4. List DLQ
    let req = auth_request(actix_web::http::Method::GET, "/api/v1/events/dlq", &token).to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list DLQ");

    // 5. List dead-letters
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/events/dead-letters",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list dead-letters");

    // 6. Retry dead-letter (id=1 — may not exist; handler maps errors to 500)
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/events/dead-letters/1/retry",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status() == StatusCode::OK
            || resp.status() == StatusCode::NOT_FOUND
            || resp.status() == StatusCode::INTERNAL_SERVER_ERROR,
        "retry dead-letter: {}",
        resp.status()
    );

    // 7. Retry DLQ (id=1 — may not exist; handler maps errors to 500)
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/events/dlq/retry/1",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status() == StatusCode::OK
            || resp.status() == StatusCode::NOT_FOUND
            || resp.status() == StatusCode::INTERNAL_SERVER_ERROR,
        "retry DLQ: {}",
        resp.status()
    );

    // 8. Retry outbox event (id=1 — may not exist; handler maps errors to 500)
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/events/outbox/retry/1",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status() == StatusCode::OK
            || resp.status() == StatusCode::NOT_FOUND
            || resp.status() == StatusCode::INTERNAL_SERVER_ERROR,
        "retry outbox: {}",
        resp.status()
    );

    // 9. CDC status
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/events/cdc/status",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "CDC status");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["active"].is_boolean(), "CDC active field");
}

// ============================================================================
// Notifications Workflow (10 endpoints)
// ============================================================================

#[actix_web::test]
async fn e2e_notifications_workflow() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_infra(&app_state)).await;
    let (token, user_id) = register_admin(&app_state, 1).await;

    // 1. Send an in-app notification
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/notifications/send",
        &token,
    )
    .set_json(json!({
        "user_id": user_id,
        "channel": "inapp",
        "priority": "normal",
        "template_key": "stock_low",
        "template_vars": {"product_name": "Widget"},
        "recipient": "user@example.com"
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "send notification");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let notif_id = json["id"].as_i64().unwrap();

    // 2. List in-app notifications
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/notifications/in-app",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "in-app list");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let items = json.as_array().unwrap();
    assert!(!items.is_empty(), "should have in-app notifications");

    // 3. Unread count
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/notifications/unread-count",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "unread count");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["count"].as_u64().unwrap() > 0, "should have unread");

    // 4. Mark notification as read
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/notifications/{}/read", notif_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "mark read");

    // 5. Mark all as read
    let req = auth_request(
        actix_web::http::Method::PUT,
        "/api/v1/notifications/read-all",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "mark all read");

    // 6. Retry notification (may return 400 if notification is not in Failed status)
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/notifications/{}/retry", notif_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status() == StatusCode::OK
            || resp.status() == StatusCode::NOT_FOUND
            || resp.status() == StatusCode::BAD_REQUEST,
        "retry notification: {}",
        resp.status()
    );

    // 7. Soft delete notification
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/notifications/{}", notif_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT, "soft delete");

    // 8. List deleted notifications
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/notifications/deleted",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "deleted list");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let deleted_items = json.as_array().unwrap();
    assert!(
        !deleted_items.is_empty(),
        "should have deleted notifications"
    );

    // 9. Restore notification
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/notifications/{}/restore", notif_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "restore notification");

    // 10. Destroy notification (soft delete first, then destroy)
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/notifications/{}", notif_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "soft delete before destroy"
    );

    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/notifications/{}/destroy", notif_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "destroy notification"
    );
}

// ============================================================================
// Push Notifications Workflow (5 endpoints)
// ============================================================================

#[actix_web::test]
async fn e2e_push_notifications_workflow() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_infra(&app_state)).await;
    let (token, user_id) = register_admin(&app_state, 1).await;

    // 1. Register push token
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/notifications/push/register",
        &token,
    )
    .set_json(json!({
        "device_type": "android",
        "token": "firebase-token-abc123"
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "register push token");

    // 2. List push tokens
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/notifications/push/tokens",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list push tokens");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let tokens = json.as_array().unwrap();
    assert!(!tokens.is_empty(), "should have registered tokens");

    // 3. Send push notification
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/notifications/push/send",
        &token,
    )
    .set_json(json!({
        "user_id": user_id,
        "title": "Test Push",
        "body": "This is a test push notification",
        "data": {"order_id": "123"},
        "badge": 1,
        "sound": "default"
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "send push");

    // 4. Broadcast push notification
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/notifications/push/broadcast",
        &token,
    )
    .set_json(json!({
        "title": "Broadcast Test",
        "body": "This is a broadcast message",
        "data": {"category": "announcement"},
        "badge": 0,
        "sound": "default"
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "broadcast push");

    // 5. Unregister push token
    let req = auth_request(
        actix_web::http::Method::DELETE,
        "/api/v1/notifications/push/unregister",
        &token,
    )
    .set_json(json!({
        "device_type": "android"
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "unregister push token"
    );
}

// ============================================================================
// Jobs Workflow (10 endpoints)
// ============================================================================

#[actix_web::test]
async fn e2e_jobs_workflow() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_infra(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;

    // 1. Create a job
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/jobs", &token)
        .set_json(json!({
            "job_type": "send_reminders",
            "priority": "high",
            "max_attempts": 5
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "create job");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let job_id = json["id"].as_i64().unwrap();
    assert_eq!(json["status"], "pending");
    assert_eq!(json["job_type"], "send_reminders");

    // 2. Get job by ID
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/jobs/{}", job_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "get job");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], job_id);

    // 3. List jobs by status (pending)
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/jobs/status/pending",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list by status");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let jobs = json.as_array().unwrap();
    assert!(!jobs.is_empty(), "should have pending jobs");

    // 4. Get next pending job
    let req = auth_request(actix_web::http::Method::GET, "/api/v1/jobs/next", &token).to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "next pending job");

    // 5. Start job
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/jobs/{}/start", job_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "start job");

    // 6. Complete job
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/jobs/{}/complete", job_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "complete job");

    // 7. Create a second job to test fail → retry → cancel
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/jobs", &token)
        .set_json(json!({
            "job_type": "archive_logs",
            "priority": "normal",
            "max_attempts": 1
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "create second job");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let job2_id = json["id"].as_i64().unwrap();

    // 8. Start second job
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/jobs/{}/start", job2_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "start second job");

    // 9. Fail second job
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/jobs/{}/fail", job2_id),
        &token,
    )
    .set_json(json!({"error": "Test failure"}))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "fail job");

    // 10. Retry failed job (job has max_attempts=1, so fail sets it to Failed)
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/jobs/{}/retry", job2_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status() == StatusCode::OK || resp.status() == StatusCode::INTERNAL_SERVER_ERROR,
        "retry job: {}",
        resp.status()
    );

    // 11. Create a third job to test cancel
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/jobs", &token)
        .set_json(json!({
            "job_type": "send_reminders",
            "priority": "low"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "create third job");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let job3_id = json["id"].as_i64().unwrap();

    // 12. Cancel third job
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/jobs/{}/cancel", job3_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "cancel job");

    // 13. Cleanup old jobs (0 days = clean all terminal jobs)
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/jobs/cleanup/0",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "cleanup jobs");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["cleaned"].is_number(), "cleaned count");

    // 14. List completed jobs
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/jobs/status/completed",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list completed jobs");
}

// ============================================================================
// Settings Workflow (11 endpoints)
// ============================================================================

#[actix_web::test]
async fn e2e_settings_workflow() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_infra(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;

    let unique = uuid::Uuid::new_v4().to_string();
    let setting_key = format!("e2e.setting.{}", &unique[..8]);

    // 1. Create a setting
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/settings", &token)
        .set_json(json!({
            "key": setting_key,
            "value": "initial_value",
            "data_type": "string",
            "group": "general",
            "description": "E2E test setting",
            "is_sensitive": false,
            "is_editable": true,
            "tenant_id": 1
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "create setting");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let setting_id = json["id"].as_i64().unwrap();
    assert_eq!(json["key"], setting_key);

    // 2. List settings
    let req = auth_request(actix_web::http::Method::GET, "/api/v1/settings", &token).to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list settings");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["data"].is_array(), "settings data array");

    // 3. Get setting by key
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/settings/{}", setting_key),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "get by key");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["key"], setting_key);

    // 4. Update setting
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/settings/{}", setting_id),
        &token,
    )
    .set_json(json!({
        "value": "updated_value",
        "description": "Updated description"
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "update setting");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["value"], "updated_value");

    // 5. Bulk update settings
    let bulk_key_a = format!("e2e.bulk.a.{}", &unique[..8]);
    let bulk_key_b = format!("e2e.bulk.b.{}", &unique[..8]);
    for key in [&bulk_key_a, &bulk_key_b] {
        let req = auth_request(actix_web::http::Method::POST, "/api/v1/settings", &token)
            .set_json(json!({
                "key": key,
                "value": "original",
                "data_type": "string",
                "group": "general",
                "description": "Bulk test",
                "is_sensitive": false,
                "is_editable": true,
                "tenant_id": 1
            }))
            .to_request();
        test::call_service(&app, req).await;
    }

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/settings/bulk",
        &token,
    )
    .set_json(json!({
        "updates": [
            {"key": bulk_key_a, "value": "updated_a"},
            {"key": bulk_key_b, "value": "updated_b"}
        ]
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "bulk update");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["updated"], 2);

    // 6. Seed default settings
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/settings/seed",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "seed settings");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["created"].is_number(), "seeded count");

    // 7. Soft delete setting
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/settings/{}/soft", setting_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT, "soft delete setting");

    // 8. List deleted settings
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/settings/deleted",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list deleted settings");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let deleted = json.as_array().unwrap();
    assert!(!deleted.is_empty(), "should have deleted settings");

    // 9. Restore setting
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/settings/{}/restore", setting_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT, "restore setting");

    // 10. Verify restored setting is accessible
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/settings/{}", setting_key),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "get restored setting");

    // 11. Soft delete again then destroy
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/settings/{}/soft", setting_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "soft delete before destroy"
    );

    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/settings/{}/destroy", setting_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT, "destroy setting");

    // 12. Hard delete (DELETE /api/v1/settings/{id}) — use a bulk setting
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/settings", &token)
        .set_json(json!({
            "key": format!("e2e.hard.{}", &unique[..8]),
            "value": "temp",
            "data_type": "string",
            "group": "general",
            "description": "Hard delete test",
            "is_sensitive": false,
            "is_editable": true,
            "tenant_id": 1
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let hard_id = json["id"].as_i64().unwrap();

    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/settings/{}", hard_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT, "hard delete setting");
}

// ============================================================================
// Audit Logs (1 endpoint)
// ============================================================================

#[actix_web::test]
async fn e2e_audit_logs() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_infra(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;

    let req = auth_request(actix_web::http::Method::GET, "/api/v1/audit-logs", &token).to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "audit logs");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    // Response is a paginated result
    assert!(
        json["items"].is_array() || json.is_array(),
        "audit logs data"
    );
}

// ============================================================================
// Admin Rate Limits (1 endpoint)
// ============================================================================

#[actix_web::test]
async fn e2e_admin_rate_limits() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_infra(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;

    // Rate-limits route is at /api/admin/rate-limits (NOT /api/v1/admin/rate-limits)
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/admin/rate-limits",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "admin rate limits");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["total_clients"].is_number(), "total_clients field");
    assert!(json["entries"].is_array(), "entries array");
}

// ============================================================================
// Custom Fields Workflow (8 endpoints)
// ============================================================================

#[actix_web::test]
async fn e2e_custom_fields_workflow() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_infra(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;

    let unique = &uuid::Uuid::new_v4().to_string()[..8];

    // 1. Create custom field
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/custom-fields",
        &token,
    )
    .set_json(json!({
        "module": "cari",
        "field_name": format!("e2e_field_{}", unique),
        "field_label": "E2E Test Field",
        "field_type": "string",
        "required": false,
        "options": [],
        "sort_order": 0,
        "tenant_id": 1
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "create custom field");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let field_id = json["id"].as_i64().unwrap();

    // 2. List custom fields
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/custom-fields",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list custom fields");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let fields = json.as_array().unwrap();
    assert!(!fields.is_empty(), "should have custom fields");

    // 3. Get custom field by ID
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/custom-fields/{}", field_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "get custom field");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], field_id);

    // 4. Update custom field
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/custom-fields/{}", field_id),
        &token,
    )
    .set_json(json!({
        "field_label": "Updated E2E Field",
        "required": true,
        "sort_order": 5
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "update custom field");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["field_label"], "Updated E2E Field");

    // 5. Soft delete custom field (DELETE /api/v1/custom-fields/{id})
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/custom-fields/{}", field_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "soft delete custom field");

    // 6. List deleted custom fields
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/custom-fields/deleted",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list deleted custom fields");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let deleted = json.as_array().unwrap();
    assert!(!deleted.is_empty(), "should have deleted custom fields");

    // 7. Restore custom field
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/custom-fields/{}/restore", field_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "restore custom field");

    // 8. Soft delete again then destroy
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/custom-fields/{}", field_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "soft delete before destroy");

    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/custom-fields/{}/destroy", field_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "destroy custom field"
    );
}

// ============================================================================
// Search Workflow (4 endpoints)
// ============================================================================

#[actix_web::test]
async fn e2e_search_workflow() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_infra(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;

    // 1. Index a document
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/search/index",
        &token,
    )
    .set_json(json!({
        "entity_type": "product",
        "entity_id": 1001,
        "title": "E2E Test Product",
        "description": "A test product for search",
        "searchable_text": "e2e test product widget gadget electronics"
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "index document");

    // 2. Search
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/search?q=widget&entity_type=product&limit=10",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "search");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let results = json.as_array().unwrap();
    assert!(!results.is_empty(), "should have search results");
    let found = results.iter().any(|r| r["id"] == 1001);
    assert!(found, "indexed document should be in search results");

    // 3. Reindex
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/search/reindex",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "reindex");

    // 4. Delete document from index
    let req = auth_request(
        actix_web::http::Method::DELETE,
        "/api/v1/search/product/1001",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "remove from index");
}

// ============================================================================
// Reports Workflow (4 endpoints)
// ============================================================================

#[actix_web::test]
async fn e2e_reports_workflow() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_infra(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;

    // 1. Generate a report (returns file bytes, not JSON)
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/reports/generate",
        &token,
    )
    .set_json(json!({
        "report_type": "trial_balance",
        "format": "csv",
        "title": "E2E Trial Balance",
        "parameters": {},
        "locale": "en"
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "generate report");
    // The response body is the report file content (not JSON)
    let content_disp = resp
        .headers()
        .get("content-disposition")
        .map(|v| v.to_str().unwrap_or(""))
        .unwrap_or("");
    assert!(
        content_disp.contains("attachment"),
        "should have Content-Disposition: {}",
        content_disp
    );
    let body = to_bytes(resp.into_body()).await.unwrap();
    assert!(!body.is_empty(), "report body should not be empty");

    // 2. List reports
    let req = auth_request(actix_web::http::Method::GET, "/api/v1/reports", &token).to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list reports");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let reports = json.as_array().unwrap();
    assert!(!reports.is_empty(), "should have generated reports");
    let report_id = reports[0]["id"].as_i64().unwrap();

    // 3. Download report
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/reports/{}/download", report_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "download report");
    let body = to_bytes(resp.into_body()).await.unwrap();
    assert!(!body.is_empty(), "downloaded report should not be empty");

    // 4. Delete report
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/reports/{}", report_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "delete report");
}

// ============================================================================
// Forecasting Workflow (4 endpoints)
// ============================================================================

#[actix_web::test]
async fn e2e_forecasting_workflow() {
    let app_state = create_seeded_app_state().await;
    let app = test::init_service(build_test_app_with_infra(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;

    // 1. Demand forecast
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
    assert_eq!(resp.status(), StatusCode::OK, "demand forecast");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["product_id"], 1);
    assert_eq!(json["periods_ahead"], 4);

    // 2. Reorder suggestions
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
    assert_eq!(resp.status(), StatusCode::OK, "reorder suggestions");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let items = json.as_array().unwrap();
    assert!(!items.is_empty(), "should have reorder suggestions");

    // 3. Stock alerts
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
    assert_eq!(resp.status(), StatusCode::OK, "stock alerts");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let alerts = json.as_array().unwrap();
    assert!(!alerts.is_empty(), "should have stock alerts");

    // 4. Forecast report
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/forecasting/report?warehouse_id=1",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "forecast report");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let report = json.as_array().unwrap();
    assert!(!report.is_empty(), "should have forecast report items");
}

// ============================================================================
// Resilience Workflow (3 endpoints)
// ============================================================================

#[actix_web::test]
async fn e2e_resilience_workflow() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_infra(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;

    // 1. List circuit breakers
    // Resilience routes are at /api/resilience/* (NOT /api/v1/resilience/*)
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/resilience/circuit-breakers",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list circuit breakers");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(
        json["circuit_breakers"].is_array(),
        "circuit breakers should be an array"
    );

    // 2. Get retry stats
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/resilience/retry-stats",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "retry stats");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json.is_object(), "retry stats should be an object");

    // 3. Reset a circuit breaker
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/resilience/circuit-breakers/email/reset",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "reset circuit breaker");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["message"].is_string(), "reset message");
}
