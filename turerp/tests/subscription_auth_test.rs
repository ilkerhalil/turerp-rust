//! Subscription Auth & Update Integration Tests

use actix_web::{body::to_bytes, http::StatusCode, test};
use serde_json::json;

mod common;
use common::*;

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
