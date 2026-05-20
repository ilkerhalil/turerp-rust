//! Invoice CRUD Integration Tests

use actix_web::{body::to_bytes, http::StatusCode, test};
use serde_json::json;

mod common;
use common::*;

// ============================================================================
// CRUD Tests
// ============================================================================

#[actix_web::test]
async fn test_create_invoice_success() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, user_id) = register_admin(&state, 1).await;

    // Create a cari first
    let cari_req = auth_request(actix_web::http::Method::POST, "/api/v1/cari", &token)
        .set_json(json!({
            "code": format!("CARI-{}", uuid::Uuid::new_v4()),
            "name": "Test Cari",
            "cari_type": "customer",
            "created_by": user_id,
            "tenant_id": 1,
        }))
        .to_request();
    let cari_resp = test::call_service(&app, cari_req).await;
    assert_eq!(cari_resp.status(), StatusCode::CREATED);
    let cari_body = to_bytes(cari_resp.into_body()).await.unwrap();
    let cari_json: serde_json::Value = serde_json::from_slice(&cari_body).unwrap();
    let cari_id = cari_json["id"].as_i64().unwrap();

    let req = auth_request(actix_web::http::Method::POST, "/api/v1/invoices", &token)
        .set_json(json!({
            "invoice_type": "SalesInvoice",
            "cari_id": cari_id,
            "issue_date": "2024-01-01T00:00:00Z",
            "due_date": "2024-02-01T00:00:00Z",
            "currency": "TRY",
            "tenant_id": 1,
            "lines": [
                {
                    "description": "Test Product",
                    "quantity": 2.0,
                    "unit_price": 100.0,
                    "tax_rate": 18.0,
                    "discount_rate": 0.0
                }
            ]
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["invoice_type"], "SalesInvoice");
    assert_eq!(json["cari_id"], cari_id);
    assert_eq!(json["currency"], "TRY");
    assert!(json["id"].is_number());
    let lines = json["lines"].as_array().unwrap();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["description"], "Test Product");
}

