//! Cost Center CRUD Integration Tests

use actix_web::{body::to_bytes, http::StatusCode, test};
use serde_json::json;

mod common;
use common::*;

// ============================================================================
// CRUD Tests
// ============================================================================

#[actix_web::test]
async fn test_create_cost_center_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/cost-centers",
        &token,
    )
    .set_json(json!({
        "code": "CC-001",
        "name": "Production",
        "description": "Main production cost center",
        "center_type": "Cost",
        "parent_id": null,
        "is_active": true
    }))
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["code"], "CC-001");
    assert_eq!(json["name"], "Production");
    assert_eq!(json["center_type"], "Cost");
    assert!(json["id"].is_number());
}

#[actix_web::test]
async fn test_list_cost_centers_paginated() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    // Create 3 cost centers
    for i in 1..=3 {
        let req = auth_request(
            actix_web::http::Method::POST,
            "/api/v1/cost-centers",
            &token,
        )
        .set_json(json!({
            "code": format!("CC-00{}", i),
            "name": format!("Center {}", i),
            "center_type": "Cost",
            "is_active": true
        }))
        .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    // List with pagination
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/cost-centers?page=1&per_page=2",
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
async fn test_list_cost_centers_by_type() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    // Create cost center
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/cost-centers",
        &token,
    )
    .set_json(json!({
        "code": "CC-COST",
        "name": "Cost Center",
        "center_type": "Cost",
        "is_active": true
    }))
    .to_request();
    test::call_service(&app, req).await;

    // Create profit center
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/cost-centers",
        &token,
    )
    .set_json(json!({
        "code": "CC-PROF",
        "name": "Profit Center",
        "center_type": "Profit",
        "is_active": true
    }))
    .to_request();
    test::call_service(&app, req).await;

    // Filter by cost type
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/cost-centers?center_type=Cost",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["center_type"], "Cost");
}

#[actix_web::test]
async fn test_get_cost_center_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/cost-centers",
        &token,
    )
    .set_json(json!({
        "code": "CC-GET",
        "name": "Get Test",
        "center_type": "Cost",
        "is_active": true
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/cost-centers/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);

    let body = to_bytes(get_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);
    assert_eq!(json["code"], "CC-GET");
}

#[actix_web::test]
async fn test_get_cost_center_not_found() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/cost-centers/99999",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_update_cost_center_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/cost-centers",
        &token,
    )
    .set_json(json!({
        "code": "CC-UPD",
        "name": "Original Name",
        "center_type": "Cost",
        "is_active": true
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let update_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/cost-centers/{}", id),
        &token,
    )
    .set_json(json!({
        "name": "Updated Name",
        "is_active": false
    }))
    .to_request();
    let update_resp = test::call_service(&app, update_req).await;
    assert_eq!(update_resp.status(), StatusCode::OK);

    let body = to_bytes(update_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["name"], "Updated Name");
    assert_eq!(json["is_active"], false);
    assert_eq!(json["code"], "CC-UPD");
}

// ============================================================================
// Soft Delete Tests
// ============================================================================

#[actix_web::test]
async fn test_delete_and_restore_cost_center() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/cost-centers",
        &token,
    )
    .set_json(json!({
        "code": "CC-DEL",
        "name": "Delete Test",
        "center_type": "Cost",
        "is_active": true
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Soft delete
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/cost-centers/{}", id),
        &token,
    )
    .to_request();
    let del_resp = test::call_service(&app, del_req).await;
    assert_eq!(del_resp.status(), StatusCode::NO_CONTENT);

    // Verify deleted - should return 404
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/cost-centers/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);

    // Restore
    let restore_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/cost-centers/{}/restore", id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::OK);

    let body = to_bytes(restore_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);
    assert_eq!(json["code"], "CC-DEL");

    // Verify restored
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/cost-centers/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn test_list_deleted_cost_centers() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/cost-centers",
        &token,
    )
    .set_json(json!({
        "code": "CC-LST-DEL",
        "name": "List Deleted Test",
        "center_type": "Cost",
        "is_active": true
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Delete
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/cost-centers/{}", id),
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    // List deleted
    let list_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/cost-centers/deleted",
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
async fn test_destroy_cost_center_permanently() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/cost-centers",
        &token,
    )
    .set_json(json!({
        "code": "CC-DEST",
        "name": "Destroy Test",
        "center_type": "Cost",
        "is_active": true
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Soft delete first
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/cost-centers/{}", id),
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    // Permanently destroy
    let destroy_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/cost-centers/{}/destroy", id),
        &token,
    )
    .to_request();
    let destroy_resp = test::call_service(&app, destroy_req).await;
    assert_eq!(destroy_resp.status(), StatusCode::NO_CONTENT);

    // Should not be restorable
    let restore_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/cost-centers/{}/restore", id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_bulk_restore_cost_centers() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let mut ids = Vec::new();
    for i in 1..=2 {
        let create_req = auth_request(
            actix_web::http::Method::POST,
            "/api/v1/cost-centers",
            &token,
        )
        .set_json(json!({
            "code": format!("CC-BULK-{}", i),
            "name": format!("Bulk {}", i),
            "center_type": "Cost",
            "is_active": true
        }))
        .to_request();
        let create_resp = test::call_service(&app, create_req).await;
        let body = to_bytes(create_resp.into_body()).await.unwrap();
        let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let id = create_json["id"].as_i64().unwrap();
        ids.push(id);

        // Delete each
        let del_req = auth_request(
            actix_web::http::Method::DELETE,
            &format!("/api/v1/cost-centers/{}", id),
            &token,
        )
        .to_request();
        test::call_service(&app, del_req).await;
    }

    // Bulk restore
    let restore_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/cost-centers/bulk-restore",
        &token,
    )
    .set_json(json!({ "ids": ids }))
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::OK);

    let body = to_bytes(restore_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["restored"], 2);
    assert_eq!(json["items"].as_array().unwrap().len(), 2);
    assert_eq!(json["failed"].as_array().unwrap().len(), 0);

    // Verify both are accessible
    for id in ids {
        let get_req = auth_request(
            actix_web::http::Method::GET,
            &format!("/api/v1/cost-centers/{}", id),
            &token,
        )
        .to_request();
        let get_resp = test::call_service(&app, get_req).await;
        assert_eq!(get_resp.status(), StatusCode::OK);
    }
}
