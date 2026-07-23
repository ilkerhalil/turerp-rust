//! End-to-end POS integration tests covering all 20 POS endpoints.
//!
//! Covers terminal CRUD + sync, sale CRUD, Z-report lifecycle (create -> close
//! -> reconcile), and offline sync queue operations.

use actix_web::{body::to_bytes, body::MessageBody, dev::ServiceResponse, http::StatusCode, test};
use serde_json::{json, Value};

use crate::common::*;

/// Helper: extract the JSON body from a service response.
async fn body_json<B: MessageBody>(resp: ServiceResponse<B>) -> Value
where
    <B as MessageBody>::Error: std::fmt::Debug,
{
    let body = to_bytes(resp.into_body()).await.unwrap();
    serde_json::from_slice(&body).unwrap_or(Value::Null)
}

fn uid() -> String {
    uuid::Uuid::new_v4()
        .to_string()
        .split('-')
        .next()
        .unwrap()
        .to_string()
}

// ============================================================================
// Terminals — CRUD + sync (7 endpoints)
// ============================================================================

#[actix_web::test]
async fn e2e_pos_terminals_full_workflow() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;
    let suffix = uid();

    // 1. POST /api/v1/pos/terminals — create
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/pos/terminals",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "terminal_code": format!("POS-{}", suffix),
        "name": format!("Terminal {}", suffix),
        "warehouse_id": null,
        "store_name": "Main Store"
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let json = body_json(resp).await;
    let id = json["id"].as_i64().unwrap();
    assert_eq!(json["terminal_code"], format!("POS-{}", suffix));
    assert_eq!(json["status"], "Active");
    assert_eq!(json["store_name"], "Main Store");

    // 2. GET /api/v1/pos/terminals — list
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/pos/terminals?page=1&per_page=50",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x["id"] == id));

    // 3. GET /api/v1/pos/terminals/{id} — get
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/pos/terminals/{}", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["id"], id);

    // 4. PUT /api/v1/pos/terminals/{id} — update
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/pos/terminals/{}", id),
        &token,
    )
    .set_json(json!({
        "name": "Updated Terminal",
        "status": "Inactive"
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["name"], "Updated Terminal");
    assert_eq!(json["status"], "Inactive");

    // Reactivate for sale tests
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/pos/terminals/{}", id),
        &token,
    )
    .set_json(json!({ "status": "Active" }))
    .to_request();
    let _ = test::call_service(&app, req).await;

    // 5. POST /api/v1/pos/terminals/{id}/sync — sync
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/pos/terminals/{}/sync", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json["last_sync_at"].is_string());

    // 6. GET /api/v1/pos/terminals/{id}/sales — sales for terminal (empty)
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/pos/terminals/{}/sales", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // 7. DELETE /api/v1/pos/terminals/{id} — soft delete
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/pos/terminals/{}", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Verify deleted terminal is gone
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/pos/terminals/{}", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn e2e_pos_terminal_create_duplicate_code_conflict() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;
    let suffix = uid();

    let payload = json!({
        "tenant_id": 1,
        "terminal_code": format!("DUP-{}", suffix),
        "name": "First Terminal",
        "warehouse_id": null,
        "store_name": null
    });

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/pos/terminals",
        &token,
    )
    .set_json(payload.clone())
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Duplicate code -> 409 Conflict
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/pos/terminals",
        &token,
    )
    .set_json(payload)
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

// ============================================================================
// Sales — CRUD (4 endpoints)
// ============================================================================

