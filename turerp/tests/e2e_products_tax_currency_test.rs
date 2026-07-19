//! E2E workflow tests for Products (categories, units, variants), Tax, Currency, and Barcodes.
//!
//! Run with: cargo test --test integration e2e_products_tax_currency

use actix_web::{body::to_bytes, http::StatusCode, test};
use serde_json::{json, Value};

use crate::common::*;

/// Generate a unique 3-letter uppercase currency code from a UUID.
fn unique_currency_code() -> String {
    let hex = uuid::Uuid::new_v4().simple().to_string();
    hex.chars()
        .take(3)
        .map(|c| ((c as u8 % 26) + b'A') as char)
        .collect()
}

// ============================================================================
// Categories
// ============================================================================

#[actix_web::test]
async fn e2e_categories_workflow() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let cat_name = format!("Cat-{}", uuid::Uuid::new_v4());

    // Create
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/categories", &token)
        .set_json(json!({
            "tenant_id": 1,
            "company_id": 1,
            "name": cat_name,
            "parent_id": null
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "create category");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let id = json["id"].as_i64().unwrap();
    assert_eq!(json["name"], cat_name);

    // List
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/categories?page=1&per_page=50",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list categories");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["total"].as_i64().unwrap() >= 1);

    // Get
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/categories/{}", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "get category");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);

    // Update
    let updated_name = format!("Updated-{}", uuid::Uuid::new_v4());
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/categories/{}", id),
        &token,
    )
    .set_json(json!({
        "name": updated_name,
        "company_id": 1
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "update category");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["name"], updated_name);

    // Soft delete
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/categories/{}", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "soft delete category");

    // Restore
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/categories/{}/restore", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "restore category");

    // Soft delete again for deleted list + destroy
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/categories/{}", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "soft delete category again");

    // Deleted list
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/categories/deleted",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list deleted categories");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let items = json.as_array().unwrap();
    assert!(items.iter().any(|item| item["id"] == id));

    // Destroy
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/categories/{}/destroy", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT, "destroy category");
}

// ============================================================================
// Units
// ============================================================================

#[actix_web::test]
async fn e2e_units_workflow() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let unit_code = format!("U-{}", uuid::Uuid::new_v4());
    let unit_name = format!("Unit-{}", uuid::Uuid::new_v4());

    // Create
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/units", &token)
        .set_json(json!({
            "tenant_id": 1,
            "company_id": 1,
            "code": unit_code,
            "name": unit_name,
            "is_integer": false
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "create unit");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let id = json["id"].as_i64().unwrap();
    assert_eq!(json["code"], unit_code);

    // List
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/units?page=1&per_page=50",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list units");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["total"].as_i64().unwrap() >= 1);

    // Get
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/units/{}", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "get unit");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);

    // Update
    let updated_name = format!("Updated-{}", uuid::Uuid::new_v4());
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/units/{}", id),
        &token,
    )
    .set_json(json!({
        "name": updated_name,
        "company_id": 1
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "update unit");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["name"], updated_name);

    // Soft delete
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/units/{}", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "soft delete unit");

    // Restore
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/units/{}/restore", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "restore unit");

    // Soft delete again
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/units/{}", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "soft delete unit again");

    // Deleted list
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/units/deleted",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list deleted units");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let items = json.as_array().unwrap();
    assert!(items.iter().any(|item| item["id"] == id));

    // Destroy
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/units/{}/destroy", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT, "destroy unit");
}

// ============================================================================
// Product Variants
// ============================================================================