#[actix_web::test]
async fn test_list_invoices_paginated() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, user_id) = register_admin(&state, 1).await;

    let cari_req = auth_request(actix_web::http::Method::POST, "/api/v1/cari", &token)
        .set_json(json!({
            "code": format!("CARI-{}", uuid::Uuid::new_v4()),
            "name": "Test Cari",
            "cari_type": "customer",
            "created_by": user_id,
            "tenant_id": 1,
        }))
        .to_request();
    let cari_resp = test::call_service(&app, cari_req).await;
    let cari_body = to_bytes(cari_resp.into_body()).await.unwrap();
    let cari_json: serde_json::Value = serde_json::from_slice(&cari_body).unwrap();
    let cari_id = cari_json["id"].as_i64().unwrap();

    for i in 1..=3 {
        let req = auth_request(
            actix_web::http::Method::POST,
            "/api/v1/invoices",
            &token,
        )
        .set_json(json!({
            "invoice_type": "SalesInvoice",
            "cari_id": cari_id,
            "issue_date": "2024-01-01T00:00:00Z",
            "due_date": format!("2024-02-{:02}T00:00:00Z", i),
            "currency": "TRY",
            "tenant_id": 1,
            "lines": [{"description": format!("Item {}", i), "quantity": 1.0, "unit_price": 100.0, "tax_rate": 18.0, "discount_rate": 0.0}]
        }))
        .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/invoices?page=1&per_page=2",
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
async fn test_get_invoice_success() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, user_id) = register_admin(&state, 1).await;

    let cari_req = auth_request(actix_web::http::Method::POST, "/api/v1/cari", &token)
        .set_json(json!({
            "code": format!("CARI-{}", uuid::Uuid::new_v4()),
            "name": "Test Cari",
            "cari_type": "customer",
            "created_by": user_id,
            "tenant_id": 1,
        }))
        .to_request();
    let cari_resp = test::call_service(&app, cari_req).await;
    let cari_body = to_bytes(cari_resp.into_body()).await.unwrap();
    let cari_json: serde_json::Value = serde_json::from_slice(&cari_body).unwrap();
    let cari_id = cari_json["id"].as_i64().unwrap();

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/invoices",
        &token,
    )
    .set_json(json!({
        "invoice_type": "SalesInvoice",
        "cari_id": cari_id,
        "issue_date": "2024-01-01T00:00:00Z",
        "due_date": "2024-02-01T00:00:00Z",
        "currency": "TRY",
        "tenant_id": 1,
        "lines": [{"description": "Test Product", "quantity": 2.0, "unit_price": 100.0, "tax_rate": 18.0, "discount_rate": 0.0}]
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/invoices/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);

    let body = to_bytes(get_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);
    assert_eq!(json["invoice_type"], "SalesInvoice");
}

#[actix_web::test]
async fn test_get_invoice_not_found() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/invoices/99999",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_update_invoice_status() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, user_id) = register_admin(&state, 1).await;

    let cari_req = auth_request(actix_web::http::Method::POST, "/api/v1/cari", &token)
        .set_json(json!({
            "code": format!("CARI-{}", uuid::Uuid::new_v4()),
            "name": "Test Cari",
            "cari_type": "customer",
            "created_by": user_id,
            "tenant_id": 1,
        }))
        .to_request();
    let cari_resp = test::call_service(&app, cari_req).await;
    let cari_body = to_bytes(cari_resp.into_body()).await.unwrap();
    let cari_json: serde_json::Value = serde_json::from_slice(&cari_body).unwrap();
    let cari_id = cari_json["id"].as_i64().unwrap();

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/invoices",
        &token,
    )
    .set_json(json!({
        "invoice_type": "SalesInvoice",
        "cari_id": cari_id,
        "issue_date": "2024-01-01T00:00:00Z",
        "due_date": "2024-02-01T00:00:00Z",
        "currency": "TRY",
        "tenant_id": 1,
        "lines": [{"description": "Test Product", "quantity": 2.0, "unit_price": 100.0, "tax_rate": 18.0, "discount_rate": 0.0}]
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let update_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/invoices/{}/status", id),
        &token,
    )
    .set_json(json!({"status": "Approved"}))
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
async fn test_delete_and_restore_invoice() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, user_id) = register_admin(&state, 1).await;

    let cari_req = auth_request(actix_web::http::Method::POST, "/api/v1/cari", &token)
        .set_json(json!({
            "code": format!("CARI-{}", uuid::Uuid::new_v4()),
            "name": "Test Cari",
            "cari_type": "customer",
            "created_by": user_id,
            "tenant_id": 1,
        }))
        .to_request();
    let cari_resp = test::call_service(&app, cari_req).await;
    let cari_body = to_bytes(cari_resp.into_body()).await.unwrap();
    let cari_json: serde_json::Value = serde_json::from_slice(&cari_body).unwrap();
    let cari_id = cari_json["id"].as_i64().unwrap();

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/invoices",
        &token,
    )
    .set_json(json!({
        "invoice_type": "SalesInvoice",
        "cari_id": cari_id,
        "issue_date": "2024-01-01T00:00:00Z",
        "due_date": "2024-02-01T00:00:00Z",
        "currency": "TRY",
        "tenant_id": 1,
        "lines": [{"description": "Test Product", "quantity": 2.0, "unit_price": 100.0, "tax_rate": 18.0, "discount_rate": 0.0}]
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Soft delete
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/invoices/{}", id),
        &token,
    )
    .to_request();
    let del_resp = test::call_service(&app, del_req).await;
    assert_eq!(del_resp.status(), StatusCode::OK);

    // Verify deleted - should return 404
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/invoices/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);

    // Restore
    let restore_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/invoices/{}/restore", id),
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
        &format!("/api/v1/invoices/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn test_list_deleted_invoices() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, user_id) = register_admin(&state, 1).await;

    let cari_req = auth_request(actix_web::http::Method::POST, "/api/v1/cari", &token)
        .set_json(json!({
            "code": format!("CARI-{}", uuid::Uuid::new_v4()),
            "name": "Test Cari",
            "cari_type": "customer",
            "created_by": user_id,
            "tenant_id": 1,
        }))
        .to_request();
    let cari_resp = test::call_service(&app, cari_req).await;
    let cari_body = to_bytes(cari_resp.into_body()).await.unwrap();
    let cari_json: serde_json::Value = serde_json::from_slice(&cari_body).unwrap();
    let cari_id = cari_json["id"].as_i64().unwrap();

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/invoices",
        &token,
    )
    .set_json(json!({
        "invoice_type": "SalesInvoice",
        "cari_id": cari_id,
        "issue_date": "2024-01-01T00:00:00Z",
        "due_date": "2024-02-01T00:00:00Z",
        "currency": "TRY",
        "tenant_id": 1,
        "lines": [{"description": "Test Product", "quantity": 2.0, "unit_price": 100.0, "tax_rate": 18.0, "discount_rate": 0.0}]
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Delete
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/invoices/{}", id),
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    // List deleted
    let list_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/invoices/deleted",
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
async fn test_destroy_invoice_permanently() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, user_id) = register_admin(&state, 1).await;

    let cari_req = auth_request(actix_web::http::Method::POST, "/api/v1/cari", &token)
        .set_json(json!({
            "code": format!("CARI-{}", uuid::Uuid::new_v4()),
            "name": "Test Cari",
            "cari_type": "customer",
            "created_by": user_id,
            "tenant_id": 1,
        }))
        .to_request();
    let cari_resp = test::call_service(&app, cari_req).await;
    let cari_body = to_bytes(cari_resp.into_body()).await.unwrap();
    let cari_json: serde_json::Value = serde_json::from_slice(&cari_body).unwrap();
    let cari_id = cari_json["id"].as_i64().unwrap();

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/invoices",
        &token,
    )
    .set_json(json!({
        "invoice_type": "SalesInvoice",
        "cari_id": cari_id,
        "issue_date": "2024-01-01T00:00:00Z",
        "due_date": "2024-02-01T00:00:00Z",
        "currency": "TRY",
        "tenant_id": 1,
        "lines": [{"description": "Test Product", "quantity": 2.0, "unit_price": 100.0, "tax_rate": 18.0, "discount_rate": 0.0}]
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Soft delete first
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/invoices/{}", id),
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    // Permanently destroy
    let destroy_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/invoices/{}/destroy", id),
        &token,
    )
    .to_request();
    let destroy_resp = test::call_service(&app, destroy_req).await;
    assert_eq!(destroy_resp.status(), StatusCode::NO_CONTENT);

    // Should not be restorable
    let restore_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/invoices/{}/restore", id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// Search
// ============================================================================

#[actix_web::test]
async fn test_search_invoices() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;
    let (token, user_id) = register_admin(&state, 1).await;

    let cari_req = auth_request(actix_web::http::Method::POST, "/api/v1/cari", &token)
        .set_json(json!({
            "code": format!("CARI-{}", uuid::Uuid::new_v4()),
            "name": "Test Cari",
            "cari_type": "customer",
            "created_by": user_id,
            "tenant_id": 1,
        }))
        .to_request();
    let cari_resp = test::call_service(&app, cari_req).await;
    let cari_body = to_bytes(cari_resp.into_body()).await.unwrap();
    let cari_json: serde_json::Value = serde_json::from_slice(&cari_body).unwrap();
    let cari_id = cari_json["id"].as_i64().unwrap();

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/invoices",
        &token,
    )
    .set_json(json!({
        "invoice_type": "SalesInvoice",
        "cari_id": cari_id,
        "issue_date": "2024-01-01T00:00:00Z",
        "due_date": "2024-02-01T00:00:00Z",
        "currency": "TRY",
        "tenant_id": 1,
        "lines": [{"description": "Test Product", "quantity": 2.0, "unit_price": 100.0, "tax_rate": 18.0, "discount_rate": 0.0}]
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let invoice_number = create_json["invoice_number"].as_str().unwrap();

    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/invoices/search?q={}", invoice_number),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json.as_array().unwrap();
    assert!(!items.is_empty());
}

// ============================================================================
// Authorization
// ============================================================================

#[actix_web::test]
async fn test_invoice_unauthorized_without_token() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/invoices")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
