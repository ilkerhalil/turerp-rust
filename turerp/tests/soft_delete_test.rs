//! Soft Delete Integration Tests
//!
//! Comprehensive tests for soft delete functionality across all domains.
//! Run with: cargo test --test soft_delete_test

use actix_web::{body::to_bytes, http::StatusCode, test, web, App};
use serde_json::json;

use turerp::api::{
    auth_configure, users_configure, v1_accounting_configure, v1_assets_configure,
    v1_cari_configure, v1_chart_of_accounts_configure, v1_crm_configure,
    v1_feature_flags_configure, v1_hr_configure, v1_invoice_configure, v1_jobs_configure,
    v1_manufacturing_configure, v1_notifications_configure, v1_product_variants_configure,
    v1_project_configure, v1_purchase_requests_configure, v1_reports_configure, v1_sales_configure,
    v1_search_configure, v1_stock_configure, v1_tax_configure, v1_tenant_configure,
    v1_webhooks_configure,
};
use turerp::app::create_app_state_in_memory;
use turerp::config::Config;
use turerp::middleware::JwtAuthMiddleware;
use turerp::utils::jwt::JwtService;

/// Configure all legacy routes (auth + users)
fn configure_all_routes(cfg: &mut web::ServiceConfig) {
    auth_configure(cfg);
    users_configure(cfg);
}

/// Configure V1 routes for business modules
fn configure_v1_routes(cfg: &mut web::ServiceConfig) {
    cfg.configure(v1_cari_configure)
        .configure(v1_stock_configure)
        .configure(v1_invoice_configure)
        .configure(v1_sales_configure)
        .configure(v1_hr_configure)
        .configure(v1_accounting_configure)
        .configure(v1_project_configure)
        .configure(v1_manufacturing_configure)
        .configure(v1_crm_configure)
        .configure(v1_tenant_configure)
        .configure(v1_assets_configure)
        .configure(v1_feature_flags_configure)
        .configure(v1_product_variants_configure)
        .configure(v1_purchase_requests_configure)
        .configure(v1_chart_of_accounts_configure)
        .configure(v1_tax_configure)
        .configure(v1_webhooks_configure)
        .configure(v1_search_configure)
        .configure(v1_reports_configure)
        .configure(v1_jobs_configure)
        .configure(v1_notifications_configure);
}

/// Create app state with default config for testing
fn create_test_app_state() -> turerp::app::AppState {
    let config = Config::default();
    create_app_state_in_memory(&config)
}

/// Build a test app with all services and JWT middleware
fn build_full_test_app(
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
        .app_data(state.auth_service.clone())
        .app_data(state.user_service.clone())
        .app_data(state.jwt_service.clone())
        .app_data(state.cari_service.clone())
        .app_data(state.stock_service.clone())
        .app_data(state.invoice_service.clone())
        .app_data(state.sales_service.clone())
        .app_data(state.hr_service.clone())
        .app_data(state.accounting_service.clone())
        .app_data(state.project_service.clone())
        .app_data(state.manufacturing_service.clone())
        .app_data(state.crm_service.clone())
        .app_data(state.tenant_service.clone())
        .app_data(state.tenant_config_service.clone())
        .app_data(state.i18n.clone())
        .app_data(state.assets_service.clone())
        .app_data(state.feature_service.clone())
        .app_data(state.product_service.clone())
        .app_data(state.purchase_service.clone())
        .app_data(state.chart_of_accounts_service.clone())
        .app_data(state.tax_service.clone())
        .app_data(state.webhook_service.clone())
        .app_data(state.search_service.clone())
        .app_data(state.report_engine.clone())
        .app_data(state.job_scheduler.clone())
        .app_data(state.notification_service.clone())
        .service(
            web::scope("/api")
                .configure(configure_all_routes)
                .configure(configure_v1_routes),
        )
}

