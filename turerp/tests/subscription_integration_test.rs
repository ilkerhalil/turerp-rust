//! Subscription Integration Tests
//!
//! Run with: cargo test --test subscription_integration_test

use actix_web::body::to_bytes;
use actix_web::http::StatusCode;
use actix_web::test;
use serde_json::json;

mod common;
use common::*;

// ============================================================================
// Plan CRUD Tests
// ============================================================================

#[actix_web::test]
async fn test_plan_crud_admin() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    // Create plan
    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/subscription-plans",
        &token,
    )
    .set_json(json!({
        "name": "Pro Plan",
        "description": "Professional tier",
        "billing_cycle": "monthly",
        "base_amount": "99.99",
        "currency": "TRY",
        "is_active": true,
        "tenant_id": 1
    }))
    .to_request();

    let resp = test::call_service(&app, create_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let plan_id = created["id"].as_i64().unwrap();
    assert_eq!(created["name"], "Pro Plan");
    assert_eq!(created["billing_cycle"], "monthly");
    assert_eq!(created["currency"], "TRY");

    // List plans
    let list_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/subscription-plans",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, list_req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let list: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(list.as_array().unwrap().len() >= 1);

    // Get plan by ID
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/subscription-plans/{}", plan_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, get_req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let got: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(got["name"], "Pro Plan");

    // Update plan
    let update_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/subscription-plans/{}", plan_id),
        &token,
    )
    .set_json(json!({
        "name": "Pro Plan Updated",
        "base_amount": "149.99"
    }))
    .to_request();
    let resp = test::call_service(&app, update_req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let updated: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(updated["name"], "Pro Plan Updated");
    assert_eq!(updated["base_amount"], "149.99");

    // Delete plan
    let delete_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/subscription-plans/{}", plan_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, delete_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Verify deletion (soft delete -> 404)
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/subscription-plans/{}", plan_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, get_req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// Subscription CRUD Tests
// ============================================================================

#[actix_web::test]
async fn test_subscription_crud() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    // Create a plan first
    let plan_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/subscription-plans",
        &token,
    )
    .set_json(json!({
        "name": "Basic Plan",
        "billing_cycle": "monthly",
        "base_amount": "49.99",
        "currency": "TRY",
        "is_active": true,
        "tenant_id": 1
    }))
    .to_request();
    let resp = test::call_service(&app, plan_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let plan: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let plan_id = plan["id"].as_i64().unwrap();

    // Create subscription
    let sub_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/subscriptions",
        &token,
    )
    .set_json(json!({
        "customer_id": 1,
        "plan_id": plan_id,
        "start_date": "2024-01-01",
        "status": "active",
        "auto_renew": true,
        "next_billing_date": "2024-02-01",
        "tenant_id": 1
    }))
    .to_request();
    let resp = test::call_service(&app, sub_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let sub: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let sub_id = sub["id"].as_i64().unwrap();
    assert_eq!(sub["customer_id"], 1);
    assert_eq!(sub["plan_id"], plan_id);
    assert_eq!(sub["status"], "active");

    // List subscriptions
    let list_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/subscriptions",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, list_req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let list: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(list.as_array().unwrap().len() >= 1);

    // Get subscription by ID
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/subscriptions/{}", sub_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, get_req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let got: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(got["id"], sub_id);

    // Update subscription
    let update_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/subscriptions/{}", sub_id),
        &token,
    )
    .set_json(json!({
        "auto_renew": false,
        "status": "cancelled"
    }))
    .to_request();
    let resp = test::call_service(&app, update_req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let updated: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(updated["auto_renew"], false);
    assert_eq!(updated["status"], "cancelled");

    // Delete subscription
    let delete_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/subscriptions/{}", sub_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, delete_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Verify soft delete
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/subscriptions/{}", sub_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, get_req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// Admin Authorization Tests
// ============================================================================

#[actix_web::test]
async fn test_plan_create_requires_admin() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;
    let (user_token, _) = register_user!(&app, 1);

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/subscription-plans",
        &user_token,
    )
    .set_json(json!({
        "name": "Should Fail",
        "billing_cycle": "monthly",
        "base_amount": "10.00",
        "currency": "TRY",
        "tenant_id": 1
    }))
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_web::test]
async fn test_plan_update_delete_requires_admin() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;
    let (admin_token, _) = register_admin(&app_state, 1).await;
    let (user_token, _) = register_user!(&app, 1);

    // Admin creates a plan
    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/subscription-plans",
        &admin_token,
    )
    .set_json(json!({
        "name": "Enterprise",
        "billing_cycle": "yearly",
        "base_amount": "999.99",
        "currency": "TRY",
        "tenant_id": 1
    }))
    .to_request();
    let resp = test::call_service(&app, create_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let plan: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let plan_id = plan["id"].as_i64().unwrap();

    // Normal user tries to update
    let update_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/subscription-plans/{}", plan_id),
        &user_token,
    )
    .set_json(json!({"name": "Hacked"}))
    .to_request();
    let resp = test::call_service(&app, update_req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // Normal user tries to delete
    let delete_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/subscription-plans/{}", plan_id),
        &user_token,
    )
    .to_request();
    let resp = test::call_service(&app, delete_req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

// ============================================================================
// Tenant Isolation Tests
// ============================================================================

#[actix_web::test]
async fn test_subscription_tenant_isolation() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;

    // Admin in tenant 1
    let (token_t1, _) = register_admin(&app_state, 1).await;
    // Admin in tenant 2
    let (token_t2, _) = register_admin(&app_state, 2).await;

    // Create plan in tenant 1
    let plan_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/subscription-plans",
        &token_t1,
    )
    .set_json(json!({
        "name": "Tenant1 Plan",
        "billing_cycle": "monthly",
        "base_amount": "50.00",
        "currency": "TRY",
        "tenant_id": 1
    }))
    .to_request();
    let resp = test::call_service(&app, plan_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let plan: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let plan_id = plan["id"].as_i64().unwrap();

    // Tenant 2 cannot see tenant 1's plan
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/subscription-plans/{}", plan_id),
        &token_t2,
    )
    .to_request();
    let resp = test::call_service(&app, get_req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // Create subscription in tenant 1
    let sub_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/subscriptions",
        &token_t1,
    )
    .set_json(json!({
        "customer_id": 1,
        "plan_id": plan_id,
        "start_date": "2024-01-01",
        "status": "active",
        "auto_renew": true,
        "next_billing_date": "2024-02-01",
        "tenant_id": 1
    }))
    .to_request();
    let resp = test::call_service(&app, sub_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let sub: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let sub_id = sub["id"].as_i64().unwrap();

    // Tenant 2 cannot see tenant 1's subscription
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/subscriptions/{}", sub_id),
        &token_t2,
    )
    .to_request();
    let resp = test::call_service(&app, get_req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // Tenant 2 list is empty for both plans and subscriptions
    let list_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/subscription-plans",
        &token_t2,
    )
    .to_request();
    let resp = test::call_service(&app, list_req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let list: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(list.as_array().unwrap().len(), 0);
}

