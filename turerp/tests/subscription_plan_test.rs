//! Subscription Plan Integration Tests

use actix_web::{body::to_bytes, http::StatusCode, test};
use serde_json::json;

mod common;
use common::*;

// ============================================================================
// Plan CRUD Tests
// ============================================================================

#[actix_web::test]
async fn test_plan_crud_admin() {
    let app_state = create_test_app_state().await;
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
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, user_id) = register_admin(&app_state, 1).await;
    // Seed a cari so the create-subscription customer_id precheck (cari FK,
    // issue #296) resolves — the InMemory cari repo starts empty.
    let customer_id = seed_cari!(&app, &token, user_id, 1);

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
        "customer_id": customer_id,
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
    assert_eq!(sub["customer_id"], customer_id);
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
    let app_state = create_test_app_state().await;
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
    let app_state = create_test_app_state().await;
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
