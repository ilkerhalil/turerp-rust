//! Workflow Template Integration Tests

use actix_web::{body::to_bytes, http::StatusCode, test};
use serde_json::json;

use crate::common::*;

// ============================================================================
// Template CRUD Tests
// ============================================================================

#[actix_web::test]
async fn test_create_template_admin() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;

    let (token, _) = register_admin(&app_state, 1).await;

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/workflows/templates",
        &token,
    )
    .set_json(json!({
        "name": "Invoice Verification",
        "description": "2-step invoice verification",
        "entity_type": "invoice",
        "config_json": {
            "steps": [
                {"step_number": 1, "step_name": "Accountant Check", "approver_role": "accountant"},
                {"step_number": 2, "step_name": "Admin Approval", "approver_role": "admin"}
            ]
        }
    }))
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["id"].is_i64());
    assert_eq!(json["name"], "Invoice Verification");
    assert_eq!(json["entity_type"], "invoice");
    assert_eq!(json["tenant_id"], 1);
}

#[actix_web::test]
async fn test_create_template_user_forbidden() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;

    let (token, _) = register_user!(&app, 1);

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/workflows/templates",
        &token,
    )
    .set_json(json!({
        "name": "Should Fail",
        "description": "This should be forbidden",
        "entity_type": "invoice",
        "config_json": {"steps": []}
    }))
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_web::test]
async fn test_list_templates() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;

    let (token, _) = register_admin(&app_state, 1).await;

    // Create a template first
    let _template_id = create_workflow_template!(&app, &token);

    // List templates
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/workflows/templates",
        &token,
    )
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.is_array());
    let templates = json.as_array().unwrap();
    assert!(!templates.is_empty());
    assert_eq!(templates[0]["name"], "Purchase Order Approval");
}

#[actix_web::test]
async fn test_list_templates_unauthorized() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/workflows/templates")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ============================================================================