// ============================================================================
// Billing Cycle & Renewal Tests
// ============================================================================

#[actix_web::test]
async fn test_renew_subscription() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    // Create plan
    let plan_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/subscription-plans",
        &token,
    )
    .set_json(json!({
        "name": "Monthly Plan",
        "billing_cycle": "monthly",
        "base_amount": "100.00",
        "currency": "TRY",
        "tenant_id": 1
    }))
    .to_request();
    let resp = test::call_service(&app, plan_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let plan: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let plan_id = plan["id"].as_i64().unwrap();

    // Create subscription with end_date
    let sub_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/subscriptions",
        &token,
    )
    .set_json(json!({
        "customer_id": 1,
        "plan_id": plan_id,
        "start_date": "2024-01-01",
        "end_date": "2024-02-01",
        "status": "active",
        "auto_renew": true,
        "next_billing_date": "2024-02-01",
        "tenant_id": 1
    }))
    .to_request();
    let resp = test::call_service(&app, sub_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let sub: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let sub_id = sub["id"].as_i64().unwrap();

    // Renew subscription
    let renew_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/subscriptions/{}/renew", sub_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, renew_req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let renewed: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(renewed["last_billed_at"].is_string());

    // Check invoices were created
    let invoices_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/subscriptions/{}/invoices", sub_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, invoices_req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let invoices: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(invoices.as_array().unwrap().len() >= 1);
}

