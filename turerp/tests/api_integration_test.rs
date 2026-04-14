//! API Integration Tests
//!
//! Run with: cargo test --test api_integration_test

use actix_web::{body::to_bytes, http::StatusCode, test, web, App, HttpResponse};
use serde_json::json;

// Import application modules
use turerp::api::{
    auth_configure, users_configure, v1_accounting_configure, v1_assets_configure,
    v1_cari_configure, v1_crm_configure, v1_feature_flags_configure, v1_hr_configure,
    v1_invoice_configure, v1_manufacturing_configure, v1_product_variants_configure,
    v1_project_configure, v1_purchase_requests_configure, v1_sales_configure, v1_stock_configure,
    v1_tenant_configure,
};
use turerp::app::create_app_state;
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
        .configure(v1_purchase_requests_configure);
}

/// Create app state with default config for testing
fn create_test_app_state() -> turerp::app::AppState {
    let config = Config::default();
    create_app_state(&config)
}

/// Create JWT service for testing from config
fn create_test_jwt_service_from_config() -> JwtService {
    let config = Config::default();
    JwtService::new(
        config.jwt.secret.clone(),
        config.jwt.access_token_expiration,
        config.jwt.refresh_token_expiration,
    )
}

/// Build a test app with all services and JWT middleware
fn build_full_test_app(
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
        .app_data(state.assets_service.clone())
        .app_data(state.feature_service.clone())
        .app_data(state.product_service.clone())
        .app_data(state.purchase_service.clone())
        .service(
            web::scope("/api")
                .configure(configure_all_routes)
                .configure(configure_v1_routes),
        )
}

