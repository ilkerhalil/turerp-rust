//! API Key CRUD Integration Tests

use actix_web::{body::to_bytes, http::StatusCode, test};
use serde_json::json;

mod common;
use common::*;

// ============================================================================
// CRUD Tests
// ============================================================================

#[actix_web::test]
async fn test_create_api_key_success() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(actix_web::http::Method::POST, "/api/v1/api-keys", &token)
        .set_json(json!({
            "name": "Test API Key",
            "tenant_id": 1,
            "user_id": _user_id,
            "scopes": ["all"],
            "expires_at": null
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["api_key"]["name"], "Test API Key");
    assert_eq!(json["api_key"]["tenant_id"], 1);
    assert_eq!(json["api_key"]["user_id"], _user_id);
    assert!(json["api_key"]["id"].is_number());
    assert!(json["plain_key"].as_str().unwrap().starts_with("tuk_"));
}

#[actix_web::test]
async fn test_list_api_keys_paginated() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, user_id) = register_admin(&state, 1).await;

    for i in 1..=3 {
        let req = auth_request(actix_web::http::Method::POST, "/api/v1/api-keys", &token)
            .set_json(json!({
                "name": format!("Key {}", i),
                "tenant_id": 1,
                "user_id": user_id,
                "scopes": ["all"],
                "expires_at": null
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/api-keys/tenant/1/paginated?page=1&per_page=2",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["items"].as_array().unwrap().len(), 2);
    assert_eq!(json["total"], 3);
    assert_eq!(json["page"], 1);
    assert_eq!(json["per_page"], 2);
}

#[actix_web::test]
async fn test_get_api_key_success() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/api-keys", &token)
        .set_json(json!({
            "name": "Get Test Key",
            "tenant_id": 1,
            "user_id": user_id,
            "scopes": ["all"],
            "expires_at": null
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["api_key"]["id"].as_i64().unwrap();

    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/api-keys/{}/tenant/1", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);

    let body = to_bytes(get_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);
    assert_eq!(json["name"], "Get Test Key");
}

#[actix_web::test]
async fn test_get_api_key_not_found() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/api-keys/99999/tenant/1",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_update_api_key_success() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/api-keys", &token)
        .set_json(json!({
            "name": "Update Test Key",
            "tenant_id": 1,
            "user_id": user_id,
            "scopes": ["all"],
            "expires_at": null
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["api_key"]["id"].as_i64().unwrap();

    let update_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/api-keys/{}/tenant/1", id),
        &token,
    )
    .set_json(json!({
        "name": "Updated Key Name",
        "is_active": false,
        "scopes": ["all"],
        "expires_at": null
    }))
    .to_request();
    let update_resp = test::call_service(&app, update_req).await;
    assert_eq!(update_resp.status(), StatusCode::OK);

    let body = to_bytes(update_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["name"], "Updated Key Name");
    assert_eq!(json["is_active"], false);
}

// ============================================================================
// Soft Delete Tests
// ============================================================================

#[actix_web::test]
async fn test_soft_delete_and_restore_api_key() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/api-keys", &token)
        .set_json(json!({
            "name": "Soft Delete Test Key",
            "tenant_id": 1,
            "user_id": user_id,
            "scopes": ["all"],
            "expires_at": null
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["api_key"]["id"].as_i64().unwrap();

    let soft_del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/api-keys/{}/tenant/1/soft", id),
        &token,
    )
    .to_request();
    let soft_del_resp = test::call_service(&app, soft_del_req).await;
    assert_eq!(soft_del_resp.status(), StatusCode::NO_CONTENT);

    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/api-keys/{}/tenant/1", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);

    let restore_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/api-keys/{}/tenant/1/restore", id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::NO_CONTENT);

    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/api-keys/{}/tenant/1", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn test_list_deleted_api_keys() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/api-keys", &token)
        .set_json(json!({
            "name": "List Deleted Test Key",
            "tenant_id": 1,
            "user_id": user_id,
            "scopes": ["all"],
            "expires_at": null
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["api_key"]["id"].as_i64().unwrap();

    let soft_del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/api-keys/{}/tenant/1/soft", id),
        &token,
    )
    .to_request();
    test::call_service(&app, soft_del_req).await;

    let list_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/api-keys/tenant/1/deleted",
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
async fn test_destroy_api_key_permanently() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/api-keys", &token)
        .set_json(json!({
            "name": "Destroy Test Key",
            "tenant_id": 1,
            "user_id": user_id,
            "scopes": ["all"],
            "expires_at": null
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["api_key"]["id"].as_i64().unwrap();

    let soft_del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/api-keys/{}/tenant/1/soft", id),
        &token,
    )
    .to_request();
    test::call_service(&app, soft_del_req).await;

    let destroy_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/api-keys/{}/tenant/1/destroy", id),
        &token,
    )
    .to_request();
    let destroy_resp = test::call_service(&app, destroy_req).await;
    assert_eq!(destroy_resp.status(), StatusCode::NO_CONTENT);

    let restore_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/api-keys/{}/tenant/1/restore", id),
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
async fn test_create_api_key_unauthorized() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/api-keys")
        .set_json(json!({
            "name": "Unauthorized Key",
            "tenant_id": 1,
            "user_id": 1,
            "scopes": ["all"],
            "expires_at": null
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_update_api_key_not_found() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::PUT,
        "/api/v1/api-keys/99999/tenant/1",
        &token,
    )
    .set_json(json!({
        "name": "Not Found",
        "is_active": false,
        "scopes": ["all"],
        "expires_at": null
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
