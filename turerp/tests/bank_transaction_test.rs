//! Bank Transaction Integration Tests

use actix_web::{body::to_bytes, http::StatusCode, test};
use serde_json::json;

mod common;
use common::*;

#[actix_web::test]
async fn test_create_reconciliation_rule() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/bank/rules")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "rule_name": "Auto Match Description",
            "match_field": "description",
            "match_pattern": "Invoice #[0-9]+",
            "auto_match": true,
            "is_active": true,
            "tenant_id": 1
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["rule_name"], "Auto Match Description");
    assert_eq!(json["match_field"], "description");
    assert!(json["id"].is_number());
}

#[actix_web::test]
async fn test_get_reconciliation_rules() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    for i in 0..2 {
        let req = test::TestRequest::post()
            .uri("/api/v1/bank/rules")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(json!({
                "rule_name": format!("Rule {}", i),
                "match_field": "amount",
                "match_pattern": format!("^{}$", i),
                "tenant_id": 1
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    let req = test::TestRequest::get()
        .uri("/api/v1/bank/rules")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let rules = json.as_array().unwrap();
    assert_eq!(rules.len(), 2);
}

#[actix_web::test]
async fn test_update_reconciliation_rule() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/bank/rules")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "rule_name": "Old Rule",
            "match_field": "reference",
            "match_pattern": "REF[0-9]+",
            "tenant_id": 1
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let rule_id = json["id"].as_i64().unwrap();

    let req = test::TestRequest::put()
        .uri(&format!("/api/v1/bank/rules/{}", rule_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "rule_name": "Updated Rule",
            "is_active": false
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["rule_name"], "Updated Rule");
    assert_eq!(json["is_active"], false);
}

#[actix_web::test]
async fn test_delete_reconciliation_rule() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/bank/rules")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "rule_name": "Delete Me",
            "match_field": "description",
            "match_pattern": "test",
            "tenant_id": 1
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let rule_id = json["id"].as_i64().unwrap();

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/bank/rules/{}", rule_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/bank/rules/{}", rule_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_get_transactions_empty() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/bank/accounts")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "bank_code": "garanti",
            "account_number": "TXN001",
            "account_name": "Transaction Account",
            "currency": "TRY",
            "tenant_id": 1
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let account_id = json["id"].as_i64().unwrap();

    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/v1/bank/accounts/{}/transactions",
            account_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json.as_array().unwrap().len(), 0);
}

#[actix_web::test]
async fn test_get_unmatched_transactions_empty() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/bank/accounts")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "bank_code": "ziraat",
            "account_number": "TXN002",
            "account_name": "Unmatched Account",
            "currency": "TRY",
            "tenant_id": 1
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let account_id = json["id"].as_i64().unwrap();

    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/v1/bank/accounts/{}/transactions/unmatched",
            account_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json.as_array().unwrap().len(), 0);
}

#[actix_web::test]
async fn test_get_reconciliation_report() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/bank/reconciliation")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["total_transactions"].is_number());
    assert!(json["matched_count"].is_number());
    assert!(json["unmatched_count"].is_number());
    // Decimal types serialize as strings in JSON
    assert!(json["total_amount"].is_string());
    assert!(json["matched_amount"].is_string());
    assert!(json["unmatched_amount"].is_string());
}

#[actix_web::test]
async fn test_auto_reconcile_admin_only() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/bank/reconcile")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["total_transactions"].is_number());
}

#[actix_web::test]
async fn test_auto_reconcile_forbidden_for_non_admin() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_user!(&app, 1);

    let req = test::TestRequest::post()
        .uri("/api/v1/bank/reconcile")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_web::test]
async fn test_upload_statement_mt940() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/bank/accounts")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "bank_code": "garanti",
            "account_number": "STMT001",
            "account_name": "Statement Account",
            "currency": "TRY",
            "tenant_id": 1
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let account_id = json["id"].as_i64().unwrap();

    let mt940_data = ":61:230101C1000,00NTRF//REF001\n:86:Invoice payment\n";

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
    assert!(json["statement_id"].is_number());
    assert!(json["transactions_imported"].is_number());
}

#[actix_web::test]
async fn test_upload_statement_forbidden_for_non_admin() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;
    let (admin_token, _) = register_admin(&app_state, 1).await;
    let (user_token, _) = register_user!(&app, 1);

    let req = test::TestRequest::post()
        .uri("/api/v1/bank/accounts")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "bank_code": "garanti",
            "account_number": "STMT002",
            "account_name": "Statement Account 2",
            "currency": "TRY",
            "tenant_id": 1
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let account_id = json["id"].as_i64().unwrap();

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/bank/accounts/{}/statements", account_id))
        .insert_header(("Authorization", format!("Bearer {}", user_token)))
        .set_json(json!({
            "format": "mt940",
            "data": ":61:230101C500,00NTRF//REF002\n"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_web::test]
async fn test_match_transaction() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/bank/accounts")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "bank_code": "garanti",
            "account_number": "MATCH001",
            "account_name": "Match Account",
            "currency": "TRY",
            "tenant_id": 1
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let account_id = json["id"].as_i64().unwrap();

    let mt940_data = ":61:230101C1000,00NTRF//REF001\n:86:Invoice payment\n";

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

    let req = test::TestRequest::post()
        .uri(&format!(
            "/api/v1/bank/transactions/{}/match",
            transaction_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "invoice_id": 1
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["match_status"], "manual");
    assert_eq!(json["matched_invoice_id"], 1);
}