/// Helper macro to register an admin user and return (access_token, user_id, app)
/// Usage: `let (token, user_id) = register_admin!(&app, 1);`
macro_rules! register_admin {
    ($app:expr, $tenant_id:expr) => {{
        let username = format!("admin_{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
        let req = test::TestRequest::post()
            .uri("/api/auth/register")
            .set_json(json!({
                "username": username,
                "email": format!("{}@test.com", username),
                "full_name": "Admin User",
                "password": "Password123!",
                "tenant_id": $tenant_id,
                "role": "admin"
            }))
            .to_request();
        let resp = test::call_service($app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED, "Admin registration failed");
        let body = to_bytes(resp.into_body()).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let access_token = json["tokens"]["access_token"].as_str().unwrap().to_string();
        let user_id = json["user"]["id"].as_i64().unwrap();
        (access_token, user_id)
    }};
}

/// Helper macro to register a normal (non-admin) user and return (access_token, user_id)
/// Usage: `let (token, user_id) = register_user!(&app, 1);`
macro_rules! register_user {
    ($app:expr, $tenant_id:expr) => {{
        let username = format!("user_{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
        let req = test::TestRequest::post()
            .uri("/api/auth/register")
            .set_json(json!({
                "username": username,
                "email": format!("{}@test.com", username),
                "full_name": "Normal User",
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
// Health Check Tests
// ============================================================================

#[actix_web::test]
async fn test_health_check() {
    let app = test::init_service(App::new().route("/health", web::get().to(health_check))).await;

    let req = test::TestRequest::get().uri("/health").to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ok");
}

async fn health_check() -> actix_web::Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(json!({
        "status": "ok",
        "service": "turerp-erp"
    })))
}

// ============================================================================
// Auth Tests (existing)
// ============================================================================

#[actix_web::test]
async fn test_auth_register() {
    let app_state = create_test_app_state();

    let app = test::init_service(
        App::new()
            .app_data(app_state.auth_service.clone())
            .app_data(app_state.user_service.clone())
            .service(web::scope("/api").configure(auth_configure)),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(json!({
            "username": "testuser",
            "email": "test@example.com",
            "full_name": "Test User",
            "password": "Password123!",
            "tenant_id": 1
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["user"]["username"], "testuser");
    assert!(json["tokens"]["access_token"].is_string());
}

#[actix_web::test]
async fn test_auth_register_validation_error() {
    let app_state = create_test_app_state();

    let app = test::init_service(
        App::new()
            .app_data(app_state.auth_service.clone())
            .app_data(app_state.user_service.clone())
            .service(web::scope("/api").configure(auth_configure)),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(json!({
            "username": "t"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn test_auth_login() {
    let app_state = create_test_app_state();

    let app = test::init_service(
        App::new()
            .app_data(app_state.auth_service.clone())
            .app_data(app_state.user_service.clone())
            .service(web::scope("/api").configure(auth_configure)),
    )
    .await;

    // First register a user
    let register_req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(json!({
            "username": "logintest",
            "email": "login@example.com",
            "full_name": "Login Test",
            "password": "Password123!",
            "tenant_id": 1
        }))
        .to_request();

    let _ = test::call_service(&app, register_req).await;

    // Now try to login
    let login_req = test::TestRequest::post()
        .uri("/api/auth/login?tenant_id=1")
        .set_json(json!({
            "username": "logintest",
            "password": "Password123!"
        }))
        .to_request();

    let resp = test::call_service(&app, login_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["tokens"]["access_token"].is_string());
}

#[actix_web::test]
async fn test_auth_login_invalid_credentials() {
    let app_state = create_test_app_state();

    let app = test::init_service(
        App::new()
            .app_data(app_state.auth_service.clone())
            .app_data(app_state.user_service.clone())
            .service(web::scope("/api").configure(auth_configure)),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/api/auth/login?tenant_id=1")
        .set_json(json!({
            "username": "nonexistent",
            "password": "wrongpassword"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_auth_me_unauthorized() {
    let app_state = create_test_app_state();
    let jwt_service = create_test_jwt_service_from_config();

    let app = test::init_service(
        App::new()
            .wrap(JwtAuthMiddleware::new(jwt_service))
            .app_data(app_state.auth_service.clone())
            .app_data(app_state.user_service.clone())
            .app_data(app_state.jwt_service.clone())
            .service(web::scope("/api").configure(auth_configure)),
    )
    .await;

    let req = test::TestRequest::get().uri("/api/auth/me").to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_auth_me_with_valid_token() {
    let app_state = create_test_app_state();
    let jwt_service = create_test_jwt_service_from_config();

    let app = test::init_service(
        App::new()
            .wrap(JwtAuthMiddleware::new(jwt_service))
            .app_data(app_state.auth_service.clone())
            .app_data(app_state.user_service.clone())
            .app_data(app_state.jwt_service.clone())
            .service(web::scope("/api").configure(auth_configure)),
    )
    .await;

    // First register a user
    let register_req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(json!({
            "username": "metest",
            "email": "metest@example.com",
            "full_name": "Me Test",
            "password": "Password123!",
            "tenant_id": 1
        }))
        .to_request();

    let resp = test::call_service(&app, register_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let access_token = json["tokens"]["access_token"].as_str().unwrap();

    // Now get current user with valid token
    let me_req = test::TestRequest::get()
        .uri("/api/auth/me")
        .insert_header(("Authorization", format!("Bearer {}", access_token)))
        .to_request();

    let resp = test::call_service(&app, me_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["username"], "metest");
}

// ============================================================================
// User Tests (existing)
// ============================================================================

#[actix_web::test]
async fn test_users_create_unauthorized() {
    let app_state = create_test_app_state();
    let jwt_service = create_test_jwt_service_from_config();

    let app = test::init_service(
        App::new()
            .wrap(JwtAuthMiddleware::new(jwt_service))
            .app_data(app_state.user_service.clone())
            .service(web::scope("/api").configure(users_configure)),
    )
    .await;

    // Without auth token, should return 401
    let req = test::TestRequest::post()
        .uri("/api/users")
        .set_json(json!({
            "username": "newuser",
            "email": "newuser@example.com",
            "full_name": "New User",
            "password": "Password123!",
            "tenant_id": 1
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_users_create_authorized() {
    let app_state = create_test_app_state();

    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (access_token, _) = register_admin!(&app, 1);

    // Create user with auth token
    let req = test::TestRequest::post()
        .uri("/api/users")
        .insert_header(("Authorization", format!("Bearer {}", access_token)))
        .set_json(json!({
            "username": "newuser",
            "email": "newuser@example.com",
            "full_name": "New User",
            "password": "Password123!",
            "tenant_id": 1
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["username"], "newuser");
}

#[actix_web::test]
async fn test_users_list_authorized() {
    let app_state = create_test_app_state();

    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (access_token, _) = register_admin!(&app, 1);

    let req = test::TestRequest::get()
        .uri("/api/users")
        .insert_header(("Authorization", format!("Bearer {}", access_token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["items"].is_array());
}

#[actix_web::test]
async fn test_users_list_unauthorized() {
    let app_state = create_test_app_state();
    let jwt_service = create_test_jwt_service_from_config();

    let app = test::init_service(
        App::new()
            .wrap(JwtAuthMiddleware::new(jwt_service))
            .app_data(app_state.user_service.clone())
            .service(web::scope("/api").configure(users_configure)),
    )
    .await;

    let req = test::TestRequest::get().uri("/api/users").to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_users_get_by_id_authorized() {
    let app_state = create_test_app_state();

    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (access_token, _) = register_admin!(&app, 1);

    // Create a user
    let create_req = test::TestRequest::post()
        .uri("/api/users")
        .insert_header(("Authorization", format!("Bearer {}", access_token)))
        .set_json(json!({
            "username": "getbyid",
            "email": "getbyid@example.com",
            "full_name": "Get By ID",
            "password": "Password123!",
            "tenant_id": 1
        }))
        .to_request();

    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let user_id = created["id"].as_i64().unwrap();

    // Get by ID
    let req = test::TestRequest::get()
        .uri(&format!("/api/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", access_token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["username"], "getbyid");
}

#[actix_web::test]
async fn test_users_update_authorized() {
    let app_state = create_test_app_state();

    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (access_token, _) = register_admin!(&app, 1);

    // Create a user
    let create_req = test::TestRequest::post()
        .uri("/api/users")
        .insert_header(("Authorization", format!("Bearer {}", access_token)))
        .set_json(json!({
            "username": "updatetest",
            "email": "update@example.com",
            "full_name": "Update Test",
            "password": "Password123!",
            "tenant_id": 1
        }))
        .to_request();

    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let user_id = created["id"].as_i64().unwrap();

    // Update the user
    let update_req = test::TestRequest::put()
        .uri(&format!("/api/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", access_token)))
        .set_json(json!({
            "full_name": "Updated Name"
        }))
        .to_request();

    let resp = test::call_service(&app, update_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["full_name"], "Updated Name");
}

#[actix_web::test]
async fn test_users_delete_authorized() {
    let app_state = create_test_app_state();

    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (access_token, _) = register_admin!(&app, 1);

    // Create a user
    let create_req = test::TestRequest::post()
        .uri("/api/users")
        .insert_header(("Authorization", format!("Bearer {}", access_token)))
        .set_json(json!({
            "username": "deletetest",
            "email": "delete@example.com",
            "full_name": "Delete Test",
            "password": "Password123!",
            "tenant_id": 1
        }))
        .to_request();

    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let user_id = created["id"].as_i64().unwrap();

    // Delete the user
    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", access_token)))
        .to_request();

    let resp = test::call_service(&app, delete_req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Verify user is deleted
    let get_req = test::TestRequest::get()
        .uri(&format!("/api/users/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", access_token)))
        .to_request();

    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// Cari (Customer/Vendor) Module Tests
// ============================================================================

#[actix_web::test]
async fn test_cari_crud() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, user_id) = register_admin!(&app, 1);

    // Create cari
    let create_req = test::TestRequest::post()
        .uri("/api/v1/cari")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "code": "CUST001",
            "name": "Test Customer",
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
    assert_eq!(json["code"], "CUST001");
    assert_eq!(json["name"], "Test Customer");

    // Get all cari
    let list_req = test::TestRequest::get()
        .uri("/api/v1/cari")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, list_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["items"].is_array());

    // Get cari by ID
    let get_req = test::TestRequest::get()
        .uri(&format!("/api/v1/cari/{}", cari_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, get_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["code"], "CUST001");

    // Update cari
    let update_req = test::TestRequest::put()
        .uri(&format!("/api/v1/cari/{}", cari_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "name": "Updated Customer",
            "email": "updated@test.com"
        }))
        .to_request();

    let resp = test::call_service(&app, update_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["name"], "Updated Customer");

    // Delete cari
    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/cari/{}", cari_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, delete_req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Verify deletion
    let get_req = test::TestRequest::get()
        .uri(&format!("/api/v1/cari/{}", cari_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, get_req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_cari_search() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, user_id) = register_admin!(&app, 1);

    // Create a cari first
    let create_req = test::TestRequest::post()
        .uri("/api/v1/cari")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "code": "VENDOR001",
            "name": "Acme Supplies",
            "cari_type": "vendor",
            "tenant_id": 1,
            "created_by": user_id
        }))
        .to_request();

    let _ = test::call_service(&app, create_req).await;

    // Search
    let search_req = test::TestRequest::get()
        .uri("/api/v1/cari/search?q=Acme")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, search_req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn test_cari_write_requires_admin() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    // Register a normal user (not admin)
    let (token, user_id) = register_user!(&app, 1);

    // Try to create cari - should be forbidden (403)
    let create_req = test::TestRequest::post()
        .uri("/api/v1/cari")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "code": "CUST002",
            "name": "Should Fail",
            "cari_type": "customer",
            "tenant_id": 1,
            "created_by": user_id
        }))
        .to_request();

    let resp = test::call_service(&app, create_req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_web::test]
async fn test_cari_read_allows_authenticated() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    // Admin creates a cari
    let (admin_token, admin_id) = register_admin!(&app, 1);

    let create_req = test::TestRequest::post()
        .uri("/api/v1/cari")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "code": "CUST003",
            "name": "Readable Customer",
            "cari_type": "customer",
            "tenant_id": 1,
            "created_by": admin_id
        }))
        .to_request();

    let _ = test::call_service(&app, create_req).await;

    // Normal user should be able to read
    let (user_token, _) = register_user!(&app, 1);

    let list_req = test::TestRequest::get()
        .uri("/api/v1/cari")
        .insert_header(("Authorization", format!("Bearer {}", user_token)))
        .to_request();

    let resp = test::call_service(&app, list_req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

// ============================================================================
// Stock Module Tests
// ============================================================================

#[actix_web::test]
async fn test_stock_warehouse_crud() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, _) = register_admin!(&app, 1);

    // Create warehouse
    let create_req = test::TestRequest::post()
        .uri("/api/v1/stock/warehouses")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "code": "WH001",
            "name": "Main Warehouse",
            "address": "123 Storage St",
            "tenant_id": 1
        }))
        .to_request();

    let resp = test::call_service(&app, create_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let wh_id = json["id"].as_i64().unwrap();
    assert_eq!(json["code"], "WH001");

    // List warehouses
    let list_req = test::TestRequest::get()
        .uri("/api/v1/stock/warehouses")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, list_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Get warehouse by ID
    let get_req = test::TestRequest::get()
        .uri(&format!("/api/v1/stock/warehouses/{}", wh_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, get_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Update warehouse
    let update_req = test::TestRequest::put()
        .uri(&format!("/api/v1/stock/warehouses/{}", wh_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "name": "Updated Warehouse"
        }))
        .to_request();

    let resp = test::call_service(&app, update_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Delete warehouse
    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/stock/warehouses/{}", wh_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, delete_req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[actix_web::test]
async fn test_stock_movement_create() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, user_id) = register_admin!(&app, 1);

    // Create a warehouse first
    let wh_req = test::TestRequest::post()
        .uri("/api/v1/stock/warehouses")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "code": "WH002",
            "name": "Movement Warehouse",
            "tenant_id": 1
        }))
        .to_request();

    let wh_resp = test::call_service(&app, wh_req).await;
    let wh_body = to_bytes(wh_resp.into_body()).await.unwrap();
    let wh_json: serde_json::Value = serde_json::from_slice(&wh_body).unwrap();
    let wh_id = wh_json["id"].as_i64().unwrap();

    // Create stock movement
    let move_req = test::TestRequest::post()
        .uri("/api/v1/stock/movements")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "warehouse_id": wh_id,
            "product_id": 1,
            "movement_type": "Purchase",
            "quantity": "100.00",
            "created_by": user_id
        }))
        .to_request();

    let resp = test::call_service(&app, move_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
}

// ============================================================================
// Invoice Module Tests
// ============================================================================

#[actix_web::test]
async fn test_invoice_crud() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, user_id) = register_admin!(&app, 1);

    // Create a cari first (invoices need cari_id)
    let cari_req = test::TestRequest::post()
        .uri("/api/v1/cari")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "code": "INV-CUST",
            "name": "Invoice Customer",
            "cari_type": "customer",
            "tenant_id": 1,
            "created_by": user_id
        }))
        .to_request();

    let cari_resp = test::call_service(&app, cari_req).await;
    let cari_body = to_bytes(cari_resp.into_body()).await.unwrap();
    let cari_json: serde_json::Value = serde_json::from_slice(&cari_body).unwrap();
    let cari_id = cari_json["id"].as_i64().unwrap();

    // Create invoice
    let now = chrono::Utc::now();
    let create_req = test::TestRequest::post()
        .uri("/api/v1/invoices")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "invoice_type": "SalesInvoice",
            "cari_id": cari_id,
            "issue_date": now.to_rfc3339(),
            "due_date": (now + chrono::Duration::days(30)).to_rfc3339(),
            "currency": "TRY",
            "tenant_id": 1,
            "lines": [{
                "description": "Test item",
                "quantity": "1.00",
                "unit_price": "100.00",
                "tax_rate": "18.00",
                "discount_rate": "0.00"
            }]
        }))
        .to_request();

    let resp = test::call_service(&app, create_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let invoice_id = json["id"].as_i64().unwrap();
    assert_eq!(json["status"], "Draft");

    // List invoices
    let list_req = test::TestRequest::get()
        .uri("/api/v1/invoices")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, list_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Get invoice by ID
    let get_req = test::TestRequest::get()
        .uri(&format!("/api/v1/invoices/{}", invoice_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, get_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Update invoice status
    let status_req = test::TestRequest::put()
        .uri(&format!("/api/v1/invoices/{}/status", invoice_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({ "status": "Sent" }))
        .to_request();

    let resp = test::call_service(&app, status_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Delete invoice
    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/invoices/{}", invoice_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, delete_req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[actix_web::test]
async fn test_invoice_write_requires_admin() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, _) = register_user!(&app, 1);

    let now = chrono::Utc::now();
    let create_req = test::TestRequest::post()
        .uri("/api/v1/invoices")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "invoice_type": "SalesInvoice",
            "cari_id": 1,
            "issue_date": now.to_rfc3339(),
            "due_date": (now + chrono::Duration::days(30)).to_rfc3339(),
            "currency": "TRY",
            "tenant_id": 1,
            "lines": [{
                "description": "Test item",
                "quantity": "1.00",
                "unit_price": "100.00",
                "tax_rate": "18.00",
                "discount_rate": "0.00"
            }]
        }))
        .to_request();

    let resp = test::call_service(&app, create_req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

// ============================================================================
// Sales Module Tests
// ============================================================================

#[actix_web::test]
async fn test_sales_order_crud() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, user_id) = register_admin!(&app, 1);

    // Create a cari first
    let cari_req = test::TestRequest::post()
        .uri("/api/v1/cari")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "code": "SALES-CUST",
            "name": "Sales Customer",
            "cari_type": "customer",
            "tenant_id": 1,
            "created_by": user_id
        }))
        .to_request();

    let cari_resp = test::call_service(&app, cari_req).await;
    let cari_body = to_bytes(cari_resp.into_body()).await.unwrap();
    let cari_json: serde_json::Value = serde_json::from_slice(&cari_body).unwrap();
    let cari_id = cari_json["id"].as_i64().unwrap();

    // Create sales order
    let now = chrono::Utc::now();
    let create_req = test::TestRequest::post()
        .uri("/api/v1/sales/orders")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "cari_id": cari_id,
            "order_date": now.to_rfc3339(),
            "tenant_id": 1,
            "lines": [{
                "description": "Test product",
                "quantity": "10.00",
                "unit_price": "50.00",
                "tax_rate": "18.00",
                "discount_rate": "0.00"
            }]
        }))
        .to_request();

    let resp = test::call_service(&app, create_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let order_id = json["id"].as_i64().unwrap();
    assert_eq!(json["status"], "Draft");

    // List sales orders
    let list_req = test::TestRequest::get()
        .uri("/api/v1/sales/orders")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, list_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Get order by ID
    let get_req = test::TestRequest::get()
        .uri(&format!("/api/v1/sales/orders/{}", order_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, get_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Delete sales order
    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/sales/orders/{}", order_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, delete_req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[actix_web::test]
async fn test_sales_quotation_crud() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, user_id) = register_admin!(&app, 1);

    // Create a cari first
    let cari_req = test::TestRequest::post()
        .uri("/api/v1/cari")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "code": "QUOTE-CUST",
            "name": "Quote Customer",
            "cari_type": "customer",
            "tenant_id": 1,
            "created_by": user_id
        }))
        .to_request();

    let cari_resp = test::call_service(&app, cari_req).await;
    let cari_body = to_bytes(cari_resp.into_body()).await.unwrap();
    let cari_json: serde_json::Value = serde_json::from_slice(&cari_body).unwrap();
    let cari_id = cari_json["id"].as_i64().unwrap();

    // Create quotation
    let create_req = test::TestRequest::post()
        .uri("/api/v1/sales/quotations")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "cari_id": cari_id,
            "valid_until": (chrono::Utc::now() + chrono::Duration::days(30)).to_rfc3339(),
            "tenant_id": 1,
            "lines": [{
                "description": "Quoted product",
                "quantity": "5.00",
                "unit_price": "200.00",
                "tax_rate": "18.00",
                "discount_rate": "0.00"
            }]
        }))
        .to_request();

    let resp = test::call_service(&app, create_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "Draft");
}

