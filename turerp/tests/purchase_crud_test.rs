//! Purchase CRUD Integration Tests

use actix_web::{body::to_bytes, http::StatusCode, test};
use serde_json::json;

mod common;
use common::*;

// ============================================================================
// Purchase Order CRUD Tests
// ============================================================================

#[actix_web::test]
async fn test_create_purchase_order_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, user_id) = register_admin(&state, 1).await;
    let cari_id = seed_cari!(&app, &token, user_id, 1);

    let now = chrono::Utc::now();
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/purchase-orders",
        &token,
    )
    .set_json(json!({
        "cari_id": cari_id,
        "order_date": now.to_rfc3339(),
        "tenant_id": 1,
        "lines": [{
            "description": "Test product",
            "quantity": "10.00",
            "unit_price": "50.00",
            "tax_rate": "18.00",
            "discount_rate": "0.00"
        }]
    }))
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "Draft");
    assert!(json["id"].is_number());
    assert!(json["order_number"].is_string());
}

#[actix_web::test]
async fn test_list_purchase_orders_paginated() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, user_id) = register_admin(&state, 1).await;
    let cari_id = seed_cari!(&app, &token, user_id, 1);

    let now = chrono::Utc::now();
    for i in 1..=3 {
        let req = auth_request(
            actix_web::http::Method::POST,
            "/api/v1/purchase-orders",
            &token,
        )
        .set_json(json!({
            "cari_id": cari_id,
            "order_date": now.to_rfc3339(),
            "tenant_id": 1,
            "lines": [{
                "description": format!("Product {}", i),
                "quantity": "1.00",
                "unit_price": "100.00",
                "tax_rate": "18.00",
                "discount_rate": "0.00"
            }]
        }))
        .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/purchase-orders?page=1&per_page=2",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["items"].as_array().unwrap().len(), 2);
    assert_eq!(json["page"], 1);
    assert_eq!(json["per_page"], 2);
}