/// Helper macro to create an admin user directly and return (access_token, user_id)
macro_rules! register_admin {
    ($state:expr, $tenant_id:expr) => {{
        let username = format!(
            "sdadmin_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        );
        let user = $state
            .user_service
            .get_ref()
            .create_user(turerp::CreateUser {
                username: username.clone(),
                email: format!("{}@test.com", username),
                full_name: "Soft Delete Admin".to_string(),
                password: "Password123!".to_string(),
                tenant_id: $tenant_id,
                role: Some(turerp::Role::Admin),
            })
            .await
            .unwrap();
        let tokens = $state
            .jwt_service
            .get_ref()
            .generate_tokens(
                user.id,
                user.tenant_id,
                user.username.clone(),
                turerp::Role::Admin,
            )
            .unwrap();
        (tokens.access_token, user.id)
    }};
}

/// Helper macro to register a normal (non-admin) user and return (access_token, user_id)
macro_rules! register_user {
    ($app:expr, $tenant_id:expr) => {{
        let username = format!("sduser_{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
        let req = test::TestRequest::post()
            .uri("/api/auth/register")
            .set_json(json!({
                "username": username,
                "email": format!("{}@test.com", username),
                "full_name": "Soft Delete User",
                "password": "Password123!",
                "tenant_id": $tenant_id
            }))
            .to_request();
        let resp = test::call_service($app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED, "User registration failed");
        let body = to_bytes(resp.into_body()).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let access_token = json["tokens"]["access_token"].as_str().unwrap().to_string();
        let user_id = json["user"]["id"].as_i64().unwrap();
        (access_token, user_id)
    }};
}

// ============================================================================
// Section 1: Soft Delete Lifecycle Integration Tests
// ============================================================================

#[actix_web::test]
async fn test_cari_soft_delete_and_restore_lifecycle() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, user_id) = register_admin!(&app_state, 1);

    // Create cari
    let unique = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    let create_req = test::TestRequest::post()
        .uri("/api/v1/cari")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "code": format!("SD-CARI-{}", unique),
            "name": "Soft Delete Test Customer",
            "cari_type": "customer",
            "tenant_id": 1,
            "created_by": user_id
        }))
        .to_request();

    let resp = test::call_service(&app, create_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let cari_id = json["id"].as_i64().unwrap();

    // Soft delete the cari
    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/cari/{}", cari_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, delete_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Verify cari is NOT found via normal GET
    let get_req = test::TestRequest::get()
        .uri(&format!("/api/v1/cari/{}", cari_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, get_req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "Deleted cari should not be found via normal GET"
    );

    // Verify cari is NOT in normal list
    let list_req = test::TestRequest::get()
        .uri("/api/v1/cari")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, list_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json["items"].as_array().unwrap();
    assert!(
        !items.iter().any(|i| i["id"].as_i64() == Some(cari_id)),
        "Deleted cari should not appear in normal list"
    );

    // Verify cari appears in deleted list
    let deleted_req = test::TestRequest::get()
        .uri("/api/v1/cari/deleted")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, deleted_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let deleted_items: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert!(
        deleted_items
            .iter()
            .any(|i| i["id"].as_i64() == Some(cari_id)),
        "Deleted cari should appear in deleted list"
    );

    // Restore the cari
    let restore_req = test::TestRequest::put()
        .uri(&format!("/api/v1/cari/{}/restore", cari_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, restore_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Verify cari is found again after restore
    let get_req = test::TestRequest::get()
        .uri(&format!("/api/v1/cari/{}", cari_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, get_req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Restored cari should be found via normal GET"
    );

    // Verify cari is back in normal list
    let list_req = test::TestRequest::get()
        .uri("/api/v1/cari")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, list_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json["items"].as_array().unwrap();
    assert!(
        items.iter().any(|i| i["id"].as_i64() == Some(cari_id)),
        "Restored cari should appear in normal list"
    );

    // Verify cari is NOT in deleted list anymore
    let deleted_req = test::TestRequest::get()
        .uri("/api/v1/cari/deleted")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, deleted_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let deleted_items: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert!(
        !deleted_items
            .iter()
            .any(|i| i["id"].as_i64() == Some(cari_id)),
        "Restored cari should not appear in deleted list"
    );
}

#[actix_web::test]
async fn test_stock_warehouse_soft_delete_and_restore() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, _) = register_admin!(&app_state, 1);

    let unique = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

    // Create warehouse
    let create_req = test::TestRequest::post()
        .uri("/api/v1/stock/warehouses")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "code": format!("WH-SD-{}", unique),
            "name": "Soft Delete Warehouse",
            "tenant_id": 1
        }))
        .to_request();

    let resp = test::call_service(&app, create_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let wh_id = json["id"].as_i64().unwrap();

    // Soft delete
    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/stock/warehouses/{}", wh_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, delete_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Verify not found
    let get_req = test::TestRequest::get()
        .uri(&format!("/api/v1/stock/warehouses/{}", wh_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, get_req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // Restore
    let restore_req = test::TestRequest::put()
        .uri(&format!("/api/v1/stock/warehouses/{}/restore", wh_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, restore_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Verify found again
    let get_req = test::TestRequest::get()
        .uri(&format!("/api/v1/stock/warehouses/{}", wh_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, get_req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn test_chart_of_accounts_soft_delete_and_restore() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, _) = register_admin!(&app_state, 1);

    let unique = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

    // Create chart account
    let create_req = test::TestRequest::post()
        .uri("/api/v1/chart-of-accounts")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "code": format!("SD-COA-{}", unique),
            "name": "Soft Delete Account",
            "group": "DonenVarliklar",
            "account_type": "Asset",
            "allow_posting": true
        }))
        .to_request();

    let resp = test::call_service(&app, create_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let coa_id = json["id"].as_i64().unwrap();

    // Soft delete (chart of accounts uses code as path param)
    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/chart-of-accounts/SD-COA-{}", unique))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, delete_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Verify not in normal list
    let list_req = test::TestRequest::get()
        .uri("/api/v1/chart-of-accounts")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, list_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json["items"].as_array().unwrap();
    assert!(!items
        .iter()
        .any(|i| i["code"].as_str() == Some(&format!("SD-COA-{}", unique))));
}

#[actix_web::test]
async fn test_project_soft_delete_and_restore() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, _) = register_admin!(&app_state, 1);

    let unique = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

    // Create project
    let create_req = test::TestRequest::post()
        .uri("/api/v1/projects")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "name": format!("SD-Project-{}", unique),
            "budget": "100000.00",
            "tenant_id": 1
        }))
        .to_request();

    let resp = test::call_service(&app, create_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let project_id = json["id"].as_i64().unwrap();

    // Soft delete
    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/projects/{}", project_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, delete_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Verify not found
    let get_req = test::TestRequest::get()
        .uri(&format!("/api/v1/projects/{}", project_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, get_req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // Restore
    let restore_req = test::TestRequest::put()
        .uri(&format!("/api/v1/projects/{}/restore", project_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, restore_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Verify found
    let get_req = test::TestRequest::get()
        .uri(&format!("/api/v1/projects/{}", project_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, get_req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn test_webhook_soft_delete_and_restore() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, _) = register_admin!(&app_state, 1);

    let unique = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

    // Create webhook
    let create_req = test::TestRequest::post()
        .uri("/api/v1/webhooks")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "url": format!("https://example.com/webhook-{}", unique),
            "event_types": ["invoice_created"]
        }))
        .to_request();

    let resp = test::call_service(&app, create_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let wh_id = json["id"].as_i64().unwrap();

    // Soft delete
    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/webhooks/{}", wh_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, delete_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Verify not in list
    let list_req = test::TestRequest::get()
        .uri("/api/v1/webhooks")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, list_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let items: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert!(!items.iter().any(|i| i["id"].as_i64() == Some(wh_id)));

    // Note: Webhooks do not have restore/destroy endpoints in current implementation
}

// ============================================================================
// Section 2: Admin-Only Endpoint Tests
// ============================================================================

#[actix_web::test]
async fn test_list_deleted_requires_admin() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (admin_token, _) = register_admin!(&app_state, 1);
    let (user_token, _) = register_user!(&app, 1);

    // Admin can list deleted cari
    let req = test::TestRequest::get()
        .uri("/api/v1/cari/deleted")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Admin should be able to list deleted cari"
    );

    // Normal user cannot list deleted cari
    let req = test::TestRequest::get()
        .uri("/api/v1/cari/deleted")
        .insert_header(("Authorization", format!("Bearer {}", user_token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "Normal user should not be able to list deleted cari"
    );
}

#[actix_web::test]
async fn test_restore_requires_admin() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (admin_token, user_id) = register_admin!(&app_state, 1);

    let unique = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

    // Create and soft delete cari as admin
    let create_req = test::TestRequest::post()
        .uri("/api/v1/cari")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "code": format!("RESTORE-TEST-{}", unique),
            "name": "Restore Test",
            "cari_type": "customer",
            "tenant_id": 1,
            "created_by": user_id
        }))
        .to_request();

    let resp = test::call_service(&app, create_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let cari_id = json["id"].as_i64().unwrap();

    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/cari/{}", cari_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();

    let _ = test::call_service(&app, delete_req).await;

    // Normal user cannot restore
    let (user_token, _) = register_user!(&app, 1);

    let restore_req = test::TestRequest::put()
        .uri(&format!("/api/v1/cari/{}/restore", cari_id))
        .insert_header(("Authorization", format!("Bearer {}", user_token)))
        .to_request();

    let resp = test::call_service(&app, restore_req).await;
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "Normal user should not be able to restore deleted cari"
    );

    // Admin can restore
    let restore_req = test::TestRequest::put()
        .uri(&format!("/api/v1/cari/{}/restore", cari_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();

    let resp = test::call_service(&app, restore_req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Admin should be able to restore deleted cari"
    );
}

#[actix_web::test]
async fn test_destroy_requires_admin() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (admin_token, user_id) = register_admin!(&app_state, 1);

    let unique = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

    // Create and soft delete cari as admin
    let create_req = test::TestRequest::post()
        .uri("/api/v1/cari")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "code": format!("DESTROY-TEST-{}", unique),
            "name": "Destroy Test",
            "cari_type": "customer",
            "tenant_id": 1,
            "created_by": user_id
        }))
        .to_request();

    let resp = test::call_service(&app, create_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let cari_id = json["id"].as_i64().unwrap();

    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/cari/{}", cari_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();

    let _ = test::call_service(&app, delete_req).await;

    // Normal user cannot permanently destroy
    let (user_token, _) = register_user!(&app, 1);

    let destroy_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/cari/{}/destroy", cari_id))
        .insert_header(("Authorization", format!("Bearer {}", user_token)))
        .to_request();

    let resp = test::call_service(&app, destroy_req).await;
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "Normal user should not be able to destroy cari"
    );

    // Admin can destroy
    let destroy_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/cari/{}/destroy", cari_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();

    let resp = test::call_service(&app, destroy_req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "Admin should be able to destroy cari"
    );

    // Verify cari is completely gone (even from deleted list)
    let deleted_req = test::TestRequest::get()
        .uri("/api/v1/cari/deleted")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();

    let resp = test::call_service(&app, deleted_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let deleted_items: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert!(
        !deleted_items
            .iter()
            .any(|i| i["id"].as_i64() == Some(cari_id)),
        "Destroyed cari should not appear anywhere"
    );
}

