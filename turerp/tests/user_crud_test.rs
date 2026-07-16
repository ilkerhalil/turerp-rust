//! User CRUD Integration Tests

use actix_web::{body::to_bytes, http::StatusCode, test, web, App};
use serde_json::json;

use crate::common::*;

use turerp::api::{auth_configure, v1_users_configure};

fn build_test_app_with_users(
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
        .wrap(turerp::middleware::JwtAuthMiddleware::new(jwt))
        .app_data(web::Data::new(state.clone()))
        .app_data(state.auth.auth_service.clone())
        .app_data(state.auth.user_service.clone())
        .app_data(state.auth.jwt_service.clone())
        .app_data(state.i18n.clone())
        .service(
            web::scope("/api")
                .configure(auth_configure)
                .configure(v1_users_configure),
        )
}

// ============================================================================
// CRUD Tests
// ============================================================================

#[actix_web::test]
async fn test_create_user_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_users(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let username = format!("user_{}", uuid::Uuid::new_v4());
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/users", &token)
        .set_json(json!({
            "username": username,
            "email": format!("{}@test.com", username),
            "full_name": "Test User",
            "password": "Password123!",
            "tenant_id": 1,
            "role": "user"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["id"].is_number());
    assert_eq!(json["username"], username);
    assert_eq!(json["role"], "user");
    assert_eq!(json["is_active"], true);
}

#[actix_web::test]
async fn test_get_user_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_users(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let username = format!("user_{}", uuid::Uuid::new_v4());
    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/users", &token)
        .set_json(json!({
            "username": username,
            "email": format!("{}@test.com", username),
            "full_name": "Get Test",
            "password": "Password123!",
            "tenant_id": 1,
            "role": "user"
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/users/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);

    let body = to_bytes(get_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);
    assert_eq!(json["username"], username);
}

#[actix_web::test]
async fn test_get_user_not_found() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_users(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req =
        auth_request(actix_web::http::Method::GET, "/api/v1/users/99999", &token).to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_list_users_paginated() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_users(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    for i in 1..=3 {
        let username = format!("user_{}_{}", i, uuid::Uuid::new_v4());
        let req = auth_request(actix_web::http::Method::POST, "/api/v1/users", &token)
            .set_json(json!({
                "username": username,
                "email": format!("{}@test.com", username),
                "full_name": format!("User {}", i),
                "password": "Password123!",
                "tenant_id": 1,
                "role": "user"
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/users?page=1&per_page=2",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["items"].as_array().unwrap().len(), 2);
    assert_eq!(json["total"], 4);
    assert_eq!(json["page"], 1);
    assert_eq!(json["per_page"], 2);
}

#[actix_web::test]
async fn test_update_user_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_users(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let username = format!("user_{}", uuid::Uuid::new_v4());
    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/users", &token)
        .set_json(json!({
            "username": username,
            "email": format!("{}@test.com", username),
            "full_name": "Original Name",
            "password": "Password123!",
            "tenant_id": 1,
            "role": "user"
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let update_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/users/{}", id),
        &token,
    )
    .set_json(json!({
        "full_name": "Updated Name",
        "is_active": false
    }))
    .to_request();
    let update_resp = test::call_service(&app, update_req).await;
    assert_eq!(update_resp.status(), StatusCode::OK);

    let body = to_bytes(update_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["full_name"], "Updated Name");
    assert_eq!(json["is_active"], false);
}

// ============================================================================
// Soft Delete Tests
// ============================================================================

#[actix_web::test]
async fn test_delete_and_restore_user() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_users(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let username = format!("user_{}", uuid::Uuid::new_v4());
    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/users", &token)
        .set_json(json!({
            "username": username,
            "email": format!("{}@test.com", username),
            "full_name": "Delete Test",
            "password": "Password123!",
            "tenant_id": 1,
            "role": "user"
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Soft delete
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/users/{}", id),
        &token,
    )
    .to_request();
    let del_resp = test::call_service(&app, del_req).await;
    assert_eq!(del_resp.status(), StatusCode::OK);

    let body = to_bytes(del_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["message"].is_string());

    // Verify deleted - should return 404
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/users/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);

    // Restore
    let restore_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/users/{}/restore", id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::OK);

    let body = to_bytes(restore_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);
    assert_eq!(json["username"], username);

    // Verify restored
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/users/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn test_list_deleted_users() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_users(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let username = format!("user_{}", uuid::Uuid::new_v4());
    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/users", &token)
        .set_json(json!({
            "username": username,
            "email": format!("{}@test.com", username),
            "full_name": "List Deleted Test",
            "password": "Password123!",
            "tenant_id": 1,
            "role": "user"
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Delete
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/users/{}", id),
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    // List deleted
    let list_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/users/deleted",
        &token,
    )
    .to_request();
    let list_resp = test::call_service(&app, list_req).await;
    assert_eq!(list_resp.status(), StatusCode::OK);

    let body = to_bytes(list_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json.as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["id"], id);
}

#[actix_web::test]
async fn test_destroy_user_permanently() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_users(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let username = format!("user_{}", uuid::Uuid::new_v4());
    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/users", &token)
        .set_json(json!({
            "username": username,
            "email": format!("{}@test.com", username),
            "full_name": "Destroy Test",
            "password": "Password123!",
            "tenant_id": 1,
            "role": "user"
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Soft delete first
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/users/{}", id),
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    // Permanently destroy
    let destroy_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/users/{}/destroy", id),
        &token,
    )
    .to_request();
    let destroy_resp = test::call_service(&app, destroy_req).await;
    assert_eq!(destroy_resp.status(), StatusCode::OK);

    let body = to_bytes(destroy_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["message"].is_string());

    // Should not be restorable
    let restore_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/users/{}/restore", id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// Authorization Tests
// ============================================================================

#[actix_web::test]
async fn test_user_unauthorized() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_users(&state)).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/users")
        .set_json(json!({
            "username": "unauthorized",
            "email": "unauthorized@test.com",
            "full_name": "Unauthorized",
            "password": "Password123!",
            "tenant_id": 1
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_user_normal_user_forbidden() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_users(&state)).await;
    let (token, _user_id) = register_user!(&app, 1);

    let req = auth_request(actix_web::http::Method::POST, "/api/v1/users", &token)
        .set_json(json!({
            "username": "forbidden",
            "email": "forbidden@test.com",
            "full_name": "Forbidden",
            "password": "Password123!",
            "tenant_id": 1,
            "role": "user"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}
