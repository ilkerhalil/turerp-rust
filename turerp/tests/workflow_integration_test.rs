//! Workflow Integration Tests
//!
//! Run with: cargo test --test workflow_integration_test

use actix_web::{body::to_bytes, http::StatusCode, test};
use serde_json::json;

mod common;
use common::*;

/// Helper macro to create a workflow template and return its ID
/// Usage: `let template_id = create_workflow_template!(&app, &token);`
macro_rules! create_workflow_template {
    ($app:expr, $token:expr) => {{
        let req = auth_request(
            actix_web::http::Method::POST,
            "/api/v1/workflows/templates",
            $token,
        )
        .set_json(json!({
            "name": "Purchase Order Approval",
            "description": "2-step approval workflow",
            "entity_type": "purchase_order",
            "config_json": {
                "steps": [
                    {"step_number": 1, "step_name": "Manager Review", "approver_role": "manager"},
                    {"step_number": 2, "step_name": "Admin Approval", "approver_role": "admin"}
                ]
            }
        }))
        .to_request();

        let resp = test::call_service($app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED, "Template creation failed");

        let body = to_bytes(resp.into_body()).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        json["id"].as_i64().unwrap()
    }};
}

// ============================================================================
// Template CRUD Tests
// ============================================================================

#[actix_web::test]
async fn test_create_template_admin() {
    let app_state = create_test_app_state();
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
    let app_state = create_test_app_state();
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
    let app_state = create_test_app_state();
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
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/workflows/templates")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ============================================================================
// Workflow Instance Tests
// ============================================================================

#[actix_web::test]
async fn test_start_workflow() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;

    let (token, user_id) = register_admin(&app_state, 1).await;
    let template_id = create_workflow_template!(&app, &token);

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/workflows/instances",
        &token,
    )
    .set_json(json!({
        "template_id": template_id,
        "entity_id": 100,
        "entity_type": "purchase_order"
    }))
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["id"].is_i64());
    assert_eq!(json["template_id"], template_id);
    assert_eq!(json["entity_id"], 100);
    assert_eq!(json["entity_type"], "purchase_order");
    assert_eq!(json["status"], "pending");
    assert_eq!(json["current_step"], 1);
    assert_eq!(json["tenant_id"], 1);
    assert_eq!(json["created_by"], user_id);
}

#[actix_web::test]
async fn test_start_workflow_invalid_entity_type() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;

    let (token, _) = register_admin(&app_state, 1).await;
    let template_id = create_workflow_template!(&app, &token);

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/workflows/instances",
        &token,
    )
    .set_json(json!({
        "template_id": template_id,
        "entity_id": 100,
        "entity_type": "invalid_type"
    }))
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn test_start_workflow_mismatched_entity_type() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;

    let (token, _) = register_admin(&app_state, 1).await;
    let template_id = create_workflow_template!(&app, &token);

    // Template is for purchase_order, but we request invoice
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/workflows/instances",
        &token,
    )
    .set_json(json!({
        "template_id": template_id,
        "entity_id": 100,
        "entity_type": "invoice"
    }))
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// ============================================================================
// Step Approval / Rejection Tests
// ============================================================================

#[actix_web::test]
async fn test_approve_step() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;

    let (token, _) = register_admin(&app_state, 1).await;
    let template_id = create_workflow_template!(&app, &token);

    // Start workflow
    let start_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/workflows/instances",
        &token,
    )
    .set_json(json!({
        "template_id": template_id,
        "entity_id": 200,
        "entity_type": "purchase_order"
    }))
    .to_request();

    let start_resp = test::call_service(&app, start_req).await;
    let body = to_bytes(start_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let instance_id = json["id"].as_i64().unwrap();

    // Approve step
    let approve_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/workflows/instances/{}/approve", instance_id),
        &token,
    )
    .set_json(json!({"comment": "Looks good"}))
    .to_request();

    let resp = test::call_service(&app, approve_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "pending");
    assert_eq!(json["current_step"], 2);
}

#[actix_web::test]
async fn test_approve_all_steps_to_completion() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;

    let (token, _) = register_admin(&app_state, 1).await;
    let template_id = create_workflow_template!(&app, &token);

    // Start workflow
    let start_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/workflows/instances",
        &token,
    )
    .set_json(json!({
        "template_id": template_id,
        "entity_id": 300,
        "entity_type": "purchase_order"
    }))
    .to_request();

    let start_resp = test::call_service(&app, start_req).await;
    let body = to_bytes(start_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let instance_id = json["id"].as_i64().unwrap();

    // Approve step 1
    let approve1_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/workflows/instances/{}/approve", instance_id),
        &token,
    )
    .set_json(json!({"comment": "Step 1 approved"}))
    .to_request();

    let resp = test::call_service(&app, approve1_req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "pending");
    assert_eq!(json["current_step"], 2);

    // Approve step 2
    let approve2_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/workflows/instances/{}/approve", instance_id),
        &token,
    )
    .set_json(json!({"comment": "Step 2 approved"}))
    .to_request();

    let resp = test::call_service(&app, approve2_req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "completed");
    assert!(json["completed_at"].is_string());
}