#[actix_web::test]
async fn e2e_product_variants_workflow() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    // Create a product first
    let product_id = seed_product!(&app, &token, 1);

    let variant_name = format!("Variant-{}", uuid::Uuid::new_v4());
    let variant_sku = format!("SKU-{}", uuid::Uuid::new_v4());

    // Create variant
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/products/{}/variants", product_id),
        &token,
    )
    .set_json(json!({
        "product_id": product_id,
        "name": variant_name,
        "sku": variant_sku,
        "barcode": null,
        "price_modifier": "10.00"
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "create variant");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let id = json["id"].as_i64().unwrap();
    assert_eq!(json["name"], variant_name);
    assert_eq!(json["sku"], variant_sku);

    // List variants by product
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/products/{}/variants", product_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list variants");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let items = json.as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["id"], id);

    // Get variant by ID
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/variants/{}", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "get variant");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);

    // Update variant
    let updated_name = format!("Updated-{}", uuid::Uuid::new_v4());
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/variants/{}", id),
        &token,
    )
    .set_json(json!({
        "name": updated_name,
        "price_modifier": "15.00"
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "update variant");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["name"], updated_name);
    assert_eq!(json["price_modifier"], "15.00");

    // Soft delete variant
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/variants/{}", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "soft delete variant");

    // Restore variant
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/variants/{}/restore", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "restore variant");

    // Soft delete again
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/variants/{}", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "soft delete variant again");

    // List deleted variants
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/products/{}/variants/deleted", product_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list deleted variants");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let items = json.as_array().unwrap();
    assert!(items.iter().any(|item| item["id"] == id));

    // Destroy variant
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/variants/{}/destroy", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT, "destroy variant");
}

// ============================================================================
// Tax Rates
// ============================================================================

#[actix_web::test]
async fn e2e_tax_rates_workflow() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    // Create rate A (KDV, default)
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/tax/rates", &token)
        .set_json(json!({
            "tax_type": "KDV",
            "rate": "0.20",
            "effective_from": "2024-01-01",
            "description": format!("KDV-{}", uuid::Uuid::new_v4()),
            "is_default": true
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "create tax rate A");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let id_a = json["id"].as_i64().unwrap();

    // Create rate B (OIV)
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/tax/rates", &token)
        .set_json(json!({
            "tax_type": "OIV",
            "rate": "0.25",
            "effective_from": "2024-01-01",
            "description": format!("OIV-{}", uuid::Uuid::new_v4()),
            "is_default": false
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "create tax rate B");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let id_b = json["id"].as_i64().unwrap();

    // List
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/tax/rates?page=1&per_page=50",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list tax rates");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["total"], 2);

    // Get
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/tax/rates/{}", id_a),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "get tax rate");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id_a);
    assert_eq!(json["tax_type"], "KDV");

    // Effective rate
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/tax/rates/effective?tax_type=KDV&date=2024-06-15",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "effective tax rate");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["tax_type"], "KDV");
    assert_eq!(json["rate"], "0.20");

    // Update
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/tax/rates/{}", id_a),
        &token,
    )
    .set_json(json!({
        "rate": "0.22",
        "description": "Updated rate"
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "update tax rate");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["rate"], "0.22");

    // Soft delete both
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/tax/rates/{}", id_a),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT, "soft delete rate A");

    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/tax/rates/{}", id_b),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT, "soft delete rate B");

    // Restore A
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/tax/rates/{}/restore", id_a),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "restore rate A");

    // Deleted list (should show B)
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/tax/rates/deleted",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list deleted tax rates");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let items = json.as_array().unwrap();
    assert!(items.iter().any(|item| item["id"] == id_b));

    // Bulk restore B
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/tax/rates/bulk-restore",
        &token,
    )
    .set_json(json!({ "ids": [id_b] }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "bulk restore tax rates");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["restored"], 1);
    assert_eq!(json["failed"].as_array().unwrap().len(), 0);

    // Destroy A (soft delete first, then destroy)
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/tax/rates/{}", id_a),
        &token,
    )
    .to_request();
    test::call_service(&app, req).await;

    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/tax/rates/{}/destroy", id_a),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT, "destroy rate A");

    // Destroy B
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/tax/rates/{}", id_b),
        &token,
    )
    .to_request();
    test::call_service(&app, req).await;

    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/tax/rates/{}/destroy", id_b),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT, "destroy rate B");
}

// ============================================================================
// Tax Periods
// ============================================================================

