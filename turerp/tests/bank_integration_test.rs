//! Bank API Integration Tests

use actix_web::{body::to_bytes, http::StatusCode, test};
use serde_json::json;

mod common;
use common::*;

#[actix_web::test]
async fn test_create_bank_account_success() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/bank/accounts")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "bank_code": "garanti",
            "account_number": "12345678",
            "account_name": "Main Account",
            "currency": "TRY",
            "iban": "TR000123456789012345678901",
            "branch_code": "001",
            "is_active": true,
            "tenant_id": 1
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["account_name"], "Main Account");
    assert!(json["id"].is_number());
}

#[actix_web::test]
async fn test_get_bank_accounts() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    // Create an account first
    let create_req = test::TestRequest::post()
        .uri("/api/v1/bank/accounts")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "bank_code": "isbankasi",
            "account_number": "87654321",
            "account_name": "Secondary",
            "currency": "TRY",
            "iban": "TR000987654321098765432109",
            "branch_code": "002",
            "is_active": true,
            "tenant_id": 1
        }))
        .to_request();
    let resp = test::call_service(&app, create_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Get all accounts
    let req = test::TestRequest::get()
        .uri("/api/v1/bank/accounts")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.as_array().unwrap().len() >= 1);
}

#[actix_web::test]
async fn test_get_bank_account_by_id() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    let create_req = test::TestRequest::post()
        .uri("/api/v1/bank/accounts")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "bank_code": "akbank",
            "account_number": "11112222",
            "account_name": "Test Account",
            "currency": "USD",
            "iban": "TR000111122223333444455556",
            "branch_code": "003",
            "is_active": true,
            "tenant_id": 1
        }))
        .to_request();
    let resp = test::call_service(&app, create_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/bank/accounts/{}", id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["account_name"], "Test Account");
}

#[actix_web::test]
async fn test_update_bank_account() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    let create_req = test::TestRequest::post()
        .uri("/api/v1/bank/accounts")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "bank_code": "halkbank",
            "account_number": "33334444",
            "account_name": "Old Name",
            "currency": "TRY",
            "iban": "TR000333344445555666677778",
            "branch_code": "004",
            "is_active": true,
            "tenant_id": 1
        }))
        .to_request();
    let resp = test::call_service(&app, create_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let update_req = test::TestRequest::put()
        .uri(&format!("/api/v1/bank/accounts/{}", id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "account_name": "Updated Name",
            "is_active": false
        }))
        .to_request();
    let resp = test::call_service(&app, update_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["account_name"], "Updated Name");
    assert_eq!(json["is_active"], false);
}

#[actix_web::test]
async fn test_delete_bank_account() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    let create_req = test::TestRequest::post()
        .uri("/api/v1/bank/accounts")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "bank_code": "ziraat",
            "account_number": "55556666",
            "account_name": "To Delete",
            "currency": "TRY",
            "iban": "TR000555566667777888899990",
            "branch_code": "005",
            "is_active": true,
            "tenant_id": 1
        }))
        .to_request();
    let resp = test::call_service(&app, create_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/bank/accounts/{}", id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, delete_req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn test_bank_account_tenant_isolation() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token1, _) = register_admin(&app_state, 1).await;
    let (token2, _) = register_admin(&app_state, 2).await;

    // Create account in tenant 1
    let create_req = test::TestRequest::post()
        .uri("/api/v1/bank/accounts")
        .insert_header(("Authorization", format!("Bearer {}", token1)))
        .set_json(json!({
            "bank_code": "yapikredi",
            "account_number": "99990000",
            "account_name": "Tenant1 Account",
            "currency": "TRY",
            "iban": "TR000999900001111222233334",
            "branch_code": "006",
            "is_active": true,
            "tenant_id": 1
        }))
        .to_request();
    let resp = test::call_service(&app, create_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Try to access from tenant 2
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/bank/accounts/{}", id))
        .insert_header(("Authorization", format!("Bearer {}", token2)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_create_bank_account_requires_admin() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_user!(&app, 1);

    let req = test::TestRequest::post()
        .uri("/api/v1/bank/accounts")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "bank_code": "garanti",
            "account_number": "12345678",
            "account_name": "Main Account",
            "currency": "TRY",
            "iban": "TR000123456789012345678901",
            "branch_code": "001",
            "is_active": true,
            "tenant_id": 1
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_web::test]
async fn test_bank_account_validation_error() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    // Missing required fields
    let req = test::TestRequest::post()
        .uri("/api/v1/bank/accounts")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "bank_code": "garanti"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn test_get_bank_account_not_found() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/bank/accounts/999999")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_restore_bank_account() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    let create_req = test::TestRequest::post()
        .uri("/api/v1/bank/accounts")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "bank_code": "yapikredi",
            "account_number": "77778888",
            "account_name": "Restorable",
            "currency": "TRY",
            "tenant_id": 1
        }))
        .to_request();
    let resp = test::call_service(&app, create_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Soft delete
    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/bank/accounts/{}", id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, delete_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Verify not found
    let get_req = test::TestRequest::get()
        .uri(&format!("/api/v1/bank/accounts/{}", id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, get_req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // Restore
    let restore_req = test::TestRequest::put()
        .uri(&format!("/api/v1/bank/accounts/{}/restore", id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, restore_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);
    assert_eq!(json["account_name"], "Restorable");

    // Verify accessible again
    let get_req = test::TestRequest::get()
        .uri(&format!("/api/v1/bank/accounts/{}", id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, get_req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn test_destroy_bank_account_permanent() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    let create_req = test::TestRequest::post()
        .uri("/api/v1/bank/accounts")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "bank_code": "isbankasi",
            "account_number": "44445555",
            "account_name": "To Destroy",
            "currency": "TRY",
            "tenant_id": 1
        }))
        .to_request();
    let resp = test::call_service(&app, create_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Permanent delete
    let destroy_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/bank/accounts/{}/destroy", id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, destroy_req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Verify gone
    let get_req = test::TestRequest::get()
        .uri(&format!("/api/v1/bank/accounts/{}", id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, get_req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // Restore should also fail
    let restore_req = test::TestRequest::put()
        .uri(&format!("/api/v1/bank/accounts/{}/restore", id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, restore_req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

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

#[actix_web::test]
async fn test_unmatch_transaction() {
    let app_state = create_test_app_state();
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
    let app_state = create_test_app_state();
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