#[actix_web::test]
async fn test_get_purchase_order_by_status() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, user_id) = register_admin(&state, 1).await;
    let cari_id = seed_cari!(&app, &token, user_id, 1);

    let now = chrono::Utc::now();
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/purchase-orders",
        &token,
    )
    .set_json(json!({
        "cari_id": cari_id,
        "order_date": now.to_rfc3339(),
        "tenant_id": 1,
        "lines": [{
            "description": "Draft order",
            "quantity": "1.00",
            "unit_price": "100.00",
            "tax_rate": "18.00",
            "discount_rate": "0.00"
        }]
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/purchase-orders?status=Draft",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(!json["items"].as_array().unwrap().is_empty());
}

#[actix_web::test]
async fn test_get_purchase_order_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, user_id) = register_admin(&state, 1).await;
    let cari_id = seed_cari!(&app, &token, user_id, 1);

    let now = chrono::Utc::now();
    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/purchase-orders",
        &token,
    )
    .set_json(json!({
        "cari_id": cari_id,
        "order_date": now.to_rfc3339(),
        "tenant_id": 1,
        "lines": [{
            "description": "Get test product",
            "quantity": "5.00",
            "unit_price": "20.00",
            "tax_rate": "18.00",
            "discount_rate": "0.00"
        }]
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/purchase-orders/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);

    let body = to_bytes(get_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);
    assert_eq!(json["status"], "Draft");
}

#[actix_web::test]
async fn test_get_purchase_order_not_found() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/purchase-orders/99999",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_update_purchase_order_status() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, user_id) = register_admin(&state, 1).await;
    let cari_id = seed_cari!(&app, &token, user_id, 1);

    let now = chrono::Utc::now();
    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/purchase-orders",
        &token,
    )
    .set_json(json!({
        "cari_id": cari_id,
        "order_date": now.to_rfc3339(),
        "tenant_id": 1,
        "lines": [{
            "description": "Update test",
            "quantity": "1.00",
            "unit_price": "100.00",
            "tax_rate": "18.00",
            "discount_rate": "0.00"
        }]
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let update_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/purchase-orders/{}/status", id),
        &token,
    )
    .set_json(json!({ "status": "Approved" }))
    .to_request();
    let update_resp = test::call_service(&app, update_req).await;
    assert_eq!(update_resp.status(), StatusCode::OK);

    let body = to_bytes(update_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "Approved");
}

// ============================================================================
// Purchase Order Soft Delete Tests
// ============================================================================

#[actix_web::test]
async fn test_delete_and_restore_purchase_order() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, user_id) = register_admin(&state, 1).await;
    let cari_id = seed_cari!(&app, &token, user_id, 1);

    let now = chrono::Utc::now();
    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/purchase-orders",
        &token,
    )
    .set_json(json!({
        "cari_id": cari_id,
        "order_date": now.to_rfc3339(),
        "tenant_id": 1,
        "lines": [{
            "description": "Delete test",
            "quantity": "1.00",
            "unit_price": "100.00",
            "tax_rate": "18.00",
            "discount_rate": "0.00"
        }]
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Soft delete
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/purchase-orders/{}", id),
        &token,
    )
    .to_request();
    let del_resp = test::call_service(&app, del_req).await;
    assert_eq!(del_resp.status(), StatusCode::OK);

    // Verify deleted - should return 404
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/purchase-orders/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);

    // Restore (POST for purchase orders)
    let restore_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/purchase-orders/{}/restore", id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::OK);

    let body = to_bytes(restore_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);

    // Verify restored
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/purchase-orders/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn test_list_deleted_purchase_orders() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, user_id) = register_admin(&state, 1).await;
    let cari_id = seed_cari!(&app, &token, user_id, 1);

    let now = chrono::Utc::now();
    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/purchase-orders",
        &token,
    )
    .set_json(json!({
        "cari_id": cari_id,
        "order_date": now.to_rfc3339(),
        "tenant_id": 1,
        "lines": [{
            "description": "List deleted test",
            "quantity": "1.00",
            "unit_price": "100.00",
            "tax_rate": "18.00",
            "discount_rate": "0.00"
        }]
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Delete
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/purchase-orders/{}", id),
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    // List deleted
    let list_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/purchase-orders/deleted",
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
async fn test_destroy_purchase_order_permanently() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, user_id) = register_admin(&state, 1).await;
    let cari_id = seed_cari!(&app, &token, user_id, 1);

    let now = chrono::Utc::now();
    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/purchase-orders",
        &token,
    )
    .set_json(json!({
        "cari_id": cari_id,
        "order_date": now.to_rfc3339(),
        "tenant_id": 1,
        "lines": [{
            "description": "Destroy test",
            "quantity": "1.00",
            "unit_price": "100.00",
            "tax_rate": "18.00",
            "discount_rate": "0.00"
        }]
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Soft delete first
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/purchase-orders/{}", id),
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    // Permanently destroy
    let destroy_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/purchase-orders/{}/destroy", id),
        &token,
    )
    .to_request();
    let destroy_resp = test::call_service(&app, destroy_req).await;
    assert_eq!(destroy_resp.status(), StatusCode::NO_CONTENT);

    // Should not be restorable
    let restore_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/purchase-orders/{}/restore", id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// Purchase Request CRUD Tests
// ============================================================================

#[actix_web::test]
async fn test_create_purchase_request_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/purchase-requests",
        &token,
    )
    .set_json(json!({
        "requested_by": 1,
        "company_id": 1,
        "tenant_id": 1,
        "priority": "High",
        "department": "IT",
        "reason": "Need new laptops",
        "lines": [{
            "description": "Laptop",
            "quantity": "5.00",
            "notes": "Developer laptops"
        }]
    }))
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "Draft");
    assert_eq!(json["priority"], "High");
    assert!(json["id"].is_number());
    assert!(json["request_number"].is_string());
}

#[actix_web::test]
async fn test_list_purchase_requests_paginated() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    for i in 1..=3 {
        let req = auth_request(
            actix_web::http::Method::POST,
            "/api/v1/purchase-requests",
            &token,
        )
        .set_json(json!({
            "requested_by": 1,
            "company_id": 1,
            "tenant_id": 1,
            "priority": "Medium",
            "department": format!("Dept {}", i),
            "lines": [{
                "description": format!("Item {}", i),
                "quantity": "1.00"
            }]
        }))
        .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/purchase-requests?page=1&per_page=2",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["items"].as_array().unwrap().len(), 2);
    assert_eq!(json["page"], 1);
    assert_eq!(json["per_page"], 2);
}

