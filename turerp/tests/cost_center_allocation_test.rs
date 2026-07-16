//! Cost Center Allocation Integration Tests
use actix_web::{body::to_bytes, http::StatusCode, test};
use serde_json::json;

use crate::common::*;

// ============================================================================
// Allocation Tests
// ============================================================================

#[actix_web::test]
async fn test_create_allocation_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/cost-centers",
        &token,
    )
    .set_json(json!({
        "code": "CC-ALLOC",
        "name": "Allocation Test",
        "center_type": "Profit",
        "is_active": true
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let alloc_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/cost-centers/{}/allocations", id),
        &token,
    )
    .set_json(json!({
        "source_type": "invoice",
        "source_id": 1,
        "cost_center_id": id,
        "amount": "1000.00",
        "percentage": "100.0",
        "description": "Test allocation"
    }))
    .to_request();
    let alloc_resp = test::call_service(&app, alloc_req).await;
    assert_eq!(alloc_resp.status(), StatusCode::CREATED);

    let body = to_bytes(alloc_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["amount"], "1000.00");
    assert_eq!(json["percentage"], "100.0");
    assert_eq!(json["source_type"], "invoice");
    assert_eq!(json["cost_center_id"], id);
}

#[actix_web::test]
async fn test_get_allocations() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/cost-centers",
        &token,
    )
    .set_json(json!({
        "code": "CC-GET-ALLOC",
        "name": "Get Allocations Test",
        "center_type": "Cost",
        "is_active": true
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Create allocation
    let alloc_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/cost-centers/{}/allocations", id),
        &token,
    )
    .set_json(json!({
        "source_type": "payroll",
        "source_id": 2,
        "cost_center_id": id,
        "amount": "500.00",
        "percentage": "50.0"
    }))
    .to_request();
    test::call_service(&app, alloc_req).await;

    // Get allocations
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/cost-centers/{}/allocations", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);

    let body = to_bytes(get_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json.as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["amount"], "500.00");
    assert_eq!(items[0]["source_type"], "payroll");
}

#[actix_web::test]
async fn test_get_profitability_report() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/cost-centers",
        &token,
    )
    .set_json(json!({
        "code": "CC-PROF",
        "name": "Profitability Test",
        "center_type": "Profit",
        "is_active": true
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Income allocation
    let income_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/cost-centers/{}/allocations", id),
        &token,
    )
    .set_json(json!({
        "source_type": "invoice",
        "source_id": 1,
        "cost_center_id": id,
        "amount": "10000.00",
        "percentage": "100.0"
    }))
    .to_request();
    test::call_service(&app, income_req).await;

    // Expense allocation
    let expense_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/cost-centers/{}/allocations", id),
        &token,
    )
    .set_json(json!({
        "source_type": "payroll",
        "source_id": 2,
        "cost_center_id": id,
        "amount": "3000.00",
        "percentage": "100.0"
    }))
    .to_request();
    test::call_service(&app, expense_req).await;

    // Get profitability report
    let report_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/cost-centers/{}/profitability", id),
        &token,
    )
    .to_request();
    let report_resp = test::call_service(&app, report_req).await;
    assert_eq!(report_resp.status(), StatusCode::OK);

    let body = to_bytes(report_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["cost_center_id"], id);
    assert_eq!(json["cost_center_code"], "CC-PROF");
    assert_eq!(json["total_income"], "10000.00");
    assert_eq!(json["total_expense"], "3000.00");
    assert_eq!(json["net_profit"], "7000.00");
    assert_eq!(json["allocation_count"], 2);
}

// ============================================================================
// Authorization Tests
// ============================================================================

