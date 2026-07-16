//! Bank Reconciliation Integration Tests

use actix_web::{body::to_bytes, http::StatusCode, test};
use serde_json::json;

use crate::common::*;

// ============================================================================
// Transaction Match/Unmatch Tests
// ============================================================================

#[actix_web::test]
async fn test_unmatch_transaction() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/bank/accounts")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "bank_code": "ziraat",
            "account_number": "UNMATCH001",
            "account_name": "Unmatch Account",
            "currency": "TRY",
            "tenant_id": 1
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let account_id = json["id"].as_i64().unwrap();

    let mt940_data = ":61:230101C500,00NTRF//REF002\n:86:Payment received\n";

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/bank/accounts/{}/statements", account_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "format": "mt940",
            "data": mt940_data
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let transactions = json["transactions"].as_array().unwrap();
    assert!(!transactions.is_empty());
    let transaction_id = transactions[0]["id"].as_i64().unwrap();

    // Match the transaction first
    let req = test::TestRequest::post()
        .uri(&format!(
            "/api/v1/bank/transactions/{}/match",
            transaction_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "invoice_id": 2
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Now unmatch it
    let req = test::TestRequest::post()
        .uri(&format!(
            "/api/v1/bank/transactions/{}/unmatch",
            transaction_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["match_status"], "unmatched");
    assert!(json["matched_invoice_id"].is_null());
}

#[actix_web::test]
async fn test_get_reconciliation_rule_by_id() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/bank/rules")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "rule_name": "Get By ID Rule",
            "match_field": "description",
            "match_pattern": "Test #[0-9]+",
            "auto_match": false,
            "is_active": true,
            "tenant_id": 1
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let rule_id = json["id"].as_i64().unwrap();

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/bank/rules/{}", rule_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], rule_id);
    assert_eq!(json["rule_name"], "Get By ID Rule");
    assert_eq!(json["match_field"], "description");
    assert_eq!(json["match_pattern"], "Test #[0-9]+");
}