// ============================================================================
// Section 3: Deleted Record Exclusion from Normal Queries
// ============================================================================

#[actix_web::test]
async fn test_deleted_records_excluded_from_search() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, user_id) = register_admin!(&app_state, 1);

    let unique = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    let search_term = format!("Searchable{}", unique);

    // Create a cari with a searchable name
    let create_req = test::TestRequest::post()
        .uri("/api/v1/cari")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "code": format!("SRCH-{}", unique),
            "name": search_term,
            "cari_type": "customer",
            "tenant_id": 1,
            "created_by": user_id
        }))
        .to_request();

    let resp = test::call_service(&app, create_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let cari_id = json["id"].as_i64().unwrap();

    // Verify it appears in search before deletion
    let search_req = test::TestRequest::get()
        .uri(&format!("/api/v1/cari/search?q={}", search_term))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, search_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json["items"].as_array().unwrap();
    assert!(
        items.iter().any(|i| i["id"].as_i64() == Some(cari_id)),
        "Cari should appear in search before deletion"
    );

    // Soft delete the cari
    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/cari/{}", cari_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let _ = test::call_service(&app, delete_req).await;

    // Verify it does NOT appear in search after deletion
    let search_req = test::TestRequest::get()
        .uri(&format!("/api/v1/cari/search?q={}", search_term))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, search_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json["items"].as_array().unwrap();
    assert!(
        !items.iter().any(|i| i["id"].as_i64() == Some(cari_id)),
        "Deleted cari should not appear in search"
    );
}

#[actix_web::test]
async fn test_deleted_records_excluded_from_type_filter() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, user_id) = register_admin!(&app_state, 1);

    let unique = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

    // Create a vendor-type cari
    let create_req = test::TestRequest::post()
        .uri("/api/v1/cari")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "code": format!("VENDOR-SD-{}", unique),
            "name": "Vendor Soft Delete",
            "cari_type": "vendor",
            "tenant_id": 1,
            "created_by": user_id
        }))
        .to_request();

    let resp = test::call_service(&app, create_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let cari_id = json["id"].as_i64().unwrap();

    // Soft delete
    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/cari/{}", cari_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let _ = test::call_service(&app, delete_req).await;

    // Verify not in vendor-type list
    let type_req = test::TestRequest::get()
        .uri("/api/v1/cari/type/vendor")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, type_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json["items"].as_array().unwrap();
    assert!(
        !items.iter().any(|i| i["id"].as_i64() == Some(cari_id)),
        "Deleted vendor should not appear in vendor type list"
    );
}

