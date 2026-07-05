//! Manufacturing CRUD Integration Tests

use actix_web::{body::to_bytes, http::StatusCode, test};
use serde_json::json;

mod common;
use common::*;

// ============================================================================
// CRUD Tests
// ============================================================================

#[actix_web::test]
async fn test_create_work_order_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;
    let product_id = seed_product!(&app, &token, 1);

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/manufacturing/work-orders",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "name": "WO-001",
        "product_id": product_id,
        "quantity": "100.00",
        "priority": "Normal",
        "planned_start": null,
        "planned_end": null
    }))
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["name"], "WO-001");
    assert_eq!(json["product_id"], product_id);
    assert_eq!(json["status"], "Draft");
    assert!(json["id"].is_number());
}

#[actix_web::test]
async fn test_list_work_orders_paginated() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;
    let product_id = seed_product!(&app, &token, 1);

    for i in 1..=3 {
        let req = auth_request(
            actix_web::http::Method::POST,
            "/api/v1/manufacturing/work-orders",
            &token,
        )
        .set_json(json!({
            "tenant_id": 1,
            "name": format!("WO-00{}", i),
            "product_id": product_id,
            "quantity": "100.00",
            "priority": "Normal"
        }))
        .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/manufacturing/work-orders?page=1&per_page=2",
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
async fn test_get_work_order_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;
    let _product_id = seed_product!(&app, &token, 1);

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/manufacturing/work-orders",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "name": "WO-GET",
        "product_id": 1,
        "quantity": "50.00",
        "priority": "High"
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/manufacturing/work-orders/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);

    let body = to_bytes(get_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);
    assert_eq!(json["name"], "WO-GET");
}

#[actix_web::test]
async fn test_get_work_order_not_found() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/manufacturing/work-orders/99999",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_update_work_order_status_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;
    let _product_id = seed_product!(&app, &token, 1);

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/manufacturing/work-orders",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "name": "WO-UPD",
        "product_id": 1,
        "quantity": "100.00",
        "priority": "Normal"
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let update_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/manufacturing/work-orders/{}/status", id),
        &token,
    )
    .set_json(json!({ "status": "InProgress" }))
    .to_request();
    let update_resp = test::call_service(&app, update_req).await;
    assert_eq!(update_resp.status(), StatusCode::OK);

    let body = to_bytes(update_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "InProgress");
    assert_eq!(json["name"], "WO-UPD");
}

// ============================================================================
// Soft Delete Tests
// ============================================================================

#[actix_web::test]
async fn test_soft_delete_and_restore_work_order() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;
    let _product_id = seed_product!(&app, &token, 1);

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/manufacturing/work-orders",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "name": "WO-DEL",
        "product_id": 1,
        "quantity": "100.00",
        "priority": "Normal"
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Soft delete
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/manufacturing/work-orders/{}", id),
        &token,
    )
    .to_request();
    let del_resp = test::call_service(&app, del_req).await;
    assert_eq!(del_resp.status(), StatusCode::OK);

    // Verify deleted - should return 404
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/manufacturing/work-orders/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);

    // Restore
    let restore_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/manufacturing/work-orders/{}/restore", id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::OK);

    let body = to_bytes(restore_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);
    assert_eq!(json["name"], "WO-DEL");

    // Verify restored
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/manufacturing/work-orders/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn test_list_deleted_work_orders() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;
    let _product_id = seed_product!(&app, &token, 1);

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/manufacturing/work-orders",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "name": "WO-LST-DEL",
        "product_id": 1,
        "quantity": "100.00",
        "priority": "Normal"
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Soft delete
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/manufacturing/work-orders/{}", id),
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    // List deleted
    let list_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/manufacturing/work-orders/deleted",
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
async fn test_destroy_work_order_permanently() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;
    let _product_id = seed_product!(&app, &token, 1);

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/manufacturing/work-orders",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "name": "WO-DEST",
        "product_id": 1,
        "quantity": "100.00",
        "priority": "Normal"
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Soft delete first
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/manufacturing/work-orders/{}", id),
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    // Permanently destroy
    let destroy_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/manufacturing/work-orders/{}/destroy", id),
        &token,
    )
    .to_request();
    let destroy_resp = test::call_service(&app, destroy_req).await;
    assert_eq!(destroy_resp.status(), StatusCode::NO_CONTENT);

    // Should not be restorable
    let restore_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/manufacturing/work-orders/{}/restore", id),
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
async fn test_create_work_order_unauthorized() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/manufacturing/work-orders")
        .set_json(json!({
            "tenant_id": 1,
            "name": "WO-UNAUTH",
            "product_id": 1,
            "quantity": "100.00",
            "priority": "Normal"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
