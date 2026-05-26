//! Accounting CRUD Integration Tests

use actix_web::{body::to_bytes, http::StatusCode, test};
use serde_json::json;

mod common;
use common::*;

// ============================================================================
// Account CRUD Tests
// ============================================================================

#[actix_web::test]
async fn test_create_account_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/accounting/accounts",
        &token,
    )
    .set_json(json!({
        "code": "9901",
        "name": "Cash",
        "account_type": "Asset",
        "sub_type": "CurrentAsset",
        "company_id": 1,
        "parent_id": null,
        "allow_transaction": true,
        "tenant_id": 1
    }))
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["code"], "9901");
    assert_eq!(json["name"], "Cash");
    assert_eq!(json["account_type"], "Asset");
    assert_eq!(json["sub_type"], "CurrentAsset");
    assert!(json["id"].is_number());
}

#[actix_web::test]
async fn test_list_accounts_paginated() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    for i in 1..=3 {
        let req = auth_request(
            actix_web::http::Method::POST,
            "/api/v1/accounting/accounts",
            &token,
        )
        .set_json(json!({
            "code": format!("990{}", i + 5),
            "name": format!("Account {}", i),
            "account_type": "Asset",
            "sub_type": "CurrentAsset",
            "company_id": 1,
            "parent_id": null,
            "allow_transaction": true,
            "tenant_id": 1
        }))
        .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/accounting/accounts?page=1&per_page=2",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["items"].as_array().unwrap().len(), 2);
    assert_eq!(json["page"], 1);
    assert_eq!(json["per_page"], 2);
}

#[actix_web::test]
async fn test_get_accounts_by_type() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/accounting/accounts",
        &token,
    )
    .set_json(json!({
        "code": "9910",
        "name": "Accounts Payable",
        "account_type": "Liability",
        "sub_type": "CurrentLiability",
        "company_id": 1,
        "parent_id": null,
        "allow_transaction": true,
        "tenant_id": 1
    }))
    .to_request();
    test::call_service(&app, req).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/accounting/accounts/type/Liability",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json.as_array().unwrap();
    assert!(!items.is_empty());
    assert_eq!(items[0]["account_type"], "Liability");
}

