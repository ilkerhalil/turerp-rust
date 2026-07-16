//! Tax CRUD Integration Tests

use actix_web::{body::to_bytes, http::StatusCode, test};
use serde_json::json;

use crate::common::*;

// ============================================================================
// Tax Rate CRUD Tests
// ============================================================================

#[actix_web::test]
async fn test_create_tax_rate_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(actix_web::http::Method::POST, "/api/v1/tax/rates", &token)
        .set_json(json!({
            "tax_type": "KDV",
            "rate": "0.20",
            "effective_from": "2024-01-01",
            "description": "Standard KDV rate",
            "is_default": true
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["tax_type"], "KDV");
    assert_eq!(json["rate"], "0.20");
    assert_eq!(json["description"], "Standard KDV rate");
    assert!(json["id"].is_number());
}

#[actix_web::test]
async fn test_list_tax_rates_paginated() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    for i in 1..=3 {
        let req = auth_request(actix_web::http::Method::POST, "/api/v1/tax/rates", &token)
            .set_json(json!({
                "tax_type": "KDV",
                "rate": format!("0.{}", i * 10),
                "effective_from": "2024-01-01",
                "description": format!("Rate {}", i),
                "is_default": false
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/tax/rates?page=1&per_page=2",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["items"].as_array().unwrap().len(), 2);
    assert_eq!(json["total"], 3);
}

#[actix_web::test]
async fn test_list_tax_rates_by_type() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(actix_web::http::Method::POST, "/api/v1/tax/rates", &token)
        .set_json(json!({
            "tax_type": "KDV",
            "rate": "0.20",
            "effective_from": "2024-01-01",
            "description": "KDV rate",
            "is_default": false
        }))
        .to_request();
    test::call_service(&app, req).await;

    let req = auth_request(actix_web::http::Method::POST, "/api/v1/tax/rates", &token)
        .set_json(json!({
            "tax_type": "OIV",
            "rate": "0.25",
            "effective_from": "2024-01-01",
            "description": "OIV rate",
            "is_default": false
        }))
        .to_request();
    test::call_service(&app, req).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/tax/rates?tax_type=KDV",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["tax_type"], "KDV");
}

#[actix_web::test]
async fn test_get_tax_rate_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/tax/rates", &token)
        .set_json(json!({
            "tax_type": "KDV",
            "rate": "0.18",
            "effective_from": "2024-01-01",
            "description": "Get test",
            "is_default": false
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/tax/rates/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);

    let body = to_bytes(get_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);
    assert_eq!(json["tax_type"], "KDV");
}

#[actix_web::test]
async fn test_get_tax_rate_not_found() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/tax/rates/99999",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_update_tax_rate_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/tax/rates", &token)
        .set_json(json!({
            "tax_type": "KDV",
            "rate": "0.10",
            "effective_from": "2024-01-01",
            "description": "Original",
            "is_default": false
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let update_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/tax/rates/{}", id),
        &token,
    )
    .set_json(json!({
        "rate": "0.15",
        "description": "Updated"
    }))
    .to_request();
    let update_resp = test::call_service(&app, update_req).await;
    assert_eq!(update_resp.status(), StatusCode::OK);

    let body = to_bytes(update_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["rate"], "0.15");
    assert_eq!(json["description"], "Updated");
    assert_eq!(json["tax_type"], "KDV");
}

// ============================================================================
// Soft Delete Tests
// ============================================================================

#[actix_web::test]
async fn test_delete_and_restore_tax_rate() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/tax/rates", &token)
        .set_json(json!({
            "tax_type": "KDV",
            "rate": "0.20",
            "effective_from": "2024-01-01",
            "description": "Delete test",
            "is_default": false
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/tax/rates/{}", id),
        &token,
    )
    .to_request();
    let del_resp = test::call_service(&app, del_req).await;
    assert_eq!(del_resp.status(), StatusCode::NO_CONTENT);

    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/tax/rates/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);

    let restore_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/tax/rates/{}/restore", id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::OK);

    let body = to_bytes(restore_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);

    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/tax/rates/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn test_list_deleted_tax_rates() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/tax/rates", &token)
        .set_json(json!({
            "tax_type": "KDV",
            "rate": "0.20",
            "effective_from": "2024-01-01",
            "description": "List deleted test",
            "is_default": false
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/tax/rates/{}", id),
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    let list_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/tax/rates/deleted",
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
async fn test_destroy_tax_rate_permanently() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/tax/rates", &token)
        .set_json(json!({
            "tax_type": "KDV",
            "rate": "0.20",
            "effective_from": "2024-01-01",
            "description": "Destroy test",
            "is_default": false
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/tax/rates/{}", id),
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    let destroy_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/tax/rates/{}/destroy", id),
        &token,
    )
    .to_request();
    let destroy_resp = test::call_service(&app, destroy_req).await;
    assert_eq!(destroy_resp.status(), StatusCode::NO_CONTENT);

    let restore_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/tax/rates/{}/restore", id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_bulk_restore_tax_rates() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let mut ids = Vec::new();
    for i in 1..=2 {
        let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/tax/rates", &token)
            .set_json(json!({
                "tax_type": "KDV",
                "rate": format!("0.{}", i * 10),
                "effective_from": "2024-01-01",
                "description": format!("Bulk {}", i),
                "is_default": false
            }))
            .to_request();
        let create_resp = test::call_service(&app, create_req).await;
        let body = to_bytes(create_resp.into_body()).await.unwrap();
        let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let id = create_json["id"].as_i64().unwrap();
        ids.push(id);

        let del_req = auth_request(
            actix_web::http::Method::DELETE,
            &format!("/api/v1/tax/rates/{}", id),
            &token,
        )
        .to_request();
        test::call_service(&app, del_req).await;
    }

    let restore_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/tax/rates/bulk-restore",
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
}

// ============================================================================
// Search / Effective Rate
// ============================================================================

#[actix_web::test]
async fn test_get_effective_tax_rate() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(actix_web::http::Method::POST, "/api/v1/tax/rates", &token)
        .set_json(json!({
            "tax_type": "KDV",
            "rate": "0.20",
            "effective_from": "2024-01-01",
            "description": "Effective rate test",
            "is_default": true
        }))
        .to_request();
    test::call_service(&app, req).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/tax/rates/effective?tax_type=KDV&date=2024-06-15",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["tax_type"], "KDV");
    assert_eq!(json["rate"], "0.20");
}

#[actix_web::test]
async fn test_calculate_tax() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(actix_web::http::Method::POST, "/api/v1/tax/rates", &token)
        .set_json(json!({
            "tax_type": "KDV",
            "rate": "0.20",
            "effective_from": "2024-01-01",
            "description": "Calc test",
            "is_default": true
        }))
        .to_request();
    test::call_service(&app, req).await;

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
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["base_amount"], "1000.00");
    assert_eq!(json["tax_type"], "KDV");
    assert_eq!(json["rate"], "0.20");
    assert_eq!(json["tax_amount"], "200.00");
    assert_eq!(json["inclusive"], false);
}

// ============================================================================
// Unauthorized / Not Found
// ============================================================================

#[actix_web::test]
async fn test_tax_unauthorized_without_token() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/tax/rates")
        .set_json(json!({
            "tax_type": "KDV",
            "rate": "0.20",
            "effective_from": "2024-01-01",
            "description": "No auth",
            "is_default": false
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_tax_rate_not_found() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/tax/rates/99999",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