#[actix_web::test]
async fn test_due_for_billing() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    // Create plan
    let plan_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/subscription-plans",
        &token,
    )
    .set_json(json!({
        "name": "Quarterly Plan",
        "billing_cycle": "quarterly",
        "base_amount": "250.00",
        "currency": "TRY",
        "tenant_id": 1
    }))
    .to_request();
    let resp = test::call_service(&app, plan_req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let plan: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let plan_id = plan["id"].as_i64().unwrap();

    // Create subscription due for billing (next_billing_date in the past)
    let sub_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/subscriptions",
        &token,
    )
    .set_json(json!({
        "customer_id": 1,
        "plan_id": plan_id,
        "start_date": "2024-01-01",
        "status": "active",
        "auto_renew": true,
        "next_billing_date": "2024-01-15",
        "tenant_id": 1
    }))
    .to_request();
    let resp = test::call_service(&app, sub_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Query due for billing with a future cutoff date
    let due_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/subscriptions/due-for-billing?date=2024-01-20",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, due_req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let due: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(due.as_array().unwrap().len() >= 1);

    // Query with a past cutoff date - should not include the subscription
    let due_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/subscriptions/due-for-billing?date=2024-01-10",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, due_req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let due: serde_json::Value = serde_json::from_slice(&body).unwrap();
    // The subscription's next_billing_date is 2024-01-15, which is > 2024-01-10
    assert_eq!(due.as_array().unwrap().len(), 0);
}

// ============================================================================
// Validation Error Tests
// ============================================================================