#[actix_web::test]
async fn test_get_purchase_request_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/purchase-requests",
        &token,
    )
    .set_json(json!({
        "requested_by": 1,
        "company_id": 1,
        "tenant_id": 1,
        "priority": "Low",
        "department": "HR",
        "lines": [{
            "description": "Office chairs",
            "quantity": "10.00"
        }]
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/purchase-requests/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);

    let body = to_bytes(get_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);
    assert_eq!(json["priority"], "Low");
}

#[actix_web::test]
async fn test_get_purchase_request_not_found() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/purchase-requests/99999",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_update_purchase_request() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/purchase-requests",
        &token,
    )
    .set_json(json!({
        "requested_by": 1,
        "company_id": 1,
        "tenant_id": 1,
        "priority": "Low",
        "department": "Old Dept",
        "lines": [{
            "description": "Item",
            "quantity": "1.00"
        }]
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let update_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/purchase-requests/{}", id),
        &token,
    )
    .set_json(json!({
        "priority": "High",
        "department": "Updated Dept"
    }))
    .to_request();
    let update_resp = test::call_service(&app, update_req).await;
    assert_eq!(update_resp.status(), StatusCode::OK);

    let body = to_bytes(update_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["priority"], "High");
    assert_eq!(json["department"], "Updated Dept");
}

#[actix_web::test]
async fn test_approve_purchase_request() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/purchase-requests",
        &token,
    )
    .set_json(json!({
        "requested_by": 1,
        "company_id": 1,
        "tenant_id": 1,
        "priority": "Medium",
        "lines": [{
            "description": "Approval test item",
            "quantity": "2.00"
        }]
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Submit first
    let submit_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/purchase-requests/{}/submit", id),
        &token,
    )
    .to_request();
    let submit_resp = test::call_service(&app, submit_req).await;
    assert_eq!(submit_resp.status(), StatusCode::OK);

    // Approve
    let approve_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/purchase-requests/{}/approve", id),
        &token,
    )
    .to_request();
    let approve_resp = test::call_service(&app, approve_req).await;
    assert_eq!(approve_resp.status(), StatusCode::OK);

    let body = to_bytes(approve_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "Approved");
}

// ============================================================================
// Purchase Request Soft Delete Tests
// ============================================================================

#[actix_web::test]
async fn test_delete_and_restore_purchase_request() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/purchase-requests",
        &token,
    )
    .set_json(json!({
        "requested_by": 1,
        "company_id": 1,
        "tenant_id": 1,
        "priority": "Low",
        "lines": [{
            "description": "Delete request test",
            "quantity": "1.00"
        }]
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Soft delete
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/purchase-requests/{}", id),
        &token,
    )
    .to_request();
    let del_resp = test::call_service(&app, del_req).await;
    assert_eq!(del_resp.status(), StatusCode::OK);

    // Verify deleted
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/purchase-requests/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);

    // Restore (PUT for purchase requests)
    let restore_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/purchase-requests/{}/restore", id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::OK);

    let body = to_bytes(restore_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);

    // Verify restored
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/purchase-requests/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn test_list_deleted_purchase_requests() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/purchase-requests",
        &token,
    )
    .set_json(json!({
        "requested_by": 1,
        "company_id": 1,
        "tenant_id": 1,
        "priority": "Low",
        "lines": [{
            "description": "List deleted request test",
            "quantity": "1.00"
        }]
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Delete
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/purchase-requests/{}", id),
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    // List deleted
    let list_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/purchase-requests/deleted",
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
async fn test_destroy_purchase_request_permanently() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/purchase-requests",
        &token,
    )
    .set_json(json!({
        "requested_by": 1,
        "company_id": 1,
        "tenant_id": 1,
        "priority": "Low",
        "lines": [{
            "description": "Destroy request test",
            "quantity": "1.00"
        }]
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Soft delete first
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/purchase-requests/{}", id),
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    // Permanently destroy
    let destroy_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/purchase-requests/{}/destroy", id),
        &token,
    )
    .to_request();
    let destroy_resp = test::call_service(&app, destroy_req).await;
    assert_eq!(destroy_resp.status(), StatusCode::NO_CONTENT);

    // Should not be restorable
    let restore_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/purchase-requests/{}/restore", id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// Security Tests
// ============================================================================

#[actix_web::test]
async fn test_purchase_order_unauthorized() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/purchase-orders")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_purchase_request_unauthorized() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/purchase-requests")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_create_purchase_order_unauthorized() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;

    let now = chrono::Utc::now();
    let req = test::TestRequest::post()
        .uri("/api/v1/purchase-orders")
        .set_json(json!({
            "cari_id": 1,
            "order_date": now.to_rfc3339(),
            "lines": [{
                "description": "Unauthorized",
                "quantity": "1.00",
                "unit_price": "100.00",
                "tax_rate": "18.00",
                "discount_rate": "0.00"
            }]
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