#[actix_web::test]
async fn test_get_account_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/accounting/accounts",
        &token,
    )
    .set_json(json!({
        "code": "9920",
        "name": "Get Test Account",
        "account_type": "Expense",
        "sub_type": "OperatingExpense",
        "company_id": 1,
        "parent_id": null,
        "allow_transaction": true,
        "tenant_id": 1
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/accounting/accounts/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);

    let body = to_bytes(get_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);
    assert_eq!(json["code"], "9920");
}

#[actix_web::test]
async fn test_get_account_not_found() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/accounting/accounts/99999",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// Account Soft Delete Tests
// ============================================================================

#[actix_web::test]
async fn test_delete_and_restore_account() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/accounting/accounts",
        &token,
    )
    .set_json(json!({
        "code": "DEL-001",
        "name": "Delete Test Account",
        "account_type": "Revenue",
        "sub_type": "OperatingRevenue",
        "company_id": 1,
        "parent_id": null,
        "allow_transaction": true,
        "tenant_id": 1
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Soft delete
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/accounting/accounts/{}", id),
        &token,
    )
    .to_request();
    let del_resp = test::call_service(&app, del_req).await;
    assert_eq!(del_resp.status(), StatusCode::OK);

    // Verify deleted - should return 404
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/accounting/accounts/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);

    // Restore
    let restore_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/accounting/accounts/{}/restore", id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::OK);

    let body = to_bytes(restore_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);
    assert_eq!(json["code"], "DEL-001");

    // Verify restored
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/accounting/accounts/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn test_list_deleted_accounts() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/accounting/accounts",
        &token,
    )
    .set_json(json!({
        "code": "LST-DEL",
        "name": "List Deleted Test",
        "account_type": "Equity",
        "sub_type": "OwnersEquity",
        "company_id": 1,
        "parent_id": null,
        "allow_transaction": true,
        "tenant_id": 1
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Delete
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/accounting/accounts/{}", id),
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    // List deleted
    let list_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/accounting/accounts/deleted",
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
async fn test_destroy_account_permanently() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/accounting/accounts",
        &token,
    )
    .set_json(json!({
        "code": "DEST-001",
        "name": "Destroy Test Account",
        "account_type": "Asset",
        "sub_type": "FixedAsset",
        "company_id": 1,
        "parent_id": null,
        "allow_transaction": true,
        "tenant_id": 1
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Soft delete first
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/accounting/accounts/{}", id),
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    // Permanently destroy
    let destroy_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/accounting/accounts/{}/destroy", id),
        &token,
    )
    .to_request();
    let destroy_resp = test::call_service(&app, destroy_req).await;
    assert_eq!(destroy_resp.status(), StatusCode::NO_CONTENT);

    // Should not be restorable
    let restore_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/accounting/accounts/{}/restore", id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// Journal Entry CRUD Tests
// ============================================================================

#[actix_web::test]
async fn test_create_journal_entry_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    // Create two accounts first
    let acc1_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/accounting/accounts",
        &token,
    )
    .set_json(json!({
        "code": "5001",
        "name": "Cash Debit",
        "account_type": "Asset",
        "sub_type": "CurrentAsset",
        "company_id": 1,
        "parent_id": null,
        "allow_transaction": true,
        "tenant_id": 1
    }))
    .to_request();
    let acc1_resp = test::call_service(&app, acc1_req).await;
    let body = to_bytes(acc1_resp.into_body()).await.unwrap();
    let acc1_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let account1_id = acc1_json["id"].as_i64().unwrap();

    let acc2_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/accounting/accounts",
        &token,
    )
    .set_json(json!({
        "code": "5002",
        "name": "Revenue Credit",
        "account_type": "Revenue",
        "sub_type": "OperatingRevenue",
        "company_id": 1,
        "parent_id": null,
        "allow_transaction": true,
        "tenant_id": 1
    }))
    .to_request();
    let acc2_resp = test::call_service(&app, acc2_req).await;
    let body = to_bytes(acc2_resp.into_body()).await.unwrap();
    let acc2_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let account2_id = acc2_json["id"].as_i64().unwrap();

    let entry_date = chrono::Utc::now();
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/accounting/journal-entries",
        &token,
    )
    .set_json(json!({
        "date": entry_date.to_rfc3339(),
        "description": "Test journal entry",
        "reference": "REF-001",
        "company_id": 1,
        "tenant_id": 1,
        "created_by": 1,
        "lines": [
            {
                "account_id": account1_id,
                "debit": "100.00",
                "credit": "0.00",
                "description": "Debit line"
            },
            {
                "account_id": account2_id,
                "debit": "0.00",
                "credit": "100.00",
                "description": "Credit line"
            }
        ]
    }))
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "Draft");
    assert_eq!(json["description"], "Test journal entry");
    assert!(json["id"].is_number());
}

#[actix_web::test]
async fn test_list_journal_entries_paginated() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    // Create accounts
    let acc_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/accounting/accounts",
        &token,
    )
    .set_json(json!({
        "code": "6001",
        "name": "Entry Test Asset",
        "account_type": "Asset",
        "sub_type": "CurrentAsset",
        "company_id": 1,
        "parent_id": null,
        "allow_transaction": true,
        "tenant_id": 1
    }))
    .to_request();
    let acc_resp = test::call_service(&app, acc_req).await;
    let body = to_bytes(acc_resp.into_body()).await.unwrap();
    let acc_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let account_id = acc_json["id"].as_i64().unwrap();

    let entry_date = chrono::Utc::now();
    for i in 1..=3 {
        let req = auth_request(
            actix_web::http::Method::POST,
            "/api/v1/accounting/journal-entries",
            &token,
        )
        .set_json(json!({
            "date": entry_date.to_rfc3339(),
            "description": format!("Entry {}", i),
            "company_id": 1,
            "tenant_id": 1,
            "created_by": 1,
            "lines": [
                {
                    "account_id": account_id,
                    "debit": "100.00",
                    "credit": "0.00"
                },
                {
                    "account_id": account_id,
                    "debit": "0.00",
                    "credit": "100.00"
                }
            ]
        }))
        .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/accounting/journal-entries?page=1&per_page=2",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["items"].as_array().unwrap().len(), 2);
    assert_eq!(json["page"], 1);
    assert_eq!(json["per_page"], 2);
}