#[actix_web::test]
async fn e2e_pos_sale_full_workflow() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;
    let suffix = uid();

    // Create a terminal first
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/pos/terminals",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "terminal_code": format!("SALE-{}", suffix),
        "name": "Sale Terminal",
        "warehouse_id": null,
        "store_name": null
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let terminal_id = body_json(resp).await["id"].as_i64().unwrap();

    // 1. POST /api/v1/pos/sales — create sale with two lines
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/pos/sales", &token)
        .set_json(json!({
            "tenant_id": 1,
            "terminal_id": terminal_id,
            "cari_id": null,
            "sale_date": "2026-01-01T12:00:00Z",
            "payment_method": "Cash",
            "discount_amount": "0",
            "notes": "E2E sale",
            "lines": [
                {
                    "product_id": null,
                    "description": "Item A",
                    "quantity": "2",
                    "unit_price": "10.00",
                    "tax_rate": "18",
                    "discount_amount": "0"
                },
                {
                    "product_id": null,
                    "description": "Item B",
                    "quantity": "1",
                    "unit_price": "50.00",
                    "tax_rate": "18",
                    "discount_amount": "0"
                }
            ]
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let json = body_json(resp).await;
    let sale_id = json["id"].as_i64().unwrap();
    assert_eq!(json["terminal_id"], terminal_id);
    assert_eq!(json["payment_method"], "Cash");
    assert_eq!(json["lines"].as_array().unwrap().len(), 2);
    // subtotal = 2*10 + 1*50 = 70
    assert_eq!(
        json["subtotal"]
            .as_str()
            .unwrap_or(json["subtotal"].to_string().as_str()),
        "70.00"
    );
    // total = subtotal + tax = 70 + 12.6 = 82.60
    let total_str = json["total_amount"].to_string();
    let total = json["total_amount"].as_str().unwrap_or(total_str.as_str());
    assert!(total.starts_with("82.6") || total.starts_with("82.60"));

    // 2. GET /api/v1/pos/sales — list
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/pos/sales?page=1&per_page=50",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x["id"] == sale_id));

    // 3. GET /api/v1/pos/sales/{id} — get sale with lines
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/pos/sales/{}", sale_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["id"], sale_id);
    assert_eq!(json["lines"].as_array().unwrap().len(), 2);

    // 4. GET /api/v1/pos/terminals/{id}/sales — sales by terminal
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/pos/terminals/{}/sales", terminal_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json.as_array().unwrap().iter().any(|x| x["id"] == sale_id));

    // 5. DELETE /api/v1/pos/sales/{id} — soft delete
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/pos/sales/{}", sale_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Verify deleted
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/pos/sales/{}", sale_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn e2e_pos_sale_empty_lines_rejected() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;
    let suffix = uid();

    // Create terminal
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/pos/terminals",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "terminal_code": format!("EMPTY-{}", suffix),
        "name": "Empty Terminal",
        "warehouse_id": null,
        "store_name": null
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    let terminal_id = body_json(resp).await["id"].as_i64().unwrap();

    // Sale with no lines -> 400
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/pos/sales", &token)
        .set_json(json!({
            "tenant_id": 1,
            "terminal_id": terminal_id,
            "sale_date": "2026-01-01T12:00:00Z",
            "payment_method": "Cash",
            "lines": []
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn e2e_pos_sale_inactive_terminal_rejected() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;
    let suffix = uid();

    // Create terminal (default Active), then set to Offline
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/pos/terminals",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "terminal_code": format!("OFF-{}", suffix),
        "name": "Offline Terminal",
        "warehouse_id": null,
        "store_name": null
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    let terminal_id = body_json(resp).await["id"].as_i64().unwrap();

    // Set status to Offline
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/pos/terminals/{}", terminal_id),
        &token,
    )
    .set_json(json!({ "status": "Offline" }))
    .to_request();
    let _ = test::call_service(&app, req).await;

    // Sale on offline terminal -> 400
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/pos/sales", &token)
        .set_json(json!({
            "tenant_id": 1,
            "terminal_id": terminal_id,
            "sale_date": "2026-01-01T12:00:00Z",
            "payment_method": "Cash",
            "lines": [{
                "product_id": null,
                "description": "Item",
                "quantity": "1",
                "unit_price": "10.00",
                "tax_rate": "0",
                "discount_amount": "0"
            }]
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// ============================================================================
// Z-Reports — lifecycle (6 endpoints)
// ============================================================================

#[actix_web::test]
async fn e2e_pos_z_report_full_lifecycle() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;
    let suffix = uid();

    // Create terminal
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/pos/terminals",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "terminal_code": format!("ZRPT-{}", suffix),
        "name": "Z-Report Terminal",
        "warehouse_id": null,
        "store_name": null
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    let terminal_id = body_json(resp).await["id"].as_i64().unwrap();

    // Record a couple of sales
    for payment in ["Cash", "CreditCard"] {
        let req = auth_request(actix_web::http::Method::POST, "/api/v1/pos/sales", &token)
            .set_json(json!({
                "tenant_id": 1,
                "terminal_id": terminal_id,
                "sale_date": "2026-01-01T12:00:00Z",
                "payment_method": payment,
                "lines": [{
                    "product_id": null,
                    "description": "Item",
                    "quantity": "1",
                    "unit_price": "100.00",
                    "tax_rate": "18",
                    "discount_amount": "0"
                }]
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    // 1. POST /api/v1/pos/z-reports — open Z-report
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/pos/z-reports",
        &token,
    )
    .set_json(json!({ "tenant_id": 1, "terminal_id": terminal_id }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let json = body_json(resp).await;
    let report_id = json["id"].as_i64().unwrap();
    assert_eq!(json["status"], "Open");
    assert_eq!(json["transaction_count"], 0);

    // 2. GET /api/v1/pos/z-reports/{id} — get
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/pos/z-reports/{}", report_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // 3. GET /api/v1/pos/z-reports — list
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/pos/z-reports?page=1&per_page=50",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x["id"] == report_id));

    // 4. POST /api/v1/pos/z-reports/{id}/close — close (computes totals)
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/pos/z-reports/{}/close", report_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["status"], "Closed");
    assert_eq!(json["transaction_count"], 2);
    // Each sale total = 100 + 18 tax = 118; two sales = 236
    let total_sales_str = json["total_sales"].to_string();
    let total_sales = json["total_sales"]
        .as_str()
        .unwrap_or(total_sales_str.as_str());
    assert!(total_sales.starts_with("236"));
    let total_cash_str = json["total_cash"].to_string();
    let total_cash = json["total_cash"]
        .as_str()
        .unwrap_or(total_cash_str.as_str());
    assert!(total_cash.starts_with("118"));
    let total_card_str = json["total_card"].to_string();
    let total_card = json["total_card"]
        .as_str()
        .unwrap_or(total_card_str.as_str());
    assert!(total_card.starts_with("118"));

    // 5. POST /api/v1/pos/z-reports/{id}/reconcile — reconcile
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/pos/z-reports/{}/reconcile", report_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["status"], "Reconciled");

    // 6. GET /api/v1/pos/terminals/{id}/z-reports — reports by terminal
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/pos/terminals/{}/z-reports", terminal_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x["id"] == report_id));
}

#[actix_web::test]
async fn e2e_pos_z_report_duplicate_open_conflict() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;
    let suffix = uid();

    // Create terminal
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/pos/terminals",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "terminal_code": format!("DUPRPT-{}", suffix),
        "name": "Dup Report Terminal",
        "warehouse_id": null,
        "store_name": null
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    let terminal_id = body_json(resp).await["id"].as_i64().unwrap();

    // First open Z-report
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/pos/z-reports",
        &token,
    )
    .set_json(json!({ "tenant_id": 1, "terminal_id": terminal_id }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Second open Z-report -> 409 Conflict
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/pos/z-reports",
        &token,
    )
    .set_json(json!({ "tenant_id": 1, "terminal_id": terminal_id }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[actix_web::test]
async fn e2e_pos_z_report_reconcile_before_close_rejected() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;
    let suffix = uid();

    // Create terminal
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/pos/terminals",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "terminal_code": format!("RC-{}", suffix),
        "name": "Reconcile Before Close Terminal",
        "warehouse_id": null,
        "store_name": null
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    let terminal_id = body_json(resp).await["id"].as_i64().unwrap();

    // Open Z-report
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/pos/z-reports",
        &token,
    )
    .set_json(json!({ "tenant_id": 1, "terminal_id": terminal_id }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    let report_id = body_json(resp).await["id"].as_i64().unwrap();

    // Reconcile before close -> 400
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/pos/z-reports/{}/reconcile", report_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// ============================================================================
// Sync Queue — enqueue + pending count (2 endpoints)
// ============================================================================

#[actix_web::test]
async fn e2e_pos_sync_queue_workflow() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;
    let suffix = uid();

    // Create terminal
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/pos/terminals",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "terminal_code": format!("SYNC-{}", suffix),
        "name": "Sync Terminal",
        "warehouse_id": null,
        "store_name": null
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    let terminal_id = body_json(resp).await["id"].as_i64().unwrap();

    // 1. POST /api/v1/pos/sync — enqueue sync item
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/pos/sync", &token)
        .set_json(json!({
            "terminal_id": terminal_id,
            "payload": "{\"type\":\"sale\",\"data\":{}}"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let json = body_json(resp).await;
    assert!(json["id"].as_i64().is_some());

    // Enqueue a second item
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/pos/sync", &token)
        .set_json(json!({
            "terminal_id": terminal_id,
            "payload": "{\"type\":\"refund\",\"data\":{}}"
        }))
        .to_request();
    let _ = test::call_service(&app, req).await;

    // 2. GET /api/v1/pos/sync/pending/{terminal_id} — pending count
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/pos/sync/pending/{}", terminal_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    // Should be 2 pending items
    let count = json
        .as_u64()
        .or_else(|| json.as_str().and_then(|s| s.parse::<u64>().ok()))
        .unwrap_or(0);
    assert_eq!(count, 2);
}

// ============================================================================
// Tenant isolation — sales from tenant 1 not visible to tenant 2
// ============================================================================

#[actix_web::test]
async fn e2e_pos_tenant_isolation() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token_t1, _uid1) = register_admin(&app_state, 1).await;
    let (token_t2, _uid2) = register_admin(&app_state, 2).await;
    let suffix = uid();

    // Create terminal in tenant 1
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/pos/terminals",
        &token_t1,
    )
    .set_json(json!({
        "tenant_id": 1,
        "terminal_code": format!("ISO-{}", suffix),
        "name": "Tenant1 Terminal",
        "warehouse_id": null,
        "store_name": null
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    let terminal_id_t1 = body_json(resp).await["id"].as_i64().unwrap();

    // Tenant 2 cannot see tenant 1's terminal
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/pos/terminals/{}", terminal_id_t1),
        &token_t2,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // Tenant 2 listing returns only their own terminals (none)
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/pos/terminals?page=1&per_page=50",
        &token_t2,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    let json = body_json(resp).await;
    assert!(json["items"]
        .as_array()
        .unwrap()
        .iter()
        .all(|x| x["id"] != terminal_id_t1));
}