// ============================================================================
// HR Module Tests
// ============================================================================

#[actix_web::test]
async fn test_hr_employee_crud() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, _) = register_admin!(&app, 1);

    // Create employee
    let create_req = test::TestRequest::post()
        .uri("/api/v1/hr/employees")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "employee_number": "EMP001",
            "first_name": "John",
            "last_name": "Doe",
            "email": "john.doe@company.com",
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
    assert_eq!(json["employee_number"], "EMP001");
    assert_eq!(json["status"], "Active");

    // List employees
    let list_req = test::TestRequest::get()
        .uri("/api/v1/hr/employees")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, list_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Get employee by ID
    let get_req = test::TestRequest::get()
        .uri(&format!("/api/v1/hr/employees/{}", emp_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, get_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Terminate employee
    let term_req = test::TestRequest::post()
        .uri(&format!("/api/v1/hr/employees/{}/terminate", emp_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, term_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "Terminated");
}

#[actix_web::test]
async fn test_hr_leave_types() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, _) = register_admin!(&app, 1);

    // Get leave types (seeded by default)
    let req = test::TestRequest::get()
        .uri("/api/v1/hr/leave-types")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

// ============================================================================
// Accounting Module Tests
// ============================================================================

#[actix_web::test]
async fn test_accounting_account_crud() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, _) = register_admin!(&app, 1);

    // Create account (use a unique code to avoid conflict with seeded defaults)
    let create_req = test::TestRequest::post()
        .uri("/api/v1/accounting/accounts")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "code": "9999",
            "name": "Test Account",
            "account_type": "Asset",
            "sub_type": "CurrentAsset",
            "allow_transaction": true,
            "tenant_id": 1
        }))
        .to_request();

    let resp = test::call_service(&app, create_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let account_id = json["id"].as_i64().unwrap();
    assert_eq!(json["code"], "9999");
    assert_eq!(json["name"], "Test Account");

    // List accounts
    let list_req = test::TestRequest::get()
        .uri("/api/v1/accounting/accounts")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, list_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Get account by ID
    let get_req = test::TestRequest::get()
        .uri(&format!("/api/v1/accounting/accounts/{}", account_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, get_req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn test_accounting_journal_entry() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, user_id) = register_admin!(&app, 1);

    // Create two accounts first (use unique codes to avoid conflict with seeded defaults)
    let debit_req = test::TestRequest::post()
        .uri("/api/v1/accounting/accounts")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "code": "9998",
            "name": "Test Bank",
            "account_type": "Asset",
            "sub_type": "CurrentAsset",
            "allow_transaction": true,
            "tenant_id": 1
        }))
        .to_request();

    let debit_resp = test::call_service(&app, debit_req).await;
    let debit_body = to_bytes(debit_resp.into_body()).await.unwrap();
    let debit_json: serde_json::Value = serde_json::from_slice(&debit_body).unwrap();
    let debit_account_id = debit_json["id"].as_i64().unwrap();

    let credit_req = test::TestRequest::post()
        .uri("/api/v1/accounting/accounts")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "code": "9999",
            "name": "Test Revenue",
            "account_type": "Revenue",
            "sub_type": "OperatingRevenue",
            "allow_transaction": true,
            "tenant_id": 1
        }))
        .to_request();

    let credit_resp = test::call_service(&app, credit_req).await;
    let credit_body = to_bytes(credit_resp.into_body()).await.unwrap();
    let credit_json: serde_json::Value = serde_json::from_slice(&credit_body).unwrap();
    let credit_account_id = credit_json["id"].as_i64().unwrap();

    // Create journal entry
    let now = chrono::Utc::now();
    let create_req = test::TestRequest::post()
        .uri("/api/v1/accounting/journal-entries")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "date": now.to_rfc3339(),
            "description": "Test journal entry",
            "reference": "JE-001",
            "tenant_id": 1,
            "created_by": user_id,
            "lines": [
                {
                    "account_id": debit_account_id,
                    "debit": "1000.00",
                    "credit": "0.00",
                    "description": "Debit line"
                },
                {
                    "account_id": credit_account_id,
                    "debit": "0.00",
                    "credit": "1000.00",
                    "description": "Credit line"
                }
            ]
        }))
        .to_request();

    let resp = test::call_service(&app, create_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let entry_id = json["id"].as_i64().unwrap();
    assert_eq!(json["status"], "Draft");

    // Post journal entry
    let post_req = test::TestRequest::post()
        .uri(&format!(
            "/api/v1/accounting/journal-entries/{}/post",
            entry_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, post_req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

// ============================================================================
// Project Module Tests
// ============================================================================

#[actix_web::test]
async fn test_project_crud() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, _) = register_admin!(&app, 1);

    // Create project
    let create_req = test::TestRequest::post()
        .uri("/api/v1/projects")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "name": "Test Project",
            "description": "A test project",
            "budget": "100000.00",
            "tenant_id": 1
        }))
        .to_request();

    let resp = test::call_service(&app, create_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let project_id = json["id"].as_i64().unwrap();
    assert_eq!(json["name"], "Test Project");

    // List projects
    let list_req = test::TestRequest::get()
        .uri("/api/v1/projects")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, list_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Get project by ID
    let get_req = test::TestRequest::get()
        .uri(&format!("/api/v1/projects/{}", project_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, get_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Update project status
    let status_req = test::TestRequest::put()
        .uri(&format!("/api/v1/projects/{}/status", project_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({ "status": "Active" }))
        .to_request();

    let resp = test::call_service(&app, status_req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

// ============================================================================
// Manufacturing Module Tests
// ============================================================================

#[actix_web::test]
async fn test_manufacturing_work_order() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, _) = register_admin!(&app, 1);

    // Create work order
    let create_req = test::TestRequest::post()
        .uri("/api/v1/manufacturing/work-orders")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "name": "WO-001",
            "product_id": 1,
            "quantity": "100.00",
            "priority": "Normal",
            "tenant_id": 1
        }))
        .to_request();

    let resp = test::call_service(&app, create_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["name"], "WO-001");
    assert_eq!(json["status"], "Draft");

    // List work orders
    let list_req = test::TestRequest::get()
        .uri("/api/v1/manufacturing/work-orders")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, list_req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

// ============================================================================
// CRM Module Tests
// ============================================================================

#[actix_web::test]
async fn test_crm_lead_crud() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, _) = register_admin!(&app, 1);

    // Create lead
    let create_req = test::TestRequest::post()
        .uri("/api/v1/crm/leads")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "name": "John Lead",
            "company": "Acme Corp",
            "email": "john@acme.com",
            "source": "Website",
            "tenant_id": 1
        }))
        .to_request();

    let resp = test::call_service(&app, create_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let lead_id = json["id"].as_i64().unwrap();
    assert_eq!(json["name"], "John Lead");

    // List leads
    let list_req = test::TestRequest::get()
        .uri("/api/v1/crm/leads")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, list_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Get lead by ID
    let get_req = test::TestRequest::get()
        .uri(&format!("/api/v1/crm/leads/{}", lead_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, get_req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn test_crm_opportunity_create() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, _) = register_admin!(&app, 1);

    // Create opportunity
    let create_req = test::TestRequest::post()
        .uri("/api/v1/crm/opportunities")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "name": "Big Deal",
            "value": "50000.00",
            "probability": "0.50",
            "tenant_id": 1
        }))
        .to_request();

    let resp = test::call_service(&app, create_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
}

