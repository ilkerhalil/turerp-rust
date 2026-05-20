//! Chart of Accounts CRUD Integration Tests

use actix_web::{body::to_bytes, http::StatusCode, test};
use serde_json::json;

mod common;
use common::*;

// ============================================================================
// CRUD Tests
// ============================================================================

#[actix_web::test]
async fn test_create_chart_account_success() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/chart-of-accounts",
        &token,
    )
    .set_json(json!({
        "code": "100",
        "name": "Cash",
        "group": "DonenVarliklar",
        "parent_code": null,
        "account_type": "Asset",
        "allow_posting": true
    }))
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["code"], "100");
    assert_eq!(json["name"], "Cash");
    assert_eq!(json["group"], "DonenVarliklar");
    assert_eq!(json["account_type"], "Asset");
    assert_eq!(json["allow_posting"], true);
    assert!(json["id"].is_number());
}

#[actix_web::test]
async fn test_list_chart_accounts_paginated() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    // Create 3 chart accounts
    for i in 1..=3 {
        let req = auth_request(
            actix_web::http::Method::POST,
            "/api/v1/chart-of-accounts",
            &token,
        )
        .set_json(json!({
            "code": format!("10{}", i),
            "name": format!("Account {}", i),
            "group": "DonenVarliklar",
            "account_type": "Asset",
            "allow_posting": true
        }))
        .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    // List with pagination
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/chart-of-accounts?page=1&per_page=2",
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
async fn test_list_chart_accounts_by_group() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    // Create asset account
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/chart-of-accounts",
        &token,
    )
    .set_json(json!({
        "code": "100",
        "name": "Cash",
        "group": "DonenVarliklar",
        "account_type": "Asset",
        "allow_posting": true
    }))
    .to_request();
    test::call_service(&app, req).await;

    // Create liability account
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/chart-of-accounts",
        &token,
    )
    .set_json(json!({
        "code": "200",
        "name": "Accounts Payable",
        "group": "KisaVadeliYabanciKaynaklar",
        "account_type": "Liability",
        "allow_posting": true
    }))
    .to_request();
    test::call_service(&app, req).await;

    // Filter by group
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/chart-of-accounts?group=DonenVarliklar",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["group"], "DonenVarliklar");
}

#[actix_web::test]
async fn test_get_chart_account_success() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/chart-of-accounts",
        &token,
    )
    .set_json(json!({
        "code": "GET-100",
        "name": "Get Test",
        "group": "DonenVarliklar",
        "account_type": "Asset",
        "allow_posting": true
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    assert_eq!(create_resp.status(), StatusCode::CREATED);

    let get_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/chart-of-accounts/GET-100",
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);

    let body = to_bytes(get_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["code"], "GET-100");
    assert_eq!(json["name"], "Get Test");
}

#[actix_web::test]
async fn test_get_chart_account_not_found() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/chart-of-accounts/NONEXISTENT",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_update_chart_account_success() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/chart-of-accounts",
        &token,
    )
    .set_json(json!({
        "code": "UPD-100",
        "name": "Original Name",
        "group": "DonenVarliklar",
        "account_type": "Asset",
        "allow_posting": true
    }))
    .to_request();
    test::call_service(&app, create_req).await;

    let update_req = auth_request(
        actix_web::http::Method::PUT,
        "/api/v1/chart-of-accounts/UPD-100",
        &token,
    )
    .set_json(json!({
        "name": "Updated Name",
        "is_active": false,
        "allow_posting": false
    }))
    .to_request();
    let update_resp = test::call_service(&app, update_req).await;
    assert_eq!(update_resp.status(), StatusCode::OK);

    let body = to_bytes(update_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["name"], "Updated Name");
    assert_eq!(json["is_active"], false);
    assert_eq!(json["allow_posting"], false);
    assert_eq!(json["code"], "UPD-100");
}

// ============================================================================
// Soft Delete Tests
// ============================================================================

#[actix_web::test]
async fn test_delete_and_restore_chart_account() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/chart-of-accounts",
        &token,
    )
    .set_json(json!({
        "code": "DEL-100",
        "name": "Delete Test",
        "group": "DonenVarliklar",
        "account_type": "Asset",
        "allow_posting": true
    }))
    .to_request();
    test::call_service(&app, create_req).await;

    // Soft delete
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        "/api/v1/chart-of-accounts/DEL-100",
        &token,
    )
    .to_request();
    let del_resp = test::call_service(&app, del_req).await;
    assert_eq!(del_resp.status(), StatusCode::OK);

    // Verify deleted - should return 404
    let get_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/chart-of-accounts/DEL-100",
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);

    // Restore
    let restore_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/chart-of-accounts/DEL-100/restore",
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::OK);

    // Verify restored
    let get_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/chart-of-accounts/DEL-100",
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn test_list_deleted_chart_accounts() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/chart-of-accounts",
        &token,
    )
    .set_json(json!({
        "code": "LST-DEL",
        "name": "List Deleted Test",
        "group": "DonenVarliklar",
        "account_type": "Asset",
        "allow_posting": true
    }))
    .to_request();
    test::call_service(&app, create_req).await;

    // Delete
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        "/api/v1/chart-of-accounts/LST-DEL",
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    // List deleted
    let list_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/chart-of-accounts/deleted",
        &token,
    )
    .to_request();
    let list_resp = test::call_service(&app, list_req).await;
    assert_eq!(list_resp.status(), StatusCode::OK);

    let body = to_bytes(list_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json.as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["code"], "LST-DEL");
}

#[actix_web::test]
async fn test_destroy_chart_account_permanently() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/chart-of-accounts",
        &token,
    )
    .set_json(json!({
        "code": "DEST-100",
        "name": "Destroy Test",
        "group": "DonenVarliklar",
        "account_type": "Asset",
        "allow_posting": true
    }))
    .to_request();
    test::call_service(&app, create_req).await;

    // Soft delete first
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        "/api/v1/chart-of-accounts/DEST-100",
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    // Permanently destroy
    let destroy_req = auth_request(
        actix_web::http::Method::DELETE,
        "/api/v1/chart-of-accounts/DEST-100/destroy",
        &token,
    )
    .to_request();
    let destroy_resp = test::call_service(&app, destroy_req).await;
    assert_eq!(destroy_resp.status(), StatusCode::NO_CONTENT);

    // Should not be restorable
    let restore_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/chart-of-accounts/DEST-100/restore",
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// Additional Endpoint Tests
// ============================================================================

#[actix_web::test]
async fn test_get_account_tree() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/chart-of-accounts/tree",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.is_array());
}

#[actix_web::test]
async fn test_get_trial_balance() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/chart-of-accounts/trial-balance",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.is_array());
}

// ============================================================================
// Authorization Tests
// ============================================================================

#[actix_web::test]
async fn test_chart_account_unauthorized() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/chart-of-accounts")
        .set_json(json!({
            "code": "UNAUTH",
            "name": "Unauthorized",
            "group": "DonenVarliklar",
            "account_type": "Asset",
            "allow_posting": true
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