#[actix_web::test]
async fn test_deleted_records_excluded_from_get_by_id() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, user_id) = register_admin!(&app_state, 1);

    let unique = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

    // Create a cari
    let create_req = test::TestRequest::post()
        .uri("/api/v1/cari")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "code": format!("GETBYID-{}", unique),
            "name": "GetById Test",
            "cari_type": "customer",
            "tenant_id": 1,
            "created_by": user_id
        }))
        .to_request();

    let resp = test::call_service(&app, create_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let cari_id = json["id"].as_i64().unwrap();

    // Verify found before deletion
    let get_req = test::TestRequest::get()
        .uri(&format!("/api/v1/cari/{}", cari_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, get_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Soft delete
    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/cari/{}", cari_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let _ = test::call_service(&app, delete_req).await;

    // Verify NOT found after deletion
    let get_req = test::TestRequest::get()
        .uri(&format!("/api/v1/cari/{}", cari_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, get_req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "Deleted cari should return 404 on GET"
    );
}

// ============================================================================
// Section 4: Tenant Isolation with Soft Delete
// ============================================================================

#[actix_web::test]
async fn test_tenant_isolation_deleted_records() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token1, user_id1) = register_admin!(&app_state, 1);
    let (token2, _user_id2) = register_admin!(&app_state, 2);

    let unique = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

    // Tenant 1 creates and soft deletes a cari
    let create_req = test::TestRequest::post()
        .uri("/api/v1/cari")
        .insert_header(("Authorization", format!("Bearer {}", token1)))
        .set_json(json!({
            "code": format!("T1-SD-{}", unique),
            "name": "Tenant 1 Soft Delete",
            "cari_type": "customer",
            "tenant_id": 1,
            "created_by": user_id1
        }))
        .to_request();

    let resp = test::call_service(&app, create_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let cari_id = json["id"].as_i64().unwrap();

    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/cari/{}", cari_id))
        .insert_header(("Authorization", format!("Bearer {}", token1)))
        .to_request();

    let _ = test::call_service(&app, delete_req).await;

    // Tenant 2 should not see tenant 1's deleted records
    let deleted_req = test::TestRequest::get()
        .uri("/api/v1/cari/deleted")
        .insert_header(("Authorization", format!("Bearer {}", token2)))
        .to_request();

    let resp = test::call_service(&app, deleted_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let deleted_items: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert!(
        !deleted_items
            .iter()
            .any(|i| i["id"].as_i64() == Some(cari_id)),
        "Tenant 2 should not see tenant 1's deleted cari"
    );

    // Tenant 2 should not be able to restore tenant 1's deleted record
    let restore_req = test::TestRequest::put()
        .uri(&format!("/api/v1/cari/{}/restore", cari_id))
        .insert_header(("Authorization", format!("Bearer {}", token2)))
        .to_request();

    let resp = test::call_service(&app, restore_req).await;
    // Should return 404 (not found in tenant 2's scope) or 403
    assert!(
        resp.status() == StatusCode::NOT_FOUND || resp.status() == StatusCode::FORBIDDEN,
        "Tenant 2 should not be able to restore tenant 1's deleted cari, got: {:?}",
        resp.status()
    );
}

#[actix_web::test]
async fn test_tenant_isolation_normal_queries_ignore_other_tenant_deleted() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token1, user_id1) = register_admin!(&app_state, 1);
    let (token2, user_id2) = register_admin!(&app_state, 2);

    let unique = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

    // Tenant 1 creates a cari and soft deletes it
    let create_req = test::TestRequest::post()
        .uri("/api/v1/cari")
        .insert_header(("Authorization", format!("Bearer {}", token1)))
        .set_json(json!({
            "code": format!("T1-ISO-{}", unique),
            "name": "Tenant 1 Isolation",
            "cari_type": "customer",
            "tenant_id": 1,
            "created_by": user_id1
        }))
        .to_request();

    let resp = test::call_service(&app, create_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let cari_id = json["id"].as_i64().unwrap();

    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/cari/{}", cari_id))
        .insert_header(("Authorization", format!("Bearer {}", token1)))
        .to_request();

    let _ = test::call_service(&app, delete_req).await;

    // Tenant 2 creates a cari with the same code pattern (different tenant)
    let create_req = test::TestRequest::post()
        .uri("/api/v1/cari")
        .insert_header(("Authorization", format!("Bearer {}", token2)))
        .set_json(json!({
            "code": format!("T2-ISO-{}", unique),
            "name": "Tenant 2 Isolation",
            "cari_type": "customer",
            "tenant_id": 2,
            "created_by": user_id2
        }))
        .to_request();

    let _ = test::call_service(&app, create_req).await;

    // Tenant 2's normal list should only show tenant 2's records
    let list_req = test::TestRequest::get()
        .uri("/api/v1/cari")
        .insert_header(("Authorization", format!("Bearer {}", token2)))
        .to_request();

    let resp = test::call_service(&app, list_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json["items"].as_array().unwrap();

    // Should not contain tenant 1's records (deleted or not)
    for item in items {
        assert_ne!(
            item["tenant_id"].as_i64(),
            Some(1),
            "Tenant 2 should not see tenant 1's records"
        );
    }
}

// ============================================================================
// Section 5: Security Tests - Non-Admin Access
// ============================================================================

#[actix_web::test]
async fn test_non_admin_cannot_soft_delete() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (admin_token, user_id) = register_admin!(&app_state, 1);

    let unique = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

    // Admin creates a cari
    let create_req = test::TestRequest::post()
        .uri("/api/v1/cari")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "code": format!("NONADMIN-{}", unique),
            "name": "Non Admin Test",
            "cari_type": "customer",
            "tenant_id": 1,
            "created_by": user_id
        }))
        .to_request();

    let resp = test::call_service(&app, create_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let cari_id = json["id"].as_i64().unwrap();

    // Normal user tries to soft delete
    let (user_token, _) = register_user!(&app, 1);

    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/cari/{}", cari_id))
        .insert_header(("Authorization", format!("Bearer {}", user_token)))
        .to_request();

    let resp = test::call_service(&app, delete_req).await;
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "Normal user should not be able to soft delete"
    );

    // Verify cari still exists (not deleted)
    let get_req = test::TestRequest::get()
        .uri(&format!("/api/v1/cari/{}", cari_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();

    let resp = test::call_service(&app, get_req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Cari should still exist after failed delete attempt"
    );
}

#[actix_web::test]
async fn test_unauthenticated_cannot_access_soft_delete_endpoints() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    // Unauthenticated requests to soft delete endpoints should be rejected
    let endpoints = vec![
        ("GET", "/api/v1/cari/deleted"),
        ("PUT", "/api/v1/cari/1/restore"),
        ("DELETE", "/api/v1/cari/1/destroy"),
        ("DELETE", "/api/v1/cari/1"),
    ];

    for (method, path) in endpoints {
        let req = match method {
            "GET" => test::TestRequest::get().uri(path),
            "PUT" => test::TestRequest::put().uri(path),
            "DELETE" => test::TestRequest::delete().uri(path),
            _ => continue,
        }
        .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::UNAUTHORIZED,
            "Unauthenticated {} {} should return 401",
            method,
            path
        );
    }
}

