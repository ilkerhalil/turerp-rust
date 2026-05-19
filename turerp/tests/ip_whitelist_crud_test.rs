//! IP Whitelist CRUD Integration Tests

use actix_web::{body::to_bytes, http::StatusCode, test};
use serde_json::json;

mod common;
use common::*;

// ============================================================================
// CRUD Tests
// ============================================================================

#[actix_web::test]
async fn test_add_ip_whitelist_success() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/ip-whitelist",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "ip_address": "192.168.1.1",
        "description": "Office network",
        "is_active": true
    }))
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["ip_address"], "192.168.1.1");
    assert_eq!(json["description"], "Office network");
    assert_eq!(json["is_active"], true);
    assert!(json["id"].is_number());
}

#[actix_web::test]
async fn test_list_ip_whitelist() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    for i in 1..=3 {
        let req = auth_request(
            actix_web::http::Method::POST,
            "/api/v1/ip-whitelist",
            &token,
        )
        .set_json(json!({
            "tenant_id": 1,
            "ip_address": format!("192.168.1.{}", i),
            "description": format!("Network {}", i),
            "is_active": true
        }))
        .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    let req =
        auth_request(actix_web::http::Method::GET, "/api/v1/ip-whitelist", &token).to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json.as_array().unwrap();
    assert_eq!(items.len(), 3);
}

#[actix_web::test]
async fn test_get_ip_whitelist_success() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/ip-whitelist",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "ip_address": "10.0.0.1",
        "description": "VPN",
        "is_active": true
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/ip-whitelist/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);

    let body = to_bytes(get_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);
    assert_eq!(json["ip_address"], "10.0.0.1");
}

#[actix_web::test]
async fn test_get_ip_whitelist_not_found() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/ip-whitelist/99999",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_update_ip_whitelist_success() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/ip-whitelist",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "ip_address": "172.16.0.1",
        "description": "Original",
        "is_active": true
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let update_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/ip-whitelist/{}", id),
        &token,
    )
    .set_json(json!({
        "ip_address": "172.16.0.2",
        "description": "Updated",
        "is_active": false
    }))
    .to_request();
    let update_resp = test::call_service(&app, update_req).await;
    assert_eq!(update_resp.status(), StatusCode::OK);

    let body = to_bytes(update_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["ip_address"], "172.16.0.2");
    assert_eq!(json["description"], "Updated");
    assert_eq!(json["is_active"], false);
}

#[actix_web::test]
async fn test_remove_ip_whitelist_success() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/ip-whitelist",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "ip_address": "192.168.100.1",
        "description": "To be removed",
        "is_active": true
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/ip-whitelist/{}", id),
        &token,
    )
    .to_request();
    let del_resp = test::call_service(&app, del_req).await;
    assert_eq!(del_resp.status(), StatusCode::NO_CONTENT);

    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/ip-whitelist/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// Authorization Tests
// ============================================================================

#[actix_web::test]
async fn test_add_ip_whitelist_unauthorized() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/ip-whitelist")
        .set_json(json!({
            "tenant_id": 1,
            "ip_address": "192.168.1.1",
            "description": "Unauthorized",
            "is_active": true
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_update_ip_whitelist_not_found() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::PUT,
        "/api/v1/ip-whitelist/99999",
        &token,
    )
    .set_json(json!({
        "ip_address": "192.168.1.1",
        "description": "Not Found",
        "is_active": false
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