#[actix_web::test]
async fn test_create_cost_center_requires_admin() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (user_token, _user_id) = register_user!(&app, 1);

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/cost-centers",
        &user_token,
    )
    .set_json(json!({
        "code": "CC-USER",
        "name": "User Attempt",
        "center_type": "Cost",
        "is_active": true
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_web::test]
async fn test_update_cost_center_requires_admin() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (admin_token, _admin_id) = register_admin(&state, 1).await;
    let (user_token, _user_id) = register_user!(&app, 1);

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/cost-centers",
        &admin_token,
    )
    .set_json(json!({
        "code": "CC-UPD-ADM",
        "name": "Update Admin Test",
        "center_type": "Cost",
        "is_active": true
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let update_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/cost-centers/{}", id),
        &user_token,
    )
    .set_json(json!({ "name": "Hacked" }))
    .to_request();
    let update_resp = test::call_service(&app, update_req).await;
    assert_eq!(update_resp.status(), StatusCode::FORBIDDEN);
}

#[actix_web::test]
async fn test_delete_cost_center_requires_admin() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (admin_token, _admin_id) = register_admin(&state, 1).await;
    let (user_token, _user_id) = register_user!(&app, 1);

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/cost-centers",
        &admin_token,
    )
    .set_json(json!({
        "code": "CC-DEL-ADM",
        "name": "Delete Admin Test",
        "center_type": "Cost",
        "is_active": true
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/cost-centers/{}", id),
        &user_token,
    )
    .to_request();
    let del_resp = test::call_service(&app, del_req).await;
    assert_eq!(del_resp.status(), StatusCode::FORBIDDEN);
}

// ============================================================================
// Tenant Isolation Tests
// ============================================================================

#[actix_web::test]
async fn test_tenant_isolation() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token_t1, _user_id_t1) = register_admin(&state, 1).await;
    let (token_t2, _user_id_t2) = register_admin(&state, 2).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/cost-centers",
        &token_t1,
    )
    .set_json(json!({
        "code": "CC-T1",
        "name": "Tenant 1 Center",
        "center_type": "Cost",
        "is_active": true
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Tenant 2 should not see tenant 1's cost center
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/cost-centers/{}", id),
        &token_t2,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);

    // Tenant 2 list should be empty
    let list_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/cost-centers",
        &token_t2,
    )
    .to_request();
    let list_resp = test::call_service(&app, list_req).await;
    assert_eq!(list_resp.status(), StatusCode::OK);

    let body = to_bytes(list_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["items"].as_array().unwrap().len(), 0);
}

// ============================================================================
// Validation Tests
// ============================================================================

#[actix_web::test]
async fn test_create_cost_center_validation_empty_code() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/cost-centers",
        &token,
    )
    .set_json(json!({
        "code": "   ",
        "name": "Valid Name",
        "center_type": "Cost",
        "is_active": true
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["error"].as_str().unwrap().contains("Code"));
}

#[actix_web::test]
async fn test_create_cost_center_validation_empty_name() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/cost-centers",
        &token,
    )
    .set_json(json!({
        "code": "CC-VALID",
        "name": "   ",
        "center_type": "Cost",
        "is_active": true
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["error"].as_str().unwrap().contains("Name"));
}

#[actix_web::test]
async fn test_create_allocation_validation_negative_amount() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/cost-centers",
        &token,
    )
    .set_json(json!({
        "code": "CC-ALLOC-VAL",
        "name": "Allocation Validation Test",
        "center_type": "Cost",
        "is_active": true
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let alloc_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/cost-centers/{}/allocations", id),
        &token,
    )
    .set_json(json!({
        "source_type": "invoice",
        "source_id": 1,
        "cost_center_id": id,
        "amount": "-100.00",
        "percentage": "100.0"
    }))
    .to_request();
    let alloc_resp = test::call_service(&app, alloc_req).await;
    assert_eq!(alloc_resp.status(), StatusCode::BAD_REQUEST);

    let body = to_bytes(alloc_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["error"].as_str().unwrap().contains("Amount"));
}

#[actix_web::test]
async fn test_create_allocation_validation_bad_percentage() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/cost-centers",
        &token,
    )
    .set_json(json!({
        "code": "CC-ALLOC-PCT",
        "name": "Allocation Pct Test",
        "center_type": "Cost",
        "is_active": true
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let alloc_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/cost-centers/{}/allocations", id),
        &token,
    )
    .set_json(json!({
        "source_type": "invoice",
        "source_id": 1,
        "cost_center_id": id,
        "amount": "100.00",
        "percentage": "101.0"
    }))
    .to_request();
    let alloc_resp = test::call_service(&app, alloc_req).await;
    assert_eq!(alloc_resp.status(), StatusCode::BAD_REQUEST);

    let body = to_bytes(alloc_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["error"].as_str().unwrap().contains("Percentage"));
}

#[actix_web::test]
async fn test_bulk_restore_validation_empty_ids() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/cost-centers/bulk-restore",
        &token,
    )
    .set_json(json!({ "ids": [] }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["error"].as_str().unwrap().contains("empty"));
}
