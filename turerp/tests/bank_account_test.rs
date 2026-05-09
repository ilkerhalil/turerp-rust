//! Bank Account Integration Tests

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
