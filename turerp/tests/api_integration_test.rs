//! API Integration Tests
//!
//! Run with: cargo test --test api_integration_test

use actix_web::{body::to_bytes, http::StatusCode, test, web, App, HttpResponse};
use serde_json::json;

// Import application modules
use turerp::api::{auth_configure, users_configure};
use turerp::app::create_app_state;
use turerp::config::Config;
use turerp::middleware::JwtAuthMiddleware;
use turerp::utils::jwt::JwtService;

/// Configure all routes (auth + users) in a single scope
fn configure_all_routes(cfg: &mut web::ServiceConfig) {
    auth_configure(cfg);
    users_configure(cfg);
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
    let jwt_service = create_test_jwt_service_from_config();

    let app = test::init_service(
        App::new()
            .wrap(JwtAuthMiddleware::new(jwt_service))
            .app_data(app_state.auth_service.clone())
            .app_data(app_state.user_service.clone())
            .app_data(app_state.jwt_service.clone())
            .service(web::scope("/api").configure(configure_all_routes)),
    )
    .await;

    // Register and get token
    let register_req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(json!({
            "username": "createuser",
            "email": "createuser@test.com",
            "full_name": "Create User",
            "password": "Password123!",
            "tenant_id": 1
        }))
        .to_request();

    let resp = test::call_service(&app, register_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let access_token = json["tokens"]["access_token"].as_str().unwrap();

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

    // Without auth token, should return 401
    let req = test::TestRequest::get().uri("/api/users").to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_users_list_authorized() {
    let app_state = create_test_app_state();
    let jwt_service = create_test_jwt_service_from_config();

    let app = test::init_service(
        App::new()
            .wrap(JwtAuthMiddleware::new(jwt_service))
            .app_data(app_state.auth_service.clone())
            .app_data(app_state.user_service.clone())
            .app_data(app_state.jwt_service.clone())
            .service(web::scope("/api").configure(configure_all_routes)),
    )
    .await;

    // Register and get token
    let register_req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(json!({
            "username": "listuser",
            "email": "listuser@test.com",
            "full_name": "List User",
            "password": "Password123!",
            "tenant_id": 1
        }))
        .to_request();

    let resp = test::call_service(&app, register_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let access_token = json["tokens"]["access_token"].as_str().unwrap();

    // List users with auth token
    let req = test::TestRequest::get()
        .uri("/api/users")
        .insert_header(("Authorization", format!("Bearer {}", access_token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.is_array());
}

#[actix_web::test]
async fn test_users_get_by_id_authorized() {
    let app_state = create_test_app_state();
    let jwt_service = create_test_jwt_service_from_config();

    let app = test::init_service(
        App::new()
            .wrap(JwtAuthMiddleware::new(jwt_service))
            .app_data(app_state.auth_service.clone())
            .app_data(app_state.user_service.clone())
            .app_data(app_state.jwt_service.clone())
            .service(web::scope("/api").configure(configure_all_routes)),
    )
    .await;

    // Register and get token
    let register_req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(json!({
            "username": "getbyiduser",
            "email": "getbyid@test.com",
            "full_name": "Get By ID User",
            "password": "Password123!",
            "tenant_id": 1
        }))
        .to_request();

    let resp = test::call_service(&app, register_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let access_token = json["tokens"]["access_token"].as_str().unwrap();

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

    // Get by ID with auth token
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
    let jwt_service = create_test_jwt_service_from_config();

    let app = test::init_service(
        App::new()
            .wrap(JwtAuthMiddleware::new(jwt_service))
            .app_data(app_state.auth_service.clone())
            .app_data(app_state.user_service.clone())
            .app_data(app_state.jwt_service.clone())
            .service(web::scope("/api").configure(configure_all_routes)),
    )
    .await;

    // Register and get token
    let register_req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(json!({
            "username": "updateuser",
            "email": "updateuser@test.com",
            "full_name": "Update User",
            "password": "Password123!",
            "tenant_id": 1
        }))
        .to_request();

    let resp = test::call_service(&app, register_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let access_token = json["tokens"]["access_token"].as_str().unwrap();

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

    // Update the user with auth token
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
    let jwt_service = create_test_jwt_service_from_config();

    let app = test::init_service(
        App::new()
            .wrap(JwtAuthMiddleware::new(jwt_service))
            .app_data(app_state.auth_service.clone())
            .app_data(app_state.user_service.clone())
            .app_data(app_state.jwt_service.clone())
            .service(web::scope("/api").configure(configure_all_routes)),
    )
    .await;

    // Register and get token
    let register_req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(json!({
            "username": "deleteuser",
            "email": "deleteuser@test.com",
            "full_name": "Delete User",
            "password": "Password123!",
            "tenant_id": 1
        }))
        .to_request();

    let resp = test::call_service(&app, register_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let access_token = json["tokens"]["access_token"].as_str().unwrap();

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

    // Delete the user with auth token
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

    // /api/auth/me should return 401 without authentication
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