// ============================================================================
// Tenant Module Tests
// ============================================================================

#[actix_web::test]
async fn test_tenant_crud() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, _) = register_admin!(&app, 1);

    // Create tenant
    let create_req = test::TestRequest::post()
        .uri("/api/v1/tenants")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "name": "Test Company",
            "subdomain": "testcompany"
        }))
        .to_request();

    let resp = test::call_service(&app, create_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let tenant_id = json["id"].as_i64().unwrap();
    assert_eq!(json["name"], "Test Company");

    // List tenants
    let list_req = test::TestRequest::get()
        .uri("/api/v1/tenants")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, list_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Get tenant by ID
    let get_req = test::TestRequest::get()
        .uri(&format!("/api/v1/tenants/{}", tenant_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, get_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Update tenant
    let update_req = test::TestRequest::put()
        .uri(&format!("/api/v1/tenants/{}", tenant_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "name": "Updated Company"
        }))
        .to_request();

    let resp = test::call_service(&app, update_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["name"], "Updated Company");

    // Delete tenant
    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/tenants/{}", tenant_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, delete_req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

// ============================================================================
// Assets Module Tests
// ============================================================================

#[actix_web::test]
async fn test_asset_crud() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (token, _) = register_admin!(&app, 1);

    // Create asset
    let create_req = test::TestRequest::post()
        .uri("/api/v1/assets")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "name": "Office Laptop",
            "description": "MacBook Pro",
            "serial_number": "SN-12345",
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
    assert_eq!(json["name"], "Office Laptop");

    // List assets
    let list_req = test::TestRequest::get()
        .uri("/api/v1/assets")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, list_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Get asset by ID
    let get_req = test::TestRequest::get()
        .uri(&format!("/api/v1/assets/{}", asset_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, get_req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

// ============================================================================
// Unauthenticated Access Tests
// ============================================================================

#[actix_web::test]
async fn test_business_endpoints_require_auth() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    // All these endpoints should return 401 without auth
    let protected_endpoints = vec![
        ("/api/v1/cari", "GET"),
        ("/api/v1/stock/warehouses", "GET"),
        ("/api/v1/invoices", "GET"),
        ("/api/v1/sales/orders", "GET"),
        ("/api/v1/sales/quotations", "GET"),
        ("/api/v1/hr/employees", "GET"),
        ("/api/v1/hr/leave-types", "GET"),
        ("/api/v1/accounting/accounts", "GET"),
        ("/api/v1/accounting/journal-entries", "GET"),
        ("/api/v1/projects", "GET"),
        ("/api/v1/manufacturing/work-orders", "GET"),
        ("/api/v1/crm/leads", "GET"),
        ("/api/v1/tenants", "GET"),
        ("/api/v1/assets", "GET"),
    ];

    for (path, _method) in protected_endpoints {
        let req = test::TestRequest::get().uri(path).to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::UNAUTHORIZED,
            "Endpoint {} should require authentication",
            path
        );
    }
}

// ============================================================================
// Tenant Isolation Tests
// ============================================================================

#[actix_web::test]
async fn test_cari_tenant_isolation() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    // Admin in tenant 1 creates a cari
    let (token1, user_id1) = register_admin!(&app, 1);

    let create_req = test::TestRequest::post()
        .uri("/api/v1/cari")
        .insert_header(("Authorization", format!("Bearer {}", token1)))
        .set_json(json!({
            "code": "TENANT1-CUST",
            "name": "Tenant 1 Customer",
            "cari_type": "customer",
            "tenant_id": 1,
            "created_by": user_id1
        }))
        .to_request();

    let resp = test::call_service(&app, create_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Admin in tenant 2 should not see tenant 1's cari
    let (token2, _) = register_admin!(&app, 2);

    let list_req = test::TestRequest::get()
        .uri("/api/v1/cari")
        .insert_header(("Authorization", format!("Bearer {}", token2)))
        .to_request();

    let resp = test::call_service(&app, list_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let caris = json["items"].as_array().unwrap();
    // Tenant 2 should see an empty list (no caris from tenant 1)
    assert!(caris.is_empty(), "Tenant 2 should not see tenant 1's caris");
}

#[actix_web::test]
async fn test_hr_tenant_isolation() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    // Admin in tenant 1 creates an employee
    let (token1, _) = register_admin!(&app, 1);

    let create_req = test::TestRequest::post()
        .uri("/api/v1/hr/employees")
        .insert_header(("Authorization", format!("Bearer {}", token1)))
        .set_json(json!({
            "employee_number": "T1-EMP",
            "first_name": "Tenant",
            "last_name": "One",
            "email": "t1@company.com",
            "hire_date": chrono::Utc::now().to_rfc3339(),
            "salary": "50000.00",
            "tenant_id": 1
        }))
        .to_request();

    let _ = test::call_service(&app, create_req).await;

    // Admin in tenant 2 should not see tenant 1's employees
    let (token2, _) = register_admin!(&app, 2);

    let list_req = test::TestRequest::get()
        .uri("/api/v1/hr/employees")
        .insert_header(("Authorization", format!("Bearer {}", token2)))
        .to_request();

    let resp = test::call_service(&app, list_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let employees = json["items"].as_array().unwrap();
    assert!(
        employees.is_empty(),
        "Tenant 2 should not see tenant 1's employees"
    );
}

// ============================================================================
// Authorization Tests (Admin vs User)
// ============================================================================

#[actix_web::test]
async fn test_admin_only_write_endpoints() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_full_test_app(&app_state)).await;

    let (user_token, _) = register_user!(&app, 1);

    // These write endpoints should all return 403 for non-admin users
    let write_endpoints = vec![
        ("/api/v1/cari", "POST"),
        ("/api/v1/stock/warehouses", "POST"),
        ("/api/v1/invoices", "POST"),
        ("/api/v1/sales/orders", "POST"),
        ("/api/v1/hr/employees", "POST"),
        ("/api/v1/accounting/accounts", "POST"),
        ("/api/v1/projects", "POST"),
        ("/api/v1/manufacturing/work-orders", "POST"),
        ("/api/v1/crm/leads", "POST"),
        ("/api/v1/tenants", "POST"),
        ("/api/v1/assets", "POST"),
    ];

    for (path, method) in write_endpoints {
        let req = match method {
            "POST" => test::TestRequest::post()
                .uri(path)
                .insert_header(("Authorization", format!("Bearer {}", user_token)))
                .set_json(json!({}))
                .to_request(),
            _ => panic!("Unsupported method"),
        };

        let resp = test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "Endpoint {} {} should be forbidden for non-admin users",
            method,
            path
        );
    }
}
