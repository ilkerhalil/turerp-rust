//! e-Fatura Integration Tests

use actix_web::{body::to_bytes, http::StatusCode, test};
use serde_json::json;

mod common;
use common::*;

// ============================================================================
// CRUD Tests
// ============================================================================

#[actix_web::test]
async fn test_create_efatura_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(actix_web::http::Method::POST, "/api/v1/efatura", &token)
        .set_json(json!({
            "invoice_id": 42,
            "profile_id": "TemelFatura"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["invoice_id"], 42);
    assert_eq!(json["profile_id"], "TemelFatura");
    assert_eq!(json["status"], "Draft");
    assert!(json["id"].is_number());
    assert!(json["uuid"].is_string());
}

#[actix_web::test]
async fn test_get_efatura_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/efatura", &token)
        .set_json(json!({
            "invoice_id": 1,
            "profile_id": "Ihracat"
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/efatura/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);

    let body = to_bytes(get_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);
    assert_eq!(json["profile_id"], "Ihracat");
}

#[actix_web::test]
async fn test_get_efatura_not_found() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/efatura/99999",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_send_efatura() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/efatura", &token)
        .set_json(json!({
            "invoice_id": 2,
            "profile_id": "TemelFatura"
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let send_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/efatura/{}/send", id),
        &token,
    )
    .to_request();
    let send_resp = test::call_service(&app, send_req).await;
    assert_eq!(send_resp.status(), StatusCode::OK);

    let body = to_bytes(send_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);
    assert_eq!(json["status"], "Sent");
}

#[actix_web::test]
async fn test_cancel_efatura() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/efatura", &token)
        .set_json(json!({
            "invoice_id": 3,
            "profile_id": "TemelFatura"
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let cancel_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/efatura/{}/cancel", id),
        &token,
    )
    .set_json(json!({ "reason": "Customer request" }))
    .to_request();
    let cancel_resp = test::call_service(&app, cancel_req).await;
    assert_eq!(cancel_resp.status(), StatusCode::OK);

    let body = to_bytes(cancel_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);
    assert_eq!(json["status"], "Cancelled");
}

// ============================================================================
// Authorization Tests
// ============================================================================

#[actix_web::test]
async fn test_efatura_unauthorized() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/efatura")
        .set_json(json!({
            "invoice_id": 1,
            "profile_id": "TemelFatura"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_efatura_normal_user_forbidden() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_user!(&app, 1);

    let req = auth_request(actix_web::http::Method::POST, "/api/v1/efatura", &token)
        .set_json(json!({
            "invoice_id": 1,
            "profile_id": "TemelFatura"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}