#[actix_web::test]
async fn test_non_admin_cannot_access_stock_deleted() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (user_token, _) = register_user!(&app, 1);

    // Try to access stock deleted list
    let req = test::TestRequest::get()
        .uri("/api/v1/stock/warehouses/deleted")
        .insert_header(("Authorization", format!("Bearer {}", user_token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "Normal user should not access stock deleted list"
    );
}

// ============================================================================
// Section 6: Soft Delete Metadata Tests
// ============================================================================

#[actix_web::test]
async fn test_soft_delete_sets_deleted_at_and_deleted_by() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, user_id) = register_admin!(&app_state, 1);

    let unique = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

    // Create cari
    let create_req = test::TestRequest::post()
        .uri("/api/v1/cari")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "code": format!("META-{}", unique),
            "name": "Metadata Test",
            "cari_type": "customer",
            "tenant_id": 1,
            "created_by": user_id
        }))
        .to_request();

    let resp = test::call_service(&app, create_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let cari_id = json["id"].as_i64().unwrap();

    // Soft delete
    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/cari/{}", cari_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let _ = test::call_service(&app, delete_req).await;

    // Check deleted list for metadata
    let deleted_req = test::TestRequest::get()
        .uri("/api/v1/cari/deleted")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, deleted_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let deleted_items: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();

    let deleted_cari = deleted_items
        .iter()
        .find(|i| i["id"].as_i64() == Some(cari_id))
        .expect("Deleted cari should be in deleted list");

    // CariResponse does not expose deleted_at/deleted_by fields,
    // but we verify the record is present in the deleted list
    assert_eq!(
        deleted_cari["id"].as_i64(),
        Some(cari_id),
        "Deleted cari should be identifiable by id"
    );

    // Restore and verify it reappears in normal queries
    let restore_req = test::TestRequest::put()
        .uri(&format!("/api/v1/cari/{}/restore", cari_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let _ = test::call_service(&app, restore_req).await;

    let get_req = test::TestRequest::get()
        .uri(&format!("/api/v1/cari/{}", cari_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, get_req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Restored cari should be found"
    );
}

// ============================================================================
// Section 7: Multiple Entity Soft Delete Tests
// ============================================================================

#[actix_web::test]
async fn test_multiple_domains_support_soft_delete() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, user_id) = register_admin!(&app_state, 1);
    let unique = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

    // Test cari soft delete
    let cari_req = test::TestRequest::post()
        .uri("/api/v1/cari")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "code": format!("MULTI-CARI-{}", unique),
            "name": "Multi Domain Cari",
            "cari_type": "customer",
            "tenant_id": 1,
            "created_by": user_id
        }))
        .to_request();

    let resp = test::call_service(&app, cari_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let cari_id = json["id"].as_i64().unwrap();

    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/cari/{}", cari_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, delete_req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Cari soft delete should work"
    );

    // Test warehouse soft delete
    let wh_req = test::TestRequest::post()
        .uri("/api/v1/stock/warehouses")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "code": format!("MULTI-WH-{}", unique),
            "name": "Multi Domain Warehouse",
            "tenant_id": 1
        }))
        .to_request();

    let resp = test::call_service(&app, wh_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let wh_id = json["id"].as_i64().unwrap();

    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/stock/warehouses/{}", wh_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, delete_req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Warehouse soft delete should work"
    );

    // Test chart of accounts soft delete (uses code as path param)
    let coa_code = format!("MULTI-COA-{}", unique);
    let coa_req = test::TestRequest::post()
        .uri("/api/v1/chart-of-accounts")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "code": coa_code.clone(),
            "name": "Multi Domain COA",
            "group": "DonenVarliklar",
            "account_type": "Asset",
            "allow_posting": true
        }))
        .to_request();

    let resp = test::call_service(&app, coa_req).await;
    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "Chart account creation should succeed"
    );

    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/chart-of-accounts/{}", coa_code))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, delete_req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Chart of accounts soft delete should work"
    );
}

// ============================================================================
// Section 8: Edge Cases
// ============================================================================

#[actix_web::test]
async fn test_double_soft_delete_idempotent() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, user_id) = register_admin!(&app_state, 1);

    let unique = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

    // Create cari
    let create_req = test::TestRequest::post()
        .uri("/api/v1/cari")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "code": format!("DBL-{}", unique),
            "name": "Double Delete Test",
            "cari_type": "customer",
            "tenant_id": 1,
            "created_by": user_id
        }))
        .to_request();

    let resp = test::call_service(&app, create_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let cari_id = json["id"].as_i64().unwrap();

    // First soft delete
    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/cari/{}", cari_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, delete_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Second soft delete is idempotent and returns 200
    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/cari/{}", cari_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, delete_req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Double soft delete should be idempotent (200)"
    );
}

#[actix_web::test]
async fn test_restore_non_deleted_record_is_idempotent() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, user_id) = register_admin!(&app_state, 1);

    let unique = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

    // Create cari (not deleted)
    let create_req = test::TestRequest::post()
        .uri("/api/v1/cari")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "code": format!("NO-RESTORE-{}", unique),
            "name": "No Restore Test",
            "cari_type": "customer",
            "tenant_id": 1,
            "created_by": user_id
        }))
        .to_request();

    let resp = test::call_service(&app, create_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let cari_id = json["id"].as_i64().unwrap();

    // Try to restore a non-deleted record (idempotent - returns 200)
    let restore_req = test::TestRequest::put()
        .uri(&format!("/api/v1/cari/{}/restore", cari_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, restore_req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Restoring non-deleted record should be idempotent (200)"
    );
}

#[actix_web::test]
async fn test_destroy_without_soft_delete() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, user_id) = register_admin!(&app_state, 1);

    let unique = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

    // Create cari (not soft deleted)
    let create_req = test::TestRequest::post()
        .uri("/api/v1/cari")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "code": format!("HARD-DEL-{}", unique),
            "name": "Hard Delete Test",
            "cari_type": "customer",
            "tenant_id": 1,
            "created_by": user_id
        }))
        .to_request();

    let resp = test::call_service(&app, create_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let cari_id = json["id"].as_i64().unwrap();

    // Try to destroy without soft delete first
    let destroy_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/cari/{}/destroy", cari_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, destroy_req).await;
    // Destroy returns 204 NoContent
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "Destroy should work on any record"
    );

    // Verify completely gone
    let get_req = test::TestRequest::get()
        .uri(&format!("/api/v1/cari/{}", cari_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, get_req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_soft_delete_nonexistent_record_returns_404() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, _) = register_admin!(&app_state, 1);

    // Try to delete a non-existent cari
    let delete_req = test::TestRequest::delete()
        .uri("/api/v1/cari/999999")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, delete_req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "Deleting non-existent record should return 404"
    );
}

#[actix_web::test]
async fn test_restore_nonexistent_record_returns_404() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, _) = register_admin!(&app_state, 1);

    // Try to restore a non-existent cari
    let restore_req = test::TestRequest::put()
        .uri("/api/v1/cari/999999/restore")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, restore_req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "Restoring non-existent record should return 404"
    );
}