#[actix_web::test]
async fn test_get_journal_entry_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    // Create account
    let acc_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/accounting/accounts",
        &token,
    )
    .set_json(json!({
        "code": "7001",
        "name": "Get Test Asset",
        "account_type": "Asset",
        "sub_type": "CurrentAsset",
        "company_id": 1,
        "parent_id": null,
        "allow_transaction": true,
        "tenant_id": 1
    }))
    .to_request();
    let acc_resp = test::call_service(&app, acc_req).await;
    let body = to_bytes(acc_resp.into_body()).await.unwrap();
    let acc_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let account_id = acc_json["id"].as_i64().unwrap();

    let entry_date = chrono::Utc::now();
    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/accounting/journal-entries",
        &token,
    )
    .set_json(json!({
        "date": entry_date.to_rfc3339(),
        "description": "Get test entry",
        "company_id": 1,
        "tenant_id": 1,
        "created_by": 1,
        "lines": [
            {
                "account_id": account_id,
                "debit": "50.00",
                "credit": "0.00"
            },
            {
                "account_id": account_id,
                "debit": "0.00",
                "credit": "50.00"
            }
        ]
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/accounting/journal-entries/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);

    let body = to_bytes(get_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);
    assert_eq!(json["description"], "Get test entry");
}

#[actix_web::test]
async fn test_get_journal_entry_not_found() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/accounting/journal-entries/99999",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_post_and_void_journal_entry() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    // Create account
    let acc_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/accounting/accounts",
        &token,
    )
    .set_json(json!({
        "code": "8001",
        "name": "Post Test Asset",
        "account_type": "Asset",
        "sub_type": "CurrentAsset",
        "company_id": 1,
        "parent_id": null,
        "allow_transaction": true,
        "tenant_id": 1
    }))
    .to_request();
    let acc_resp = test::call_service(&app, acc_req).await;
    let body = to_bytes(acc_resp.into_body()).await.unwrap();
    let acc_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let account_id = acc_json["id"].as_i64().unwrap();

    let entry_date = chrono::Utc::now();
    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/accounting/journal-entries",
        &token,
    )
    .set_json(json!({
        "date": entry_date.to_rfc3339(),
        "description": "Post test entry",
        "company_id": 1,
        "tenant_id": 1,
        "created_by": 1,
        "lines": [
            {
                "account_id": account_id,
                "debit": "75.00",
                "credit": "0.00"
            },
            {
                "account_id": account_id,
                "debit": "0.00",
                "credit": "75.00"
            }
        ]
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Post entry
    let post_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/accounting/journal-entries/{}/post", id),
        &token,
    )
    .to_request();
    let post_resp = test::call_service(&app, post_req).await;
    assert_eq!(post_resp.status(), StatusCode::OK);

    let body = to_bytes(post_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "Posted");

    // Void entry
    let void_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/accounting/journal-entries/{}/void", id),
        &token,
    )
    .to_request();
    let void_resp = test::call_service(&app, void_req).await;
    assert_eq!(void_resp.status(), StatusCode::OK);

    let body = to_bytes(void_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "Voided");
}

