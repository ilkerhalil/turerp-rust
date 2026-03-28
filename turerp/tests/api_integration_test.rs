//! API Integration Tests
//!
//! Run with: cargo test --test api_integration_test

use actix_web::{body::to_bytes, http::StatusCode, test, web, App, HttpResponse};
use serde_json::json;

// Import application modules
use turerp::api::{auth_configure, users_configure};
use turerp::app::create_app_state;

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
    let app_state = create_app_state();

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
            "password": "password123",
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
    let app_state = create_app_state();

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
    let app_state = create_app_state();

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
            "password": "password123",
            "tenant_id": 1
        }))
        .to_request();

    let _ = test::call_service(&app, register_req).await;

    // Now try to login
    let login_req = test::TestRequest::post()
        .uri("/api/auth/login?tenant_id=1")
        .set_json(json!({
            "username": "logintest",
            "password": "password123"
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
    let app_state = create_app_state();

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
async fn test_users_create() {
    let app_state = create_app_state();

    let app = test::init_service(
        App::new()
            .app_data(app_state.user_service.clone())
            .service(web::scope("/api").configure(users_configure)),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/api/users")
        .set_json(json!({
            "username": "newuser",
            "email": "newuser@example.com",
            "full_name": "New User",
            "password": "password123",
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
async fn test_users_list() {
    let app_state = create_app_state();

    let app = test::init_service(
        App::new()
            .app_data(app_state.user_service.clone())
            .service(web::scope("/api").configure(users_configure)),
    )
    .await;

    let req = test::TestRequest::get().uri("/api/users").to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.is_array());
}

#[actix_web::test]
async fn test_users_get_by_id() {
    let app_state = create_app_state();

    let app = test::init_service(
        App::new()
            .app_data(app_state.user_service.clone())
            .service(web::scope("/api").configure(users_configure)),
    )
    .await;

    // First create a user
    let create_req = test::TestRequest::post()
        .uri("/api/users")
        .set_json(json!({
            "username": "getbyid",
            "email": "getbyid@example.com",
            "full_name": "Get By ID",
            "password": "password123",
            "tenant_id": 1
        }))
        .to_request();

    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let user_id = created["id"].as_i64().unwrap();

    // Now get by ID
    let req = test::TestRequest::get()
        .uri(&format!("/api/users/{}", user_id))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["username"], "getbyid");
}

#[actix_web::test]
async fn test_users_update() {
    let app_state = create_app_state();

    let app = test::init_service(
        App::new()
            .app_data(app_state.user_service.clone())
            .service(web::scope("/api").configure(users_configure)),
    )
    .await;

    // First create a user
    let create_req = test::TestRequest::post()
        .uri("/api/users")
        .set_json(json!({
            "username": "updatetest",
            "email": "update@example.com",
            "full_name": "Update Test",
            "password": "password123",
            "tenant_id": 1
        }))
        .to_request();

    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let user_id = created["id"].as_i64().unwrap();

    // Now update the user
    let update_req = test::TestRequest::put()
        .uri(&format!("/api/users/{}", user_id))
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
async fn test_users_delete() {
    let app_state = create_app_state();

    let app = test::init_service(
        App::new()
            .app_data(app_state.user_service.clone())
            .service(web::scope("/api").configure(users_configure)),
    )
    .await;

    // First create a user
    let create_req = test::TestRequest::post()
        .uri("/api/users")
        .set_json(json!({
            "username": "deletetest",
            "email": "delete@example.com",
            "full_name": "Delete Test",
            "password": "password123",
            "tenant_id": 1
        }))
        .to_request();

    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let user_id = created["id"].as_i64().unwrap();

    // Now delete the user
    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/users/{}", user_id))
        .to_request();

    let resp = test::call_service(&app, delete_req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Verify user is deleted
    let get_req = test::TestRequest::get()
        .uri(&format!("/api/users/{}", user_id))
        .to_request();

    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_auth_me_not_implemented() {
    let app_state = create_app_state();

    let app = test::init_service(
        App::new()
            .app_data(app_state.auth_service.clone())
            .app_data(app_state.user_service.clone())
            .service(web::scope("/api").configure(auth_configure)),
    )
    .await;

    let req = test::TestRequest::get().uri("/api/auth/me").to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["message"].is_string());
}
