//! Sales CRUD Integration Tests

use actix_web::{body::to_bytes, http::StatusCode, test};
use serde_json::json;

mod common;
use common::*;

// ============================================================================
// Sales Order CRUD Tests
// ============================================================================

#[actix_web::test]
async fn test_create_sales_order_success() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let now = chrono::Utc::now();
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/sales/orders",
        &token,
    )
    .set_json(json!({
        "cari_id": 1,
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
async fn test_list_sales_orders_paginated() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let now = chrono::Utc::now();
    for i in 1..=3 {
        let req = auth_request(
            actix_web::http::Method::POST,
            "/api/v1/sales/orders",
            &token,
        )
        .set_json(json!({
            "cari_id": 1,
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
        "/api/v1/sales/orders?page=1&per_page=2",
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
async fn test_get_sales_order_by_status() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let now = chrono::Utc::now();
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/sales/orders",
        &token,
    )
    .set_json(json!({
        "cari_id": 1,
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
        "/api/v1/sales/orders/status/Draft",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json["items"].as_array().unwrap();
    assert!(!items.is_empty());
    assert_eq!(items[0]["status"], "Draft");
}

#[actix_web::test]
async fn test_get_sales_order_success() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let now = chrono::Utc::now();
    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/sales/orders",
        &token,
    )
    .set_json(json!({
        "cari_id": 1,
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
        &format!("/api/v1/sales/orders/{}", id),
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
async fn test_get_sales_order_not_found() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/sales/orders/99999",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_update_sales_order_status() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let now = chrono::Utc::now();
    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/sales/orders",
        &token,
    )
    .set_json(json!({
        "cari_id": 1,
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
        &format!("/api/v1/sales/orders/{}/status", id),
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
// Soft Delete Tests
// ============================================================================

#[actix_web::test]
async fn test_delete_and_restore_sales_order() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let now = chrono::Utc::now();
    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/sales/orders",
        &token,
    )
    .set_json(json!({
        "cari_id": 1,
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
        &format!("/api/v1/sales/orders/{}", id),
        &token,
    )
    .to_request();
    let del_resp = test::call_service(&app, del_req).await;
    assert_eq!(del_resp.status(), StatusCode::OK);

    // Verify deleted - should return 404
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/sales/orders/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);

    // Restore
    let restore_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/sales/orders/{}/restore", id),
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
        &format!("/api/v1/sales/orders/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn test_list_deleted_sales_orders() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let now = chrono::Utc::now();
    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/sales/orders",
        &token,
    )
    .set_json(json!({
        "cari_id": 1,
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
        &format!("/api/v1/sales/orders/{}", id),
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    // List deleted
    let list_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/sales/orders/deleted",
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
async fn test_destroy_sales_order_permanently() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let now = chrono::Utc::now();
    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/sales/orders",
        &token,
    )
    .set_json(json!({
        "cari_id": 1,
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
        &format!("/api/v1/sales/orders/{}", id),
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    // Permanently destroy
    let destroy_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/sales/orders/{}/destroy", id),
        &token,
    )
    .to_request();
    let destroy_resp = test::call_service(&app, destroy_req).await;
    assert_eq!(destroy_resp.status(), StatusCode::NO_CONTENT);

    // Should not be restorable
    let restore_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/sales/orders/{}/restore", id),
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
async fn test_sales_order_unauthorized() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/sales/orders")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_create_sales_order_unauthorized() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;

    let now = chrono::Utc::now();
    let req = test::TestRequest::post()
        .uri("/api/v1/sales/orders")
        .set_json(json!({
            "cari_id": 1,
            "order_date": now.to_rfc3339(),
            "lines": [{
                "description": "Unauthorized test",
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

// ============================================================================
// Quotation CRUD Tests
// ============================================================================

#[actix_web::test]
async fn test_create_quotation_success() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let valid_until = chrono::Utc::now() + chrono::Duration::days(30);
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/sales/quotations",
        &token,
    )
    .set_json(json!({
        "cari_id": 1,
        "company_id": 1,
        "tenant_id": 1,
        "valid_until": valid_until.to_rfc3339(),
        "lines": [{
            "description": "Quotation item",
            "quantity": "5.00",
            "unit_price": "200.00",
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
    assert!(json["quotation_number"].is_string());
}

#[actix_web::test]
async fn test_list_quotations_paginated() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let valid_until = chrono::Utc::now() + chrono::Duration::days(30);
    for i in 1..=3 {
        let req = auth_request(
            actix_web::http::Method::POST,
            "/api/v1/sales/quotations",
            &token,
        )
        .set_json(json!({
            "cari_id": 1,
            "company_id": 1,
            "tenant_id": 1,
            "valid_until": valid_until.to_rfc3339(),
            "lines": [{
                "description": format!("Quote item {}", i),
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
        "/api/v1/sales/quotations?page=1&per_page=2",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["items"].as_array().unwrap().len(), 2);
}

#[actix_web::test]
async fn test_get_quotation_success() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let valid_until = chrono::Utc::now() + chrono::Duration::days(30);
    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/sales/quotations",
        &token,
    )
    .set_json(json!({
        "cari_id": 1,
        "company_id": 1,
        "tenant_id": 1,
        "valid_until": valid_until.to_rfc3339(),
        "lines": [{
            "description": "Get quote test",
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

    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/sales/quotations/{}", id),
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
async fn test_get_quotation_not_found() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/sales/quotations/99999",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_delete_and_restore_quotation() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let valid_until = chrono::Utc::now() + chrono::Duration::days(30);
    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/sales/quotations",
        &token,
    )
    .set_json(json!({
        "cari_id": 1,
        "company_id": 1,
        "tenant_id": 1,
        "valid_until": valid_until.to_rfc3339(),
        "lines": [{
            "description": "Delete quote test",
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
        &format!("/api/v1/sales/quotations/{}", id),
        &token,
    )
    .to_request();
    let del_resp = test::call_service(&app, del_req).await;
    assert_eq!(del_resp.status(), StatusCode::OK);

    // Verify deleted
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/sales/quotations/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);

    // Restore
    let restore_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/sales/quotations/{}/restore", id),
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
        &format!("/api/v1/sales/quotations/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn test_quotation_unauthorized() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/sales/quotations")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