#[actix_web::test]
async fn test_delete_and_restore_journal_entry() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    // Create account
    let acc_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/accounting/accounts",
        &token,
    )
    .set_json(json!({
        "code": "9001",
        "name": "Del Test Asset",
        "account_type": "Asset",
        "sub_type": "CurrentAsset",
        "company_id": 1,
        "parent_id": null,
        "allow_transaction": true,
        "tenant_id": 1
    }))
    .to_request();
    let acc_resp = test::call_service(&app, acc_req).await;
    let body = to_bytes(acc_resp.into_body()).await.unwrap();
    let acc_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let account_id = acc_json["id"].as_i64().unwrap();

    let entry_date = chrono::Utc::now();
    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/accounting/journal-entries",
        &token,
    )
    .set_json(json!({
        "date": entry_date.to_rfc3339(),
        "description": "Delete test entry",
        "company_id": 1,
        "tenant_id": 1,
        "created_by": 1,
        "lines": [
            {
                "account_id": account_id,
                "debit": "25.00",
                "credit": "0.00"
            },
            {
                "account_id": account_id,
                "debit": "0.00",
                "credit": "25.00"
            }
        ]
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Soft delete
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/accounting/journal-entries/{}", id),
        &token,
    )
    .to_request();
    let del_resp = test::call_service(&app, del_req).await;
    assert_eq!(del_resp.status(), StatusCode::OK);

    // Verify deleted
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/accounting/journal-entries/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);

    // Restore
    let restore_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/accounting/journal-entries/{}/restore", id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::OK);

    let body = to_bytes(restore_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);

    // Verify restored
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/accounting/journal-entries/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn test_list_deleted_journal_entries() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    // Create account
    let acc_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/accounting/accounts",
        &token,
    )
    .set_json(json!({
        "code": "9002",
        "name": "Del List Asset",
        "account_type": "Asset",
        "sub_type": "CurrentAsset",
        "company_id": 1,
        "parent_id": null,
        "allow_transaction": true,
        "tenant_id": 1
    }))
    .to_request();
    let acc_resp = test::call_service(&app, acc_req).await;
    let body = to_bytes(acc_resp.into_body()).await.unwrap();
    let acc_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let account_id = acc_json["id"].as_i64().unwrap();

    let entry_date = chrono::Utc::now();
    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/accounting/journal-entries",
        &token,
    )
    .set_json(json!({
        "date": entry_date.to_rfc3339(),
        "description": "List deleted entry",
        "company_id": 1,
        "tenant_id": 1,
        "created_by": 1,
        "lines": [
            {
                "account_id": account_id,
                "debit": "10.00",
                "credit": "0.00"
            },
            {
                "account_id": account_id,
                "debit": "0.00",
                "credit": "10.00"
            }
        ]
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Delete
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/accounting/journal-entries/{}", id),
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    // List deleted
    let list_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/accounting/journal-entries/deleted",
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
async fn test_destroy_journal_entry_permanently() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    // Create account
    let acc_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/accounting/accounts",
        &token,
    )
    .set_json(json!({
        "code": "9003",
        "name": "Destroy List Asset",
        "account_type": "Asset",
        "sub_type": "CurrentAsset",
        "company_id": 1,
        "parent_id": null,
        "allow_transaction": true,
        "tenant_id": 1
    }))
    .to_request();
    let acc_resp = test::call_service(&app, acc_req).await;
    let body = to_bytes(acc_resp.into_body()).await.unwrap();
    let acc_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let account_id = acc_json["id"].as_i64().unwrap();

    let entry_date = chrono::Utc::now();
    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/accounting/journal-entries",
        &token,
    )
    .set_json(json!({
        "date": entry_date.to_rfc3339(),
        "description": "Destroy test entry",
        "company_id": 1,
        "tenant_id": 1,
        "created_by": 1,
        "lines": [
            {
                "account_id": account_id,
                "debit": "30.00",
                "credit": "0.00"
            },
            {
                "account_id": account_id,
                "debit": "0.00",
                "credit": "30.00"
            }
        ]
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Soft delete first
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/accounting/journal-entries/{}", id),
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    // Permanently destroy
    let destroy_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/accounting/journal-entries/{}/destroy", id),
        &token,
    )
    .to_request();
    let destroy_resp = test::call_service(&app, destroy_req).await;
    assert_eq!(destroy_resp.status(), StatusCode::NO_CONTENT);

    // Should not be restorable
    let restore_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/accounting/journal-entries/{}/restore", id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// Security Tests
// ============================================================================

#[actix_web::test]
async fn test_account_unauthorized() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/accounting/accounts")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_journal_entry_unauthorized() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/accounting/journal-entries")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_create_account_unauthorized() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/accounting/accounts")
        .set_json(json!({
            "code": "UNAUTH",
            "name": "Unauthorized Account",
            "account_type": "Asset",
            "sub_type": "CurrentAsset",
            "company_id": 1,
            "allow_transaction": true,
            "tenant_id": 1
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