#[actix_web::test]
async fn e2e_tax_periods_workflow() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    // Create period A (KDV, 2024-01)
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/tax/periods", &token)
        .set_json(json!({
            "tax_type": "KDV",
            "period_year": 2024,
            "period_month": 1
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "create tax period A");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let id_a = json["id"].as_i64().unwrap();
    assert_eq!(json["status"], "Open");

    // Create period B (OIV, 2024-01)
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/tax/periods", &token)
        .set_json(json!({
            "tax_type": "OIV",
            "period_year": 2024,
            "period_month": 1
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "create tax period B");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let id_b = json["id"].as_i64().unwrap();

    // List
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/tax/periods?page=1&per_page=50",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list tax periods");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["total"], 2);

    // Get
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/tax/periods/{}", id_a),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "get tax period");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id_a);

    // Calculate (Open → Calculated)
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/tax/periods/{}/calculate", id_a),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "calculate tax period");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "Calculated");

    // File (Calculated → Filed)
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/tax/periods/{}/file", id_a),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "file tax period");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "Filed");

    // Soft delete both
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/tax/periods/{}", id_a),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "soft delete period A"
    );

    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/tax/periods/{}", id_b),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "soft delete period B"
    );

    // Restore A
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/tax/periods/{}/restore", id_a),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "restore period A");

    // Deleted list (should show B)
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/tax/periods/deleted",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list deleted tax periods");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let items = json.as_array().unwrap();
    assert!(items.iter().any(|item| item["id"] == id_b));

    // Bulk restore B
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/tax/periods/bulk-restore",
        &token,
    )
    .set_json(json!({ "ids": [id_b] }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "bulk restore tax periods");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["restored"], 1);

    // Destroy A
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/tax/periods/{}", id_a),
        &token,
    )
    .to_request();
    test::call_service(&app, req).await;

    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/tax/periods/{}/destroy", id_a),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT, "destroy period A");

    // Destroy B
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/tax/periods/{}", id_b),
        &token,
    )
    .to_request();
    test::call_service(&app, req).await;

    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/tax/periods/{}/destroy", id_b),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT, "destroy period B");
}

// ============================================================================
// Tax Calculate
// ============================================================================

#[actix_web::test]
async fn e2e_tax_calculate() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    // Create a KDV rate effective from 2024-01-01
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/tax/rates", &token)
        .set_json(json!({
            "tax_type": "KDV",
            "rate": "0.20",
            "effective_from": "2024-01-01",
            "description": "Calc test rate",
            "is_default": true
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "create rate for calc");

    // Calculate tax
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/tax/calculate",
        &token,
    )
    .set_json(json!({
        "amount": "1000.00",
        "tax_type": "KDV",
        "date": "2024-06-15",
        "inclusive": false
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "calculate tax");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["base_amount"], "1000.00");
    assert_eq!(json["tax_type"], "KDV");
    assert_eq!(json["rate"], "0.20");
    assert_eq!(json["tax_amount"], "200.00");
    assert_eq!(json["inclusive"], false);
}

#[actix_web::test]
async fn e2e_tax_calculate_invoice() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    // calculate-invoice is not implemented yet — returns 501
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/tax/calculate-invoice",
        &token,
    )
    .set_json(json!({ "invoice_id": 1 }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_IMPLEMENTED,
        "calculate invoice tax (not implemented)"
    );
}

// ============================================================================
// Currencies
// ============================================================================

#[actix_web::test]
async fn e2e_currencies_workflow() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let code = unique_currency_code();

    // Create
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/currencies", &token)
        .set_json(json!({
            "code": code,
            "name": "Test Currency",
            "symbol": "$",
            "decimal_places": 2,
            "is_active": true,
            "is_base": false
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "create currency");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["code"], code);

    // List
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/currencies?page=1&per_page=50",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list currencies");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["total"].as_i64().unwrap() >= 1);

    // Get by code
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/currencies/{}", code),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "get currency");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["code"], code);

    // Update
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/currencies/{}", code),
        &token,
    )
    .set_json(json!({
        "name": "Updated Currency",
        "is_active": false
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "update currency");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["name"], "Updated Currency");
    assert_eq!(json["is_active"], false);

    // Soft delete
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/currencies/{}/soft", code),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "soft delete currency"
    );

    // Restore
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/currencies/{}/restore", code),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT, "restore currency");

    // Soft delete again
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/currencies/{}/soft", code),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "soft delete currency again"
    );

    // Deleted list
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/currencies/deleted",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list deleted currencies");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let items = json.as_array().unwrap();
    assert!(items.iter().any(|item| item["code"] == code));

    // Destroy
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/currencies/{}/destroy", code),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT, "destroy currency");
}