#[actix_web::test]
async fn test_reject_step() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;

    let (token, _) = register_admin(&app_state, 1).await;
    let template_id = create_workflow_template!(&app, &token);

    // Start workflow
    let start_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/workflows/instances",
        &token,
    )
    .set_json(json!({
        "template_id": template_id,
        "entity_id": 400,
        "entity_type": "purchase_order"
    }))
    .to_request();

    let start_resp = test::call_service(&app, start_req).await;
    let body = to_bytes(start_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let instance_id = json["id"].as_i64().unwrap();

    // Reject step
    let reject_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/workflows/instances/{}/reject", instance_id),
        &token,
    )
    .set_json(json!({"comment": "Missing documentation"}))
    .to_request();

    let resp = test::call_service(&app, reject_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "rejected");
}

#[actix_web::test]
async fn test_resubmit_workflow() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;

    let (token, _) = register_admin(&app_state, 1).await;
    let template_id = create_workflow_template!(&app, &token);

    // Start workflow
    let start_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/workflows/instances",
        &token,
    )
    .set_json(json!({
        "template_id": template_id,
        "entity_id": 500,
        "entity_type": "purchase_order"
    }))
    .to_request();

    let start_resp = test::call_service(&app, start_req).await;
    let body = to_bytes(start_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let instance_id = json["id"].as_i64().unwrap();

    // Reject step
    let reject_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/workflows/instances/{}/reject", instance_id),
        &token,
    )
    .set_json(json!({"comment": "Rejected initially"}))
    .to_request();

    let _ = test::call_service(&app, reject_req).await;

    // Resubmit
    let resubmit_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/workflows/instances/{}/resubmit", instance_id),
        &token,
    )
    .to_request();

    let resp = test::call_service(&app, resubmit_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "pending");
    assert_eq!(json["current_step"], 1);
}

#[actix_web::test]
async fn test_approve_nonexistent_instance() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;

    let (token, _) = register_admin(&app_state, 1).await;

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/workflows/instances/99999/approve",
        &token,
    )
    .set_json(json!({"comment": "Should fail"}))
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// Instance Query Tests
// ============================================================================

#[actix_web::test]
async fn test_get_instance() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;

    let (token, user_id) = register_admin(&app_state, 1).await;
    let template_id = create_workflow_template!(&app, &token);

    // Start workflow
    let start_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/workflows/instances",
        &token,
    )
    .set_json(json!({
        "template_id": template_id,
        "entity_id": 600,
        "entity_type": "purchase_order"
    }))
    .to_request();

    let start_resp = test::call_service(&app, start_req).await;
    let body = to_bytes(start_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let instance_id = json["id"].as_i64().unwrap();

    // Get instance details
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/workflows/instances/{}", instance_id),
        &token,
    )
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], instance_id);
    assert_eq!(json["created_by"], user_id);
    assert!(json["steps"].is_array());
    assert_eq!(json["steps"].as_array().unwrap().len(), 2);
    assert!(json["audit_log"].is_array());
}

#[actix_web::test]
async fn test_get_instance_not_found() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;

    let (token, _) = register_admin(&app_state, 1).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/workflows/instances/99999",
        &token,
    )
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_get_instance_audit() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;

    let (token, _) = register_admin(&app_state, 1).await;
    let template_id = create_workflow_template!(&app, &token);

    // Start workflow
    let start_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/workflows/instances",
        &token,
    )
    .set_json(json!({
        "template_id": template_id,
        "entity_id": 700,
        "entity_type": "purchase_order"
    }))
    .to_request();

    let start_resp = test::call_service(&app, start_req).await;
    let body = to_bytes(start_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let instance_id = json["id"].as_i64().unwrap();

    // Get audit trail
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/workflows/instances/{}/audit", instance_id),
        &token,
    )
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.is_array());
    let logs = json.as_array().unwrap();
    assert!(!logs.is_empty());
    assert_eq!(logs[0]["action"], "start");
}

#[actix_web::test]
async fn test_get_pending_approvals() {
    let app_state = create_test_app_state();
    let app = test::init_service(build_test_app(&app_state)).await;

    let (token, _user_id) = register_admin(&app_state, 1).await;
    let template_id = create_workflow_template!(&app, &token);

    // Start workflow
    let start_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/workflows/instances",
        &token,
    )
    .set_json(json!({
        "template_id": template_id,
        "entity_id": 800,
        "entity_type": "purchase_order"
    }))
    .to_request();

    let _ = test::call_service(&app, start_req).await;

    // Get pending approvals
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/workflows/pending",
        &token,
    )
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.is_array());
}

// ============================================================================
// Tenant Isolation Tests
// ============================================================================

#[actix_web::test]
async fn test_tenant_isolation_templates() {
    let app_state = create_test_app_state();
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
    let app_state = create_test_app_state();
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
    let app_state = create_test_app_state();
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
    let app_state = create_test_app_state();
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
    let app_state = create_test_app_state();
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
