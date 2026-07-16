//! e-Defter CRUD Integration Tests

use actix_web::{body::to_bytes, http::StatusCode, test};
use serde_json::json;

use crate::common::*;

// ============================================================================
// CRUD Tests
// ============================================================================

#[actix_web::test]
async fn test_create_ledger_period_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/edefter/periods",
        &token,
    )
    .set_json(json!({
        "year": 2024,
        "month": 6,
        "period_type": "YevmiyeDefteri"
    }))
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["year"], 2024);
    assert_eq!(json["month"], 6);
    assert_eq!(json["period_type"], "YevmiyeDefteri");
    assert_eq!(json["status"], "Draft");
    assert!(json["id"].is_number());
}

#[actix_web::test]
async fn test_get_ledger_period_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/edefter/periods",
        &token,
    )
    .set_json(json!({
        "year": 2024,
        "month": 7,
        "period_type": "KebirDefter"
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/edefter/periods/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);

    let body = to_bytes(get_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);
    assert_eq!(json["year"], 2024);
    assert_eq!(json["month"], 7);
    assert_eq!(json["period_type"], "KebirDefter");
}

#[actix_web::test]
async fn test_get_ledger_period_not_found() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/edefter/periods/99999",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_check_period_status() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/edefter/periods",
        &token,
    )
    .set_json(json!({
        "year": 2024,
        "month": 8,
        "period_type": "YevmiyeDefteri"
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let status_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/edefter/periods/{}/status", id),
        &token,
    )
    .to_request();
    let status_resp = test::call_service(&app, status_req).await;
    assert_eq!(status_resp.status(), StatusCode::OK);

    let body = to_bytes(status_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "Draft");
}

#[actix_web::test]
async fn test_populate_period() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/edefter/periods",
        &token,
    )
    .set_json(json!({
        "year": 2024,
        "month": 9,
        "period_type": "YevmiyeDefteri"
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let populate_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/edefter/periods/{}/populate", id),
        &token,
    )
    .set_json(json!({
        "entries": [
            {
                "id": 1,
                "period_id": id,
                "entry_number": 1,
                "entry_date": "2024-09-01",
                "explanation": "Test entry",
                "debit_total": "100.00",
                "credit_total": "100.00",
                "lines": [
                    {
                        "account_code": "100",
                        "account_name": "Cash",
                        "debit": "100.00",
                        "credit": "0.00",
                        "explanation": "Cash debit"
                    },
                    {
                        "account_code": "300",
                        "account_name": "Equity",
                        "debit": "0.00",
                        "credit": "100.00",
                        "explanation": "Equity credit"
                    }
                ]
            }
        ]
    }))
    .to_request();
    let populate_resp = test::call_service(&app, populate_req).await;
    assert_eq!(populate_resp.status(), StatusCode::OK);

    let body = to_bytes(populate_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["entries_count"], 1);
}

#[actix_web::test]
async fn test_validate_period_balance() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/edefter/periods",
        &token,
    )
    .set_json(json!({
        "year": 2024,
        "month": 10,
        "period_type": "YevmiyeDefteri"
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let validate_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/edefter/periods/{}/validate", id),
        &token,
    )
    .to_request();
    let validate_resp = test::call_service(&app, validate_req).await;
    assert_eq!(validate_resp.status(), StatusCode::OK);

    let body = to_bytes(validate_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["is_balanced"].is_boolean());
    assert!(json["total_debit"].is_string());
    assert!(json["total_credit"].is_string());
    assert!(json["difference"].is_string());
    assert!(json["errors"].is_array());
}

// ============================================================================
// Authorization Tests
// ============================================================================

#[actix_web::test]
async fn test_edefter_unauthorized() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/edefter/periods")
        .set_json(json!({
            "year": 2024,
            "month": 1,
            "period_type": "YevmiyeDefteri"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_edefter_normal_user_forbidden() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_user!(&app, 1);

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/edefter/periods",
        &token,
    )
    .set_json(json!({
        "year": 2024,
        "month": 1,
        "period_type": "YevmiyeDefteri"
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}