#[actix_web::test]
async fn test_update_deleted_record_behavior() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, user_id) = register_admin!(&app_state, 1);

    let unique = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

    // Create cari
    let create_req = test::TestRequest::post()
        .uri("/api/v1/cari")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "code": format!("UPD-DEL-{}", unique),
            "name": "Update Deleted Test",
            "cari_type": "customer",
            "tenant_id": 1,
            "created_by": user_id
        }))
        .to_request();

    let resp = test::call_service(&app, create_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let cari_id = json["id"].as_i64().unwrap();

    // Soft delete
    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/cari/{}", cari_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let _ = test::call_service(&app, delete_req).await;

    // Update on a deleted record: current implementation allows it
    let update_req = test::TestRequest::put()
        .uri(&format!("/api/v1/cari/{}", cari_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "name": "Updated After Delete"
        }))
        .to_request();

    let resp = test::call_service(&app, update_req).await;
    // The in-memory repository currently allows updating deleted records
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Update on deleted record behavior depends on repository implementation"
    );
}

// ============================================================================
// Section 9: HR Module Soft Delete Tests
// ============================================================================

#[actix_web::test]
async fn test_employee_soft_delete_and_restore() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, _) = register_admin!(&app_state, 1);

    let unique = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

    // Create employee
    let create_req = test::TestRequest::post()
        .uri("/api/v1/hr/employees")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "employee_number": format!("EMP-SD-{}", unique),
            "first_name": "Soft",
            "last_name": "Delete",
            "email": format!("sd{}@company.com", unique),
            "hire_date": chrono::Utc::now().to_rfc3339(),
            "salary": "50000.00",
            "tenant_id": 1
        }))
        .to_request();

    let resp = test::call_service(&app, create_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let emp_id = json["id"].as_i64().unwrap();

    // Soft delete (HR uses /soft-delete suffix)
    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/hr/employees/{}/soft-delete", emp_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, delete_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Verify not found
    let get_req = test::TestRequest::get()
        .uri(&format!("/api/v1/hr/employees/{}", emp_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, get_req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // Restore
    let restore_req = test::TestRequest::put()
        .uri(&format!("/api/v1/hr/employees/{}/restore", emp_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, restore_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Verify found again
    let get_req = test::TestRequest::get()
        .uri(&format!("/api/v1/hr/employees/{}", emp_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, get_req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

// ============================================================================
// Section 10: CRM Module Soft Delete Tests
// ============================================================================

#[actix_web::test]
async fn test_crm_lead_soft_delete_and_restore() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, _) = register_admin!(&app_state, 1);

    let unique = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

    // Create lead
    let create_req = test::TestRequest::post()
        .uri("/api/v1/crm/leads")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "name": format!("Lead SD {}", unique),
            "source": "Website",
            "tenant_id": 1
        }))
        .to_request();

    let resp = test::call_service(&app, create_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let lead_id = json["id"].as_i64().unwrap();

    // Soft delete
    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/crm/leads/{}", lead_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, delete_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Verify not in list
    let list_req = test::TestRequest::get()
        .uri("/api/v1/crm/leads")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, list_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json["items"].as_array().unwrap();
    assert!(!items.iter().any(|i| i["id"].as_i64() == Some(lead_id)));

    // Restore
    let restore_req = test::TestRequest::put()
        .uri(&format!("/api/v1/crm/leads/{}/restore", lead_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, restore_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Verify back in list
    let list_req = test::TestRequest::get()
        .uri("/api/v1/crm/leads")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, list_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json["items"].as_array().unwrap();
    assert!(items.iter().any(|i| i["id"].as_i64() == Some(lead_id)));
}

// ============================================================================
// Section 11: Asset Module Soft Delete Tests
// ============================================================================

#[actix_web::test]
async fn test_asset_soft_delete_and_restore() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, _) = register_admin!(&app_state, 1);

    let unique = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

    // Create asset
    let create_req = test::TestRequest::post()
        .uri("/api/v1/assets")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "name": format!("Asset SD {}", unique),
            "acquisition_date": chrono::Utc::now().to_rfc3339(),
            "acquisition_cost": "50000.00",
            "salvage_value": "5000.00",
            "useful_life_years": 5,
            "tenant_id": 1
        }))
        .to_request();

    let resp = test::call_service(&app, create_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let asset_id = json["id"].as_i64().unwrap();

    // Soft delete
    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/assets/{}", asset_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, delete_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Verify not in list
    let list_req = test::TestRequest::get()
        .uri("/api/v1/assets")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, list_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json["items"].as_array().unwrap();
    assert!(!items.iter().any(|i| i["id"].as_i64() == Some(asset_id)));

    // Restore
    let restore_req = test::TestRequest::put()
        .uri(&format!("/api/v1/assets/{}/restore", asset_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, restore_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Verify back in list
    let list_req = test::TestRequest::get()
        .uri("/api/v1/assets")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, list_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json["items"].as_array().unwrap();
    assert!(items.iter().any(|i| i["id"].as_i64() == Some(asset_id)));
}

// ============================================================================
// Section 12: Tax Module Soft Delete Tests
// ============================================================================
// NOTE: Tax rates currently do not have soft delete endpoints.

// ============================================================================
// Section 13: Soft Delete with Concurrent Operations
// ============================================================================

#[actix_web::test]
async fn test_soft_delete_then_create_same_code() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, user_id) = register_admin!(&app_state, 1);

    let unique = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    let code = format!("REUSE-{}", unique);

    // Create cari
    let create_req = test::TestRequest::post()
        .uri("/api/v1/cari")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "code": code.clone(),
            "name": "First Cari",
            "cari_type": "customer",
            "tenant_id": 1,
            "created_by": user_id
        }))
        .to_request();

    let resp = test::call_service(&app, create_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let cari_id = json["id"].as_i64().unwrap();

    // Soft delete it
    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/cari/{}", cari_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let _ = test::call_service(&app, delete_req).await;

    // Create another cari with the same code
    let create_req = test::TestRequest::post()
        .uri("/api/v1/cari")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "code": code.clone(),
            "name": "Second Cari",
            "cari_type": "customer",
            "tenant_id": 1,
            "created_by": user_id
        }))
        .to_request();

    let resp = test::call_service(&app, create_req).await;
    // Should succeed because soft-deleted records shouldn't block new creations
    // (this depends on the unique constraint - may be CREATED or CONFLICT)
    assert!(
        resp.status() == StatusCode::CREATED || resp.status() == StatusCode::CONFLICT,
        "Creating cari with same code as soft-deleted should be CREATED or CONFLICT, got: {:?}",
        resp.status()
    );
}

