//! Workflow Auth & Tenant Isolation Integration Tests

use actix_web::{body::to_bytes, http::StatusCode, test};
use serde_json::json;

mod common;
use common::*;

// Tenant Isolation Tests
// ============================================================================

#[actix_web::test]
async fn test_tenant_isolation_templates() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;

    let (token1, _) = register_admin(&app_state, 1).await;
    let (token2, _) = register_admin(&app_state, 2).await;

    // Create template in tenant 1
    let template_id = create_workflow_template!(&app, &token1);

    // Tenant 1 should see the template
    let req1 = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/workflows/templates",
        &token1,
    )
    .to_request();

    let resp1 = test::call_service(&app, req1).await;
    let body1 = to_bytes(resp1.into_body()).await.unwrap();
    let json1: serde_json::Value = serde_json::from_slice(&body1).unwrap();
    let templates1 = json1.as_array().unwrap();
    assert!(templates1.iter().any(|t| t["id"] == template_id));

    // Tenant 2 should not see the template
    let req2 = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/workflows/templates",
        &token2,
    )
    .to_request();

    let resp2 = test::call_service(&app, req2).await;
    let body2 = to_bytes(resp2.into_body()).await.unwrap();
    let json2: serde_json::Value = serde_json::from_slice(&body2).unwrap();
    let templates2 = json2.as_array().unwrap();
    assert!(!templates2.iter().any(|t| t["id"] == template_id));
}

#[actix_web::test]
async fn test_tenant_isolation_instances() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;

    let (token1, _) = register_admin(&app_state, 1).await;
    let (token2, _) = register_admin(&app_state, 2).await;

    let template_id = create_workflow_template!(&app, &token1);

    // Start workflow in tenant 1
    let start_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/workflows/instances",
        &token1,
    )
    .set_json(json!({
        "template_id": template_id,
        "entity_id": 900,
        "entity_type": "purchase_order"
    }))
    .to_request();

    let start_resp = test::call_service(&app, start_req).await;
    let body = to_bytes(start_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let instance_id = json["id"].as_i64().unwrap();

    // Tenant 2 should not be able to access the instance
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/workflows/instances/{}", instance_id),
        &token2,
    )
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// Authorization Tests
// ============================================================================

#[actix_web::test]
async fn test_unauthorized_access() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;

    // No auth token
    let req = test::TestRequest::get()
        .uri("/api/v1/workflows/templates")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // No auth token for instance creation
    let req = test::TestRequest::post()
        .uri("/api/v1/workflows/instances")
        .set_json(json!({"template_id": 1, "entity_id": 1, "entity_type": "invoice"}))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_normal_user_can_start_workflow() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;

    let (admin_token, _) = register_admin(&app_state, 1).await;
    let (user_token, _) = register_user!(&app, 1);

    let template_id = create_workflow_template!(&app, &admin_token);

    // Normal user can start a workflow
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/workflows/instances",
        &user_token,
    )
    .set_json(json!({
        "template_id": template_id,
        "entity_id": 1000,
        "entity_type": "purchase_order"
    }))
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
}

// ============================================================================
// Validation Tests
// ============================================================================

#[actix_web::test]
async fn test_start_workflow_missing_fields() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;

    let (token, _) = register_admin(&app_state, 1).await;

    // Missing entity_type
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/workflows/instances",
        &token,
    )
    .set_json(json!({
        "template_id": 1,
        "entity_id": 100
    }))
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}
