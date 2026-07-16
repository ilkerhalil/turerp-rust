//! Inter-company API Integration Tests

use actix_web::{body::to_bytes, http::StatusCode, test};
use serde_json::json;

use crate::common::*;

macro_rules! create_company {
    ($app:expr, $token:expr, $name:expr) => {{
        let code = format!("CMP-{}", uuid::Uuid::new_v4());
        let req = auth_request(actix_web::http::Method::POST, "/api/v1/companies", $token)
            .set_json(json!({
                "code": code,
                "name": $name,
                "tax_number": "1234567890",
                "currency": "TRY",
                "tenant_id": 1
            }))
            .to_request();
        let resp = test::call_service($app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = to_bytes(resp.into_body()).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        json["id"].as_i64().unwrap()
    }};
}

macro_rules! create_product {
    ($app:expr, $token:expr) => {{
        let code = format!("PROD-{}", uuid::Uuid::new_v4());
        let req = auth_request(actix_web::http::Method::POST, "/api/v1/products", $token)
            .set_json(json!({
                "code": code,
                "name": "Test Product",
                "purchase_price": 50.0,
                "sale_price": 100.0,
                "tax_rate": 18.0,
                "tenant_id": 1
            }))
            .to_request();
        let resp = test::call_service($app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = to_bytes(resp.into_body()).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        json["id"].as_i64().unwrap()
    }};
}

macro_rules! create_warehouse {
    ($app:expr, $token:expr, $company_id:expr) => {{
        let code = format!("WH-{}", uuid::Uuid::new_v4());
        let req = auth_request(
            actix_web::http::Method::POST,
            "/api/v1/stock/warehouses",
            $token,
        )
        .set_json(json!({
            "code": code,
            "name": "Main Warehouse",
            "company_id": $company_id,
            "tenant_id": 1
        }))
        .to_request();
        let resp = test::call_service($app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = to_bytes(resp.into_body()).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        json["id"].as_i64().unwrap()
    }};
}

macro_rules! seed_stock {
    ($app:expr, $token:expr, $wh_id:expr, $co_id:expr, $prod_id:expr, $user_id:expr) => {{
        let req = auth_request(
            actix_web::http::Method::POST,
            "/api/v1/stock/movements",
            $token,
        )
        .set_json(json!({
            "warehouse_id": $wh_id,
            "company_id": $co_id,
            "product_id": $prod_id,
            "movement_type": "Purchase",
            "quantity": 100,
            "reference_type": "InitialStock",
            "reference_id": null,
            "notes": "Seed stock",
            "tenant_id": 1,
            "created_by": $user_id
        }))
        .to_request();
        let resp = test::call_service($app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }};
}

// ============================================================================
// Cross-company Invoice API Tests
// ============================================================================

#[actix_web::test]
async fn test_cross_company_invoice_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let seller_id = create_company!(&app, &token, "Seller Company");
    let buyer_id = create_company!(&app, &token, "Buyer Company");
    let product_id = create_product!(&app, &token);

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/companies/cross-invoice",
        &token,
    )
    .set_json(json!({
        "seller_company_id": seller_id,
        "buyer_company_id": buyer_id,
        "lines": [
            {
                "product_id": product_id,
                "description": "Cross sale",
                "quantity": 2,
                "unit_price": 95.0,
                "vat_rate": 18.0
            }
        ]
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["sales_invoice_id"].is_number());
    assert!(json["purchase_invoice_id"].is_number());
}

#[actix_web::test]
async fn test_cross_company_invoice_empty_lines_validation() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/companies/cross-invoice",
        &token,
    )
    .set_json(json!({
        "seller_company_id": 1,
        "buyer_company_id": 2,
        "lines": []
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn test_cross_company_invoice_company_not_found() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/companies/cross-invoice",
        &token,
    )
    .set_json(json!({
        "seller_company_id": 99999,
        "buyer_company_id": 2,
        "lines": [
            {
                "product_id": 1,
                "description": "Cross sale",
                "quantity": 2,
                "unit_price": 100.0,
                "vat_rate": 18.0
            }
        ]
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_cross_company_invoice_transfer_pricing_violation() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let seller_id = create_company!(&app, &token, "Seller Company");
    let buyer_id = create_company!(&app, &token, "Buyer Company");
    let product_id = create_product!(&app, &token);

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/companies/cross-invoice",
        &token,
    )
    .set_json(json!({
        "seller_company_id": seller_id,
        "buyer_company_id": buyer_id,
        "lines": [
            {
                "product_id": product_id,
                "description": "Cross sale",
                "quantity": 1,
                "unit_price": 200.0,
                "vat_rate": 18.0
            }
        ]
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn test_cross_company_invoice_unauthorized() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/companies/cross-invoice")
        .set_json(json!({
            "seller_company_id": 1,
            "buyer_company_id": 2,
            "lines": [
                {
                    "product_id": 1,
                    "description": "Cross sale",
                    "quantity": 2,
                    "unit_price": 100.0,
                    "vat_rate": 18.0
                }
            ]
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_cross_company_invoice_normal_user_forbidden() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_user!(&app, 1);

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/companies/cross-invoice",
        &token,
    )
    .set_json(json!({
        "seller_company_id": 1,
        "buyer_company_id": 2,
        "lines": [
            {
                "product_id": 1,
                "description": "Cross sale",
                "quantity": 2,
                "unit_price": 100.0,
                "vat_rate": 18.0
            }
        ]
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

// ============================================================================
// Stock Transfer API Tests
// ============================================================================

#[actix_web::test]
async fn test_stock_transfer_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, user_id) = register_admin(&state, 1).await;

    let from_id = create_company!(&app, &token, "Source Company");
    let to_id = create_company!(&app, &token, "Dest Company");
    let warehouse_id = create_warehouse!(&app, &token, from_id);
    let product_id = create_product!(&app, &token);
    seed_stock!(&app, &token, warehouse_id, from_id, product_id, user_id);

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/companies/stock-transfer",
        &token,
    )
    .set_json(json!({
        "from_company_id": from_id,
        "to_company_id": to_id,
        "product_id": product_id,
        "warehouse_id": warehouse_id,
        "quantity": 10
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["out_movement_id"].is_number());
    assert!(json["in_movement_id"].is_number());
}

#[actix_web::test]
async fn test_stock_transfer_company_not_found() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/companies/stock-transfer",
        &token,
    )
    .set_json(json!({
        "from_company_id": 99999,
        "to_company_id": 2,
        "product_id": 1,
        "warehouse_id": 1,
        "quantity": 10
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_stock_transfer_unauthorized() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/companies/stock-transfer")
        .set_json(json!({
            "from_company_id": 1,
            "to_company_id": 2,
            "product_id": 1,
            "warehouse_id": 1,
            "quantity": 10
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_stock_transfer_insufficient_stock() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let from_id = create_company!(&app, &token, "Source Company");
    let to_id = create_company!(&app, &token, "Dest Company");
    let warehouse_id = create_warehouse!(&app, &token, from_id);
    let product_id = create_product!(&app, &token);

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/companies/stock-transfer",
        &token,
    )
    .set_json(json!({
        "from_company_id": from_id,
        "to_company_id": to_id,
        "product_id": product_id,
        "warehouse_id": warehouse_id,
        "quantity": 10
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}