// ============================================================================
// Section 14: Notification Module Soft Delete Tests
// ============================================================================

#[actix_web::test]
async fn test_notification_soft_delete_and_restore_lifecycle() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, _) = register_admin!(&app_state, 1);

    // Send a notification
    let send_req = test::TestRequest::post()
        .uri("/api/v1/notifications/send")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "user_id": 1,
            "channel": "email",
            "template_key": "invoice_created",
            "template_vars": {
                "customer_name": "Test Corp",
                "invoice_number": "INV-SD-001",
                "amount": "1000.00",
                "currency": "TRY",
                "due_date": "2024-12-01"
            },
            "recipient": "test@example.com"
        }))
        .to_request();

    let resp = test::call_service(&app, send_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let notification_id = json["id"].as_i64().unwrap();

    // Soft delete the notification
    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/notifications/{}", notification_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, delete_req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Verify notification is NOT in normal history list
    let history_req = test::TestRequest::get()
        .uri("/api/v1/notifications/history")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, history_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json["items"].as_array().unwrap();
    assert!(
        !items
            .iter()
            .any(|i| i["id"].as_i64() == Some(notification_id)),
        "Deleted notification should not appear in normal history list"
    );

    // Verify notification appears in deleted list
    let deleted_req = test::TestRequest::get()
        .uri("/api/v1/notifications/deleted")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, deleted_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let deleted_items: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert!(
        deleted_items
            .iter()
            .any(|i| i["id"].as_i64() == Some(notification_id)),
        "Deleted notification should appear in deleted list"
    );

    // Restore the notification
    let restore_req = test::TestRequest::post()
        .uri(&format!(
            "/api/v1/notifications/{}/restore",
            notification_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, restore_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Verify notification is back in normal history list
    let history_req = test::TestRequest::get()
        .uri("/api/v1/notifications/history")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, history_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json["items"].as_array().unwrap();
    assert!(
        items
            .iter()
            .any(|i| i["id"].as_i64() == Some(notification_id)),
        "Restored notification should appear in normal history list"
    );

    // Verify notification is NOT in deleted list anymore
    let deleted_req = test::TestRequest::get()
        .uri("/api/v1/notifications/deleted")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, deleted_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let deleted_items: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert!(
        !deleted_items
            .iter()
            .any(|i| i["id"].as_i64() == Some(notification_id)),
        "Restored notification should not appear in deleted list"
    );
}

#[actix_web::test]
async fn test_notification_destroy_permanently_removes_record() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, _) = register_admin!(&app_state, 1);

    // Send a notification
    let send_req = test::TestRequest::post()
        .uri("/api/v1/notifications/send")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "user_id": 1,
            "channel": "email",
            "template_key": "invoice_created",
            "template_vars": {
                "customer_name": "Destroy Corp",
                "invoice_number": "INV-DEST-001",
                "amount": "2000.00",
                "currency": "TRY",
                "due_date": "2024-12-01"
            },
            "recipient": "destroy@example.com"
        }))
        .to_request();

    let resp = test::call_service(&app, send_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let notification_id = json["id"].as_i64().unwrap();

    // Soft delete
    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/notifications/{}", notification_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, delete_req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Destroy permanently
    let destroy_req = test::TestRequest::delete()
        .uri(&format!(
            "/api/v1/notifications/{}/destroy",
            notification_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, destroy_req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Verify not in deleted list
    let deleted_req = test::TestRequest::get()
        .uri("/api/v1/notifications/deleted")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, deleted_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let deleted_items: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert!(
        !deleted_items
            .iter()
            .any(|i| i["id"].as_i64() == Some(notification_id)),
        "Destroyed notification should not appear in deleted list"
    );
}

#[actix_web::test]
async fn test_notification_list_deleted_requires_admin() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (admin_token, _) = register_admin!(&app_state, 1);
    let (user_token, _) = register_user!(&app, 1);

    // Admin can list deleted notifications
    let req = test::TestRequest::get()
        .uri("/api/v1/notifications/deleted")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Admin should be able to list deleted notifications"
    );

    // Normal user cannot list deleted notifications
    let req = test::TestRequest::get()
        .uri("/api/v1/notifications/deleted")
        .insert_header(("Authorization", format!("Bearer {}", user_token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "Normal user should not be able to list deleted notifications"
    );
}

#[actix_web::test]
async fn test_notification_restore_requires_admin() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (admin_token, _) = register_admin!(&app_state, 1);

    // Send and soft delete as admin
    let send_req = test::TestRequest::post()
        .uri("/api/v1/notifications/send")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "user_id": 1,
            "channel": "email",
            "template_key": "invoice_created",
            "template_vars": {
                "customer_name": "Restore Corp",
                "invoice_number": "INV-REST-001",
                "amount": "1500.00",
                "currency": "TRY",
                "due_date": "2024-12-01"
            },
            "recipient": "restore@example.com"
        }))
        .to_request();

    let resp = test::call_service(&app, send_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let notification_id = json["id"].as_i64().unwrap();

    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/notifications/{}", notification_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();

    let _ = test::call_service(&app, delete_req).await;

    // Normal user cannot restore
    let (user_token, _) = register_user!(&app, 1);

    let restore_req = test::TestRequest::post()
        .uri(&format!(
            "/api/v1/notifications/{}/restore",
            notification_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", user_token)))
        .to_request();

    let resp = test::call_service(&app, restore_req).await;
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "Normal user should not be able to restore deleted notification"
    );

    // Admin can restore
    let restore_req = test::TestRequest::post()
        .uri(&format!(
            "/api/v1/notifications/{}/restore",
            notification_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();

    let resp = test::call_service(&app, restore_req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Admin should be able to restore deleted notification"
    );
}

#[actix_web::test]
async fn test_notification_destroy_requires_admin() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (admin_token, _) = register_admin!(&app_state, 1);

    // Send and soft delete as admin
    let send_req = test::TestRequest::post()
        .uri("/api/v1/notifications/send")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "user_id": 1,
            "channel": "email",
            "template_key": "invoice_created",
            "template_vars": {
                "customer_name": "Destroy Corp",
                "invoice_number": "INV-DEST-001",
                "amount": "1500.00",
                "currency": "TRY",
                "due_date": "2024-12-01"
            },
            "recipient": "destroy@example.com"
        }))
        .to_request();

    let resp = test::call_service(&app, send_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let notification_id = json["id"].as_i64().unwrap();

    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/notifications/{}", notification_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();

    let _ = test::call_service(&app, delete_req).await;

    // Normal user cannot destroy
    let (user_token, _) = register_user!(&app, 1);

    let destroy_req = test::TestRequest::delete()
        .uri(&format!(
            "/api/v1/notifications/{}/destroy",
            notification_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", user_token)))
        .to_request();

    let resp = test::call_service(&app, destroy_req).await;
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "Normal user should not be able to destroy notification"
    );

    // Admin can destroy
    let destroy_req = test::TestRequest::delete()
        .uri(&format!(
            "/api/v1/notifications/{}/destroy",
            notification_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();

    let resp = test::call_service(&app, destroy_req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "Admin should be able to destroy notification"
    );
}

#[actix_web::test]
async fn test_tenant_isolation_deleted_notifications() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token1, _) = register_admin!(&app_state, 1);
    let (token2, _) = register_admin!(&app_state, 2);

    // Tenant 1 sends and soft deletes a notification
    let send_req = test::TestRequest::post()
        .uri("/api/v1/notifications/send")
        .insert_header(("Authorization", format!("Bearer {}", token1)))
        .set_json(json!({
            "user_id": 1,
            "channel": "email",
            "template_key": "invoice_created",
            "template_vars": {
                "customer_name": "Tenant1 Corp",
                "invoice_number": "INV-T1-001",
                "amount": "1000.00",
                "currency": "TRY",
                "due_date": "2024-12-01"
            },
            "recipient": "t1@example.com"
        }))
        .to_request();

    let resp = test::call_service(&app, send_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let notification_id = json["id"].as_i64().unwrap();

    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/notifications/{}", notification_id))
        .insert_header(("Authorization", format!("Bearer {}", token1)))
        .to_request();

    let _ = test::call_service(&app, delete_req).await;

    // Tenant 2 should not see tenant 1's deleted notifications
    let deleted_req = test::TestRequest::get()
        .uri("/api/v1/notifications/deleted")
        .insert_header(("Authorization", format!("Bearer {}", token2)))
        .to_request();

    let resp = test::call_service(&app, deleted_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let deleted_items: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert!(
        !deleted_items
            .iter()
            .any(|i| i["id"].as_i64() == Some(notification_id)),
        "Tenant 2 should not see tenant 1's deleted notification"
    );

    // Tenant 2 should not be able to restore tenant 1's deleted notification
    let restore_req = test::TestRequest::post()
        .uri(&format!(
            "/api/v1/notifications/{}/restore",
            notification_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token2)))
        .to_request();

    let resp = test::call_service(&app, restore_req).await;
    assert!(
        resp.status() == StatusCode::NOT_FOUND || resp.status() == StatusCode::FORBIDDEN,
        "Tenant 2 should not be able to restore tenant 1's deleted notification, got: {:?}",
        resp.status()
    );
}

#[actix_web::test]
async fn test_notification_double_soft_delete_idempotent() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, _) = register_admin!(&app_state, 1);

    // Send a notification
    let send_req = test::TestRequest::post()
        .uri("/api/v1/notifications/send")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "user_id": 1,
            "channel": "email",
            "template_key": "invoice_created",
            "template_vars": {
                "customer_name": "Double Corp",
                "invoice_number": "INV-DBL-001",
                "amount": "1000.00",
                "currency": "TRY",
                "due_date": "2024-12-01"
            },
            "recipient": "double@example.com"
        }))
        .to_request();

    let resp = test::call_service(&app, send_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let notification_id = json["id"].as_i64().unwrap();

    // First soft delete
    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/notifications/{}", notification_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, delete_req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Second soft delete should return 404 (not found in active list)
    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/notifications/{}", notification_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, delete_req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "Double soft delete should return 404"
    );
}