// ============================================================================
// Exchange Rates
// ============================================================================

#[actix_web::test]
async fn e2e_exchange_rates_workflow() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    // Create currencies
    let from_code = unique_currency_code();
    let to_code = unique_currency_code();

    for (code, name, symbol) in [
        (from_code.as_str(), "Source Currency", "S"),
        (to_code.as_str(), "Target Currency", "T"),
    ] {
        let req = auth_request(actix_web::http::Method::POST, "/api/v1/currencies", &token)
            .set_json(json!({
                "code": code,
                "name": name,
                "symbol": symbol,
                "decimal_places": 2,
                "is_active": true,
                "is_base": false
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::CREATED,
            "create currency {}",
            code
        );
    }

    // Create exchange rate
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/exchange-rates",
        &token,
    )
    .set_json(json!({
        "from_currency": from_code,
        "to_currency": to_code,
        "rate": "0.85",
        "effective_date": "2024-01-01"
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "create exchange rate");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let id = json["id"].as_i64().unwrap();
    assert_eq!(json["from_currency"], from_code);
    assert_eq!(json["to_currency"], to_code);
    assert_eq!(json["rate"], "0.85");

    // List
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/exchange-rates?page=1&per_page=50",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list exchange rates");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["total"].as_i64().unwrap() >= 1);

    // Effective rate
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!(
            "/api/v1/exchange-rates/effective?from={}&to={}&date=2024-01-01",
            from_code, to_code
        ),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "effective exchange rate");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["from_currency"], from_code);
    assert_eq!(json["to_currency"], to_code);
    assert_eq!(json["rate"], "0.85");

    // Convert
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!(
            "/api/v1/exchange-rates/convert?amount=100&from={}&to={}&date=2024-01-01",
            from_code, to_code
        ),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "convert amount");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["from_currency"], from_code);
    assert_eq!(json["to_currency"], to_code);
    assert_eq!(json["converted_amount"], "85.00");

    // Soft delete
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/exchange-rates/{}/soft", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "soft delete exchange rate"
    );

    // Restore
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/exchange-rates/{}/restore", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "restore exchange rate"
    );

    // Soft delete again
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/exchange-rates/{}/soft", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "soft delete exchange rate again"
    );

    // Deleted list
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/exchange-rates/deleted",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list deleted exchange rates");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let items = json.as_array().unwrap();
    assert!(items.iter().any(|item| item["id"] == id));

    // Destroy
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/exchange-rates/{}/destroy", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "destroy exchange rate"
    );
}

// ============================================================================
// Barcodes
// ============================================================================

#[actix_web::test]
async fn e2e_barcodes_workflow() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let code = format!("BC-{}", uuid::Uuid::new_v4());

    // Generate
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/barcodes/generate",
        &token,
    )
    .set_json(json!({
        "entity_type": "product",
        "entity_type_id": 1,
        "barcode_type": "Code128",
        "code": code,
        "width": null,
        "height": null
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "generate barcode");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let id = json["id"].as_i64().unwrap();
    assert_eq!(json["entity_type"], "product");
    assert_eq!(json["entity_id"], 1);
    assert_eq!(json["code"], code);

    // List
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/barcodes?page=1&per_page=50",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list barcodes");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["total"].as_i64().unwrap() >= 1);

    // Get by entity
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/barcodes/product/1",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "get barcode by entity");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);

    // Delete
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/barcodes/{}", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT, "delete barcode");

    // Verify deleted
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/barcodes/product/1",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "barcode not found after delete"
    );
}
