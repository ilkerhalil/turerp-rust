//! Feature Flag CRUD Integration Tests

use actix_web::{body::to_bytes, http::StatusCode, test};
use serde_json::json;

mod common;
use common::*;

// ============================================================================
// CRUD Tests
// ============================================================================

#[actix_web::test]
async fn test_create_feature_flag_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/feature-flags",
        &token,
    )
    .set_json(json!({
        "name": "test-feature-001",
        "description": "A test feature flag",
        "status": "enabled",
        "tenant_id": null
    }))
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["name"], "test-feature-001");
    assert_eq!(json["description"], "A test feature flag");
    assert_eq!(json["status"], "enabled");
    assert!(json["id"].is_number());
}

#[actix_web::test]
async fn test_list_feature_flags_paginated() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    // Create 3 feature flags
    for i in 1..=3 {
        let req = auth_request(
            actix_web::http::Method::POST,
            "/api/v1/feature-flags",
            &token,
        )
        .set_json(json!({
            "name": format!("feature-{}", i),
            "description": format!("Feature {}", i),
            "status": "disabled",
            "tenant_id": null
        }))
        .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    // List with pagination
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/feature-flags?page=1&per_page=2",
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
async fn test_get_feature_flag_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/feature-flags",
        &token,
    )
    .set_json(json!({
        "name": "get-test",
        "description": "Get test",
        "status": "enabled",
        "tenant_id": null
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/feature-flags/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);

    let body = to_bytes(get_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);
    assert_eq!(json["name"], "get-test");
}

#[actix_web::test]
async fn test_get_feature_flag_not_found() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/feature-flags/99999",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_update_feature_flag_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/feature-flags",
        &token,
    )
    .set_json(json!({
        "name": "upd-test",
        "description": "Original desc",
        "status": "disabled",
        "tenant_id": 1
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let update_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/feature-flags/{}", id),
        &token,
    )
    .set_json(json!({
        "description": "Updated desc",
        "status": "enabled"
    }))
    .to_request();
    let update_resp = test::call_service(&app, update_req).await;
    assert_eq!(update_resp.status(), StatusCode::OK);

    let body = to_bytes(update_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["description"], "Updated desc");
    assert_eq!(json["status"], "enabled");
    assert_eq!(json["name"], "upd-test");
}

// ============================================================================
// Soft Delete Tests
// ============================================================================

#[actix_web::test]
async fn test_soft_delete_and_restore_feature_flag() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/feature-flags",
        &token,
    )
    .set_json(json!({
        "name": "del-test",
        "description": "Delete test",
        "status": "disabled",
        "tenant_id": 1
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Soft delete
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/feature-flags/{}/soft", id),
        &token,
    )
    .to_request();
    let del_resp = test::call_service(&app, del_req).await;
    assert_eq!(del_resp.status(), StatusCode::OK);

    // Verify deleted - should return 404
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/feature-flags/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);

    // Restore
    let restore_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/feature-flags/{}/restore", id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::OK);

    // Verify restored
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/feature-flags/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn test_destroy_feature_flag_permanently() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/feature-flags",
        &token,
    )
    .set_json(json!({
        "name": "dest-test",
        "description": "Destroy test",
        "status": "disabled",
        "tenant_id": 1
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Soft delete first
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/feature-flags/{}/soft", id),
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    // Permanently destroy
    let destroy_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/feature-flags/{}/destroy", id),
        &token,
    )
    .to_request();
    let destroy_resp = test::call_service(&app, destroy_req).await;
    assert_eq!(destroy_resp.status(), StatusCode::OK);

    // Should not be restorable
    let restore_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/feature-flags/{}/restore", id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// Enable / Disable / Check Tests
// ============================================================================

#[actix_web::test]
async fn test_enable_disable_feature_flag() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/feature-flags",
        &token,
    )
    .set_json(json!({
        "name": "toggle-test",
        "description": "Toggle test",
        "status": "disabled",
        "tenant_id": 1
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Enable
    let enable_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/feature-flags/{}/enable", id),
        &token,
    )
    .to_request();
    let enable_resp = test::call_service(&app, enable_req).await;
    assert_eq!(enable_resp.status(), StatusCode::OK);
    let body = to_bytes(enable_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "enabled");

    // Disable
    let disable_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/feature-flags/{}/disable", id),
        &token,
    )
    .to_request();
    let disable_resp = test::call_service(&app, disable_req).await;
    assert_eq!(disable_resp.status(), StatusCode::OK);
    let body = to_bytes(disable_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "disabled");
}

#[actix_web::test]
async fn test_check_feature_enabled() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/feature-flags",
        &token,
    )
    .set_json(json!({
        "name": "check-feature",
        "description": "Check test",
        "status": "enabled",
        "tenant_id": null
    }))
    .to_request();
    test::call_service(&app, create_req).await;

    let check_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/feature-flags/check/check-feature",
        &token,
    )
    .to_request();
    let check_resp = test::call_service(&app, check_req).await;
    assert_eq!(check_resp.status(), StatusCode::OK);

    let body = to_bytes(check_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["name"], "check-feature");
    assert_eq!(json["enabled"], true);
}

// ============================================================================
// Authorization Tests
// ============================================================================

#[actix_web::test]
async fn test_feature_flag_unauthorized() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;

    // No token
    let req = test::TestRequest::post()
        .uri("/api/v1/feature-flags")
        .set_json(json!({
            "name": "unauth-test",
            "description": "Should fail",
            "status": "disabled",
            "tenant_id": null
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_feature_flag_normal_user_forbidden() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_user!(&app, 1);

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/feature-flags",
        &token,
    )
    .set_json(json!({
        "name": "forbidden-test",
        "description": "Should fail",
        "status": "disabled",
        "tenant_id": null
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    // Non-admin should get 403 Forbidden for admin-only endpoints
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}
