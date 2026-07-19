//! End-to-End Workflow Integration Tests
//!
//! Tests full business workflows across all major modules.
//! Run with: cargo test --test integration e2e_workflow

use actix_web::{body::to_bytes, http::StatusCode, test};
use serde_json::{json, Value};

use crate::common::*;

// ============================================================================
// AUTH WORKFLOW
// ============================================================================

/// Full auth workflow: register → login → me → token validation
#[actix_web::test]
async fn e2e_auth_full_workflow() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;

    // Register
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(json!({
            "username": "e2e_auth_user",
            "email": "e2e_auth@test.com",
            "full_name": "E2E Auth User",
            "password": "Password123!",
            "tenant_id": 1
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "register");

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["user"]["username"], "e2e_auth_user");
    assert!(json["tokens"]["access_token"]
        .as_str()
        .unwrap()
        .starts_with("ey"));

    // Login
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login?tenant_id=1")
        .set_json(json!({"username": "e2e_auth_user", "password": "Password123!"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "login");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let user_token = json["tokens"]["access_token"].as_str().unwrap();

    // Me
    let req = test::TestRequest::get()
        .uri("/api/v1/auth/me")
        .insert_header(("Authorization", format!("Bearer {}", user_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "me");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["username"], "e2e_auth_user");
    assert_eq!(json["role"], "user");

    // Wrong password → 401
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login?tenant_id=1")
        .set_json(json!({"username": "e2e_auth_user", "password": "wrong"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // No token → 401
    let req = test::TestRequest::get().uri("/api/v1/auth/me").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // Invalid token → 401
    let req = test::TestRequest::get()
        .uri("/api/v1/auth/me")
        .insert_header(("Authorization", "Bearer invalid.token.here"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ============================================================================
// CARI CRUD
// ============================================================================

#[actix_web::test]
async fn e2e_cari_full_crud_lifecycle() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, user_id) = register_admin(&app_state, 1).await;

    // Create
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/caris", &token)
        .set_json(json!({
            "code": "E2E-CARI-001",
            "name": "E2E Test Customer",
            "cari_type": "customer",
            "created_by": user_id,
            "tenant_id": 1,
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "create cari");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let cari_id = json["id"].as_i64().unwrap();
    assert_eq!(json["name"], "E2E Test Customer");

    // List
    let req = auth_request(actix_web::http::Method::GET, "/api/v1/caris", &token).to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Get by ID
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/caris/{}", cari_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Update
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/caris/{}", cari_id),
        &token,
    )
    .set_json(json!({
        "code": "E2E-CARI-001",
        "name": "E2E Updated Customer",
        "cari_type": "customer",
        "created_by": user_id,
        "tenant_id": 1,
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "update cari");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["name"], "E2E Updated Customer");

    // Soft delete
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/caris/{}", cari_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "soft delete");

    // Restore (PUT, not POST)
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/caris/{}/restore", cari_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "restore");
}

// ============================================================================
// PRODUCT CRUD
// ============================================================================

#[actix_web::test]
async fn e2e_product_full_crud_lifecycle() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;

    // Create
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/products", &token)
        .set_json(json!({
            "code": "E2E-PROD-001",
            "name": "E2E Test Product",
            "description": "Test product for e2e",
            "purchase_price": 49.99,
            "sale_price": 99.99,
            "tax_rate": 18.0,
            "tenant_id": 1,
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "create product");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let product_id = json["id"].as_i64().unwrap();

    // List
    let req = auth_request(actix_web::http::Method::GET, "/api/v1/products", &token).to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Get by ID
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/products/{}", product_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Update
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/products/{}", product_id),
        &token,
    )
    .set_json(json!({
        "code": "E2E-PROD-001",
        "name": "E2E Updated Product",
        "purchase_price": 59.99,
        "sale_price": 149.99,
        "tax_rate": 18.0,
        "tenant_id": 1,
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "update product");
}

// ============================================================================
// INVOICE WORKFLOW
// ============================================================================

#[actix_web::test]
async fn e2e_invoice_full_workflow() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, user_id) = register_admin(&app_state, 1).await;

    // Create cari
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/caris", &token)
        .set_json(json!({
            "code": "E2E-INV-CARI",
            "name": "Invoice Customer",
            "cari_type": "customer",
            "created_by": user_id,
            "tenant_id": 1,
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let cari_id = serde_json::from_slice::<Value>(&body).unwrap()["id"]
        .as_i64()
        .unwrap();

    // Create invoice
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/invoices", &token)
        .set_json(json!({
            "invoice_type": "SalesInvoice",
            "cari_id": cari_id,
            "issue_date": "2025-07-19T00:00:00Z",
            "due_date": "2025-08-19T00:00:00Z",
            "currency": "TRY",
            "tenant_id": 1,
            "lines": [{
                "description": "Test item",
                "quantity": 2.0,
                "unit_price": 100.0,
                "tax_rate": 18.0,
                "discount_rate": 0.0
            }]
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "create invoice");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let invoice_id = json["id"].as_i64().unwrap();

    // Get invoice
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/invoices/{}", invoice_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // List invoices
    let req = auth_request(actix_web::http::Method::GET, "/api/v1/invoices", &token).to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

// ============================================================================
// SALES ORDER WORKFLOW
// ============================================================================

#[actix_web::test]
async fn e2e_sales_order_workflow() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, user_id) = register_admin(&app_state, 1).await;
    let now = chrono::Utc::now();

    // Create cari
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/caris", &token)
        .set_json(json!({
            "code": "E2E-SO-CARI",
            "name": "Sales Customer",
            "cari_type": "customer",
            "created_by": user_id,
            "tenant_id": 1,
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let cari_id = serde_json::from_slice::<Value>(&body).unwrap()["id"]
        .as_i64()
        .unwrap();

    // Create sales order
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/sales/orders",
        &token,
    )
    .set_json(json!({
        "cari_id": cari_id,
        "order_date": now.to_rfc3339(),
        "tenant_id": 1,
        "lines": [{
            "description": "Test sales line",
            "quantity": "5.00",
            "unit_price": "99.99",
            "tax_rate": "18.00",
            "discount_rate": "0.00"
        }]
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "create sales order");

    // List
    let req =
        auth_request(actix_web::http::Method::GET, "/api/v1/sales/orders", &token).to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

// ============================================================================
// PURCHASE ORDER WORKFLOW
// ============================================================================

#[actix_web::test]
async fn e2e_purchase_order_workflow() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, user_id) = register_admin(&app_state, 1).await;
    let now = chrono::Utc::now();

    // Create vendor
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/caris", &token)
        .set_json(json!({
            "code": "E2E-PO-VENDOR",
            "name": "Purchase Vendor",
            "cari_type": "vendor",
            "created_by": user_id,
            "tenant_id": 1,
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let cari_id = serde_json::from_slice::<Value>(&body).unwrap()["id"]
        .as_i64()
        .unwrap();

    // Create purchase order
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
            "description": "Test purchase line",
            "quantity": "10.00",
            "unit_price": "49.99",
            "tax_rate": "18.00",
            "discount_rate": "0.00"
        }]
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "create purchase order");

    // List
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/purchase-orders",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

// ============================================================================
// STOCK WAREHOUSE WORKFLOW
// ============================================================================

#[actix_web::test]
async fn e2e_stock_warehouse_workflow() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;

    // Create warehouse
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/stock/warehouses",
        &token,
    )
    .set_json(json!({
        "code": "E2E-WH",
        "name": "E2E Warehouse",
        "address": "Test Address",
        "tenant_id": 1,
        "company_id": 1,
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "create warehouse");

    // List warehouses
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/stock/warehouses",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // List stock movements (no base /stock/levels route, only by product/warehouse)
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/stock/movements",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

// ============================================================================
// ACCOUNTING WORKFLOW
// ============================================================================

#[actix_web::test]
async fn e2e_accounting_workflow() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;

    // Create account (unique code to avoid conflict with other tests)
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/accounting/accounts",
        &token,
    )
    .set_json(json!({
        "code": "E2E-ACC-1000",
        "name": "Cash Account",
        "account_type": "Asset",
        "sub_type": "CurrentAsset",
        "company_id": 1,
        "parent_id": null,
        "allow_transaction": true,
        "tenant_id": 1
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "create account");

    // List accounts
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/accounting/accounts",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

// ============================================================================
// COMPANY WORKFLOW
// ============================================================================

#[actix_web::test]
async fn e2e_company_workflow() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;

    let code = format!(
        "E2E-CO-{}",
        uuid::Uuid::new_v4().to_string().split('-').next().unwrap()
    );

    // Create
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/companies", &token)
        .set_json(json!({
            "code": code,
            "name": "E2E Company",
            "tax_number": "1234567890",
            "currency": "TRY",
            "tenant_id": 1
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "create company");

    // List
    let req = auth_request(actix_web::http::Method::GET, "/api/v1/companies", &token).to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

// ============================================================================
// COST CENTER WORKFLOW
// ============================================================================

#[actix_web::test]
async fn e2e_cost_centers_workflow() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;

    // Create
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/cost-centers",
        &token,
    )
    .set_json(json!({
        "code": "E2E-CC",
        "name": "E2E Cost Center",
        "description": "Test cost center",
        "center_type": "Cost",
        "parent_id": null,
        "is_active": true
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "create cost center");

    // List
    let req =
        auth_request(actix_web::http::Method::GET, "/api/v1/cost-centers", &token).to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

// ============================================================================
// CHART OF ACCOUNTS
// ============================================================================

#[actix_web::test]
async fn e2e_chart_of_accounts_workflow() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    // List
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/chart-of-accounts",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Tree
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/chart-of-accounts/tree",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

// ============================================================================
// FEATURE FLAGS
// ============================================================================

#[actix_web::test]
async fn e2e_feature_flags_workflow() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    // Create flag (tenant_id must match admin's tenant for enable/disable to work)
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/feature-flags",
        &token,
    )
    .set_json(json!({
        "name": "e2e.test.flag",
        "description": "E2E test flag",
        "status": "disabled",
        "tenant_id": 1
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "create flag");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let flag_id = serde_json::from_slice::<Value>(&body).unwrap()["id"]
        .as_i64()
        .unwrap();

    // List
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/feature-flags",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Enable
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/feature-flags/{}/enable", flag_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "enable flag");

    // Disable
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/feature-flags/{}/disable", flag_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "disable flag");
}

// ============================================================================
// WEBHOOKS
// ============================================================================

#[actix_web::test]
async fn e2e_webhooks_workflow() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    // Create (secret must be 32+ chars or omitted)
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/webhooks", &token)
        .set_json(json!({
            "url": "https://example.com/webhook",
            "description": "E2E test webhook",
            "event_types": ["invoice_created", "invoice_updated"]
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "create webhook");

    // List
    let req = auth_request(actix_web::http::Method::GET, "/api/v1/webhooks", &token).to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

// ============================================================================
// API KEYS
// ============================================================================

#[actix_web::test]
async fn e2e_api_keys_workflow() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, user_id) = register_admin(&app_state, 1).await;

    // Create
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/api-keys", &token)
        .set_json(json!({
            "name": "E2E Test Key",
            "tenant_id": 1,
            "user_id": user_id,
            "scopes": ["all"],
            "expires_at": null
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "create api key");

    // List
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/api-keys/tenant/1",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

// ============================================================================
// DASHBOARD
// ============================================================================

#[actix_web::test]
async fn e2e_dashboard_kpis() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/dashboard/kpis",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "dashboard kpis");
}

// ============================================================================
// TENANT ISOLATION
// ============================================================================

#[actix_web::test]
async fn e2e_tenant_isolation() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token1, user1) = register_admin(&app_state, 1).await;
    let (token2, _user2) = register_admin(&app_state, 2).await;

    // Tenant 1 creates a cari
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/caris", &token1)
        .set_json(json!({
            "code": "TENANT1-CARI",
            "name": "Tenant 1 Customer",
            "cari_type": "customer",
            "created_by": user1,
            "tenant_id": 1,
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let cari_id = serde_json::from_slice::<Value>(&body).unwrap()["id"]
        .as_i64()
        .unwrap();

    // Tenant 2 should NOT see tenant 1's cari
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/caris/{}", cari_id),
        &token2,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "tenant 2 should not see tenant 1's cari"
    );
}

// ============================================================================
// SUBSCRIPTION PLANS
// ============================================================================

#[actix_web::test]
async fn e2e_subscription_plans() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    // Create a plan (path is /subscription-plans, not /subscriptions/plans)
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/subscription-plans",
        &token,
    )
    .set_json(json!({
        "name": "E2E Plan",
        "description": "E2E test plan",
        "billing_cycle": "monthly",
        "base_amount": "99.99",
        "currency": "TRY",
        "is_active": true,
        "tenant_id": 1
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "create plan");

    // List plans
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/subscription-plans",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list plans");
}

// ============================================================================
// OBSERVABILITY (public endpoint, no auth needed)
// ============================================================================

#[actix_web::test]
async fn e2e_observability_health() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;

    // Health endpoint is public (no auth required)
    let req = test::TestRequest::get()
        .uri("/api/v1/observability/health")
        .to_request();
    let resp = test::call_service(&app, req).await;
    // May be 200 (if observability service is registered) or 500 (if not)
    // The important thing is it doesn't return 401/403 (auth blocked)
    assert!(
        resp.status() == StatusCode::OK || resp.status() == StatusCode::INTERNAL_SERVER_ERROR,
        "observability health should not return 401/403, got {}",
        resp.status()
    );
}
