//! Barcode CRUD Integration Tests

use actix_web::{body::to_bytes, http::StatusCode, test};
use serde_json::json;

use crate::common::*;

// ============================================================================
// CRUD Tests
// ============================================================================

#[actix_web::test]
async fn test_generate_barcode_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/barcodes/generate",
        &token,
    )
    .set_json(json!({
        "entity_type": "product",
        "entity_type_id": 1,
        "barcode_type": "Ean13",
        "code": "5901234123457",
        "width": null,
        "height": null
    }))
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["entity_type"], "product");
    assert_eq!(json["entity_id"], 1);
    assert_eq!(json["barcode_type"], "Ean13");
    assert_eq!(json["code"], "5901234123457");
    assert!(json["id"].is_number());
}

#[actix_web::test]
async fn test_list_barcodes_paginated() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    for i in 1..=3 {
        let req = auth_request(
            actix_web::http::Method::POST,
            "/api/v1/barcodes/generate",
            &token,
        )
        .set_json(json!({
            "entity_type": "product",
            "entity_type_id": i,
            "barcode_type": "Ean13",
            "code": format!("59012341234{}", i + 4),
            "width": null,
            "height": null
        }))
        .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/barcodes?page=1&per_page=2",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["data"].as_array().unwrap().len(), 2);
    assert_eq!(json["total"], 3);
    assert_eq!(json["page"], 1);
    assert_eq!(json["per_page"], 2);
}

#[actix_web::test]
async fn test_get_barcode_for_entity_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/barcodes/generate",
        &token,
    )
    .set_json(json!({
        "entity_type": "invoice",
        "entity_type_id": 42,
        "barcode_type": "QrCode",
        "code": "INV-2024-001",
        "width": null,
        "height": null
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    assert_eq!(create_resp.status(), StatusCode::CREATED);

    let get_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/barcodes/invoice/42",
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);

    let body = to_bytes(get_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["entity_type"], "invoice");
    assert_eq!(json["entity_id"], 42);
    assert_eq!(json["barcode_type"], "QrCode");
    assert_eq!(json["code"], "INV-2024-001");
}

#[actix_web::test]
async fn test_get_barcode_for_entity_not_found() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/barcodes/product/99999",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_delete_barcode_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/barcodes/generate",
        &token,
    )
    .set_json(json!({
        "entity_type": "product",
        "entity_type_id": 99,
        "barcode_type": "Code128",
        "code": "CODE128-TEST",
        "width": null,
        "height": null
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/barcodes/{}", id),
        &token,
    )
    .to_request();
    let del_resp = test::call_service(&app, del_req).await;
    assert_eq!(del_resp.status(), StatusCode::NO_CONTENT);

    let get_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/barcodes/product/99",
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// Authorization Tests
// ============================================================================

#[actix_web::test]
async fn test_generate_barcode_unauthorized() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/barcodes/generate")
        .set_json(json!({
            "entity_type": "product",
            "entity_type_id": 1,
            "barcode_type": "Ean13",
            "code": "5901234123457",
            "width": null,
            "height": null
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_delete_barcode_not_found() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::DELETE,
        "/api/v1/barcodes/99999",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