#[actix_web::test]
async fn test_subscription_validation_errors() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    // Missing required fields
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/subscriptions",
        &token,
    )
    .set_json(json!({
        "plan_id": 1
        // missing customer_id, start_date
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // Invalid plan validation (empty name)
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/subscription-plans",
        &token,
    )
    .set_json(json!({
        "name": "",
        "billing_cycle": "monthly",
        "base_amount": "10.00",
        "currency": "TRY"
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// ============================================================================
// Soft Delete Tests
// ============================================================================

#[actix_web::test]
async fn test_plan_soft_delete() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    // Create plan
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/subscription-plans",
        &token,
    )
    .set_json(json!({
        "name": "Soft Delete Plan",
        "billing_cycle": "monthly",
        "base_amount": "10.00",
        "currency": "TRY",
        "tenant_id": 1
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let plan: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let plan_id = plan["id"].as_i64().unwrap();

    // Delete
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/subscription-plans/{}", plan_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Get returns 404
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/subscription-plans/{}", plan_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // List no longer includes it
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/subscription-plans",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let list: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let ids: Vec<i64> = list
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|p| p["id"].as_i64())
        .collect();
    assert!(!ids.contains(&plan_id));
}

#[actix_web::test]
async fn test_subscription_soft_delete() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    // Create plan first
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/subscription-plans",
        &token,
    )
    .set_json(json!({
        "name": "Sub Soft Delete Plan",
        "billing_cycle": "monthly",
        "base_amount": "10.00",
        "currency": "TRY",
        "tenant_id": 1
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let plan: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let plan_id = plan["id"].as_i64().unwrap();

    // Create subscription
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/subscriptions",
        &token,
    )
    .set_json(json!({
        "customer_id": 1,
        "plan_id": plan_id,
        "start_date": "2024-01-01",
        "status": "active",
        "auto_renew": true,
        "tenant_id": 1
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let sub: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let sub_id = sub["id"].as_i64().unwrap();

    // Delete
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/subscriptions/{}", sub_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Get returns 404
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/subscriptions/{}", sub_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // List no longer includes it
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/subscriptions",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let list: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let ids: Vec<i64> = list
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|s| s["id"].as_i64())
        .collect();
    assert!(!ids.contains(&sub_id));
}

// ============================================================================
// Business Rule Tests
// ============================================================================

#[actix_web::test]
async fn test_delete_plan_with_active_subscriptions_fails() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    // Create plan
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/subscription-plans",
        &token,
    )
    .set_json(json!({
        "name": "Protected Plan",
        "billing_cycle": "monthly",
        "base_amount": "75.00",
        "currency": "TRY",
        "tenant_id": 1
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let plan: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let plan_id = plan["id"].as_i64().unwrap();

    // Create subscription using that plan
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/subscriptions",
        &token,
    )
    .set_json(json!({
        "customer_id": 1,
        "plan_id": plan_id,
        "start_date": "2024-01-01",
        "status": "active",
        "auto_renew": true,
        "tenant_id": 1
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Try to delete the plan - should fail because of active subscription
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/subscription-plans/{}", plan_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // Verify plan still exists
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/subscription-plans/{}", plan_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

// ============================================================================
// Unauthenticated Tests
// ============================================================================

#[actix_web::test]
async fn test_subscription_endpoints_require_auth() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;

    let endpoints = vec![
        ("GET", "/api/v1/subscription-plans"),
        ("POST", "/api/v1/subscription-plans"),
        ("GET", "/api/v1/subscription-plans/1"),
        ("PUT", "/api/v1/subscription-plans/1"),
        ("DELETE", "/api/v1/subscription-plans/1"),
        ("GET", "/api/v1/subscriptions"),
        ("POST", "/api/v1/subscriptions"),
        ("GET", "/api/v1/subscriptions/1"),
        ("PUT", "/api/v1/subscriptions/1"),
        ("DELETE", "/api/v1/subscriptions/1"),
        ("POST", "/api/v1/subscriptions/1/renew"),
        ("GET", "/api/v1/subscriptions/due-for-billing"),
        ("GET", "/api/v1/subscriptions/1/invoices"),
    ];

    for (method, uri) in endpoints {
        let req = match method {
            "GET" => test::TestRequest::get().uri(uri).to_request(),
            "POST" => test::TestRequest::post().uri(uri).to_request(),
            "PUT" => test::TestRequest::put().uri(uri).to_request(),
            "DELETE" => test::TestRequest::delete().uri(uri).to_request(),
            _ => panic!("Unknown method"),
        };
        let resp = test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::UNAUTHORIZED,
            "{} {} should require authentication",
            method,
            uri
        );
    }
}

// ============================================================================
// Dedicated Update Tests
// ============================================================================

#[actix_web::test]
async fn test_update_subscription() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    // Create initial plan
    let plan_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/subscription-plans",
        &token,
    )
    .set_json(json!({
        "name": "Initial Plan",
        "billing_cycle": "monthly",
        "base_amount": "49.99",
        "currency": "TRY",
        "is_active": true,
        "tenant_id": 1
    }))
    .to_request();
    let resp = test::call_service(&app, plan_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let plan: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let plan_id = plan["id"].as_i64().unwrap();

    // Create second plan to switch to
    let plan2_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/subscription-plans",
        &token,
    )
    .set_json(json!({
        "name": "Upgraded Plan",
        "billing_cycle": "yearly",
        "base_amount": "499.99",
        "currency": "TRY",
        "is_active": true,
        "tenant_id": 1
    }))
    .to_request();
    let resp = test::call_service(&app, plan2_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let plan2: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let plan2_id = plan2["id"].as_i64().unwrap();

    // Create subscription
    let sub_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/subscriptions",
        &token,
    )
    .set_json(json!({
        "customer_id": 1,
        "plan_id": plan_id,
        "start_date": "2024-01-01",
        "status": "active",
        "auto_renew": true,
        "tenant_id": 1
    }))
    .to_request();
    let resp = test::call_service(&app, sub_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let sub: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let sub_id = sub["id"].as_i64().unwrap();
    assert_eq!(sub["plan_id"], plan_id);
    assert_eq!(sub["status"], "active");
    assert_eq!(sub["auto_renew"], true);

    // Update subscription: change plan_id, status, and end_date
    let update_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/subscriptions/{}", sub_id),
        &token,
    )
    .set_json(json!({
        "plan_id": plan2_id,
        "status": "expired",
        "end_date": "2024-12-31",
        "auto_renew": false
    }))
    .to_request();
    let resp = test::call_service(&app, update_req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let updated: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(updated["id"], sub_id);
    assert_eq!(updated["plan_id"], plan2_id);
    assert_eq!(updated["status"], "expired");
    assert_eq!(updated["auto_renew"], false);
    assert_eq!(updated["end_date"], "2024-12-31");

    // Verify by GET
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/subscriptions/{}", sub_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, get_req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let got: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(got["plan_id"], plan2_id);
    assert_eq!(got["status"], "expired");
    assert_eq!(got["auto_renew"], false);
    assert_eq!(got["end_date"], "2024-12-31");
}