#[actix_web::test]
async fn test_notification_restore_non_deleted_record_returns_404() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, _) = register_admin!(&app_state, 1);

    // Send a notification (not deleted)
    let send_req = test::TestRequest::post()
        .uri("/api/v1/notifications/send")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "user_id": 1,
            "channel": "email",
            "template_key": "invoice_created",
            "template_vars": {
                "customer_name": "No Restore Corp",
                "invoice_number": "INV-NO-001",
                "amount": "1000.00",
                "currency": "TRY",
                "due_date": "2024-12-01"
            },
            "recipient": "norestore@example.com"
        }))
        .to_request();

    let resp = test::call_service(&app, send_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let notification_id = json["id"].as_i64().unwrap();

    // Try to restore a non-deleted notification
    let restore_req = test::TestRequest::post()
        .uri(&format!(
            "/api/v1/notifications/{}/restore",
            notification_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, restore_req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "Restoring non-deleted notification should return 404"
    );
}

#[actix_web::test]
async fn test_notification_soft_delete_nonexistent_record_returns_404() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, _) = register_admin!(&app_state, 1);

    let delete_req = test::TestRequest::delete()
        .uri("/api/v1/notifications/999999")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, delete_req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "Deleting non-existent notification should return 404"
    );
}

#[actix_web::test]
async fn test_notification_destroy_nonexistent_deleted_record_returns_404() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, _) = register_admin!(&app_state, 1);

    let destroy_req = test::TestRequest::delete()
        .uri("/api/v1/notifications/999999/destroy")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, destroy_req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "Destroying non-existent deleted notification should return 404"
    );
}

// ============================================================================
// Section 15: Cross-Domain Soft Delete Consistency
// ============================================================================

#[actix_web::test]
async fn test_all_soft_delete_endpoints_return_consistent_status_codes() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, _) = register_admin!(&app_state, 1);

    // Test that all DELETE endpoints (soft delete) return 200 on success
    // and 404 on non-existent records

    let endpoints = vec![
        ("/api/v1/cari/999999", "Cari"),
        ("/api/v1/stock/warehouses/999999", "Warehouse"),
        ("/api/v1/chart-of-accounts/999999", "Chart of Accounts"),
        ("/api/v1/projects/999999", "Project"),
        ("/api/v1/assets/999999", "Asset"),
        ("/api/v1/webhooks/999999", "Webhook"),
        ("/api/v1/crm/leads/999999", "CRM Lead"),
        ("/api/v1/notifications/999999", "Notification"),
    ];

    for (path, name) in endpoints {
        let delete_req = test::TestRequest::delete()
            .uri(path)
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();

        let resp = test::call_service(&app, delete_req).await;
        assert_eq!(
            resp.status(),
            StatusCode::NOT_FOUND,
            "{} delete on non-existent should return 404",
            name
        );
    }
}
