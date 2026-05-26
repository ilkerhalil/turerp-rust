//! Workflow Instance Integration Tests

use actix_web::{body::to_bytes, http::StatusCode, test};
use serde_json::json;

mod common;
use common::*;

// Workflow Instance Tests
// ============================================================================

#[actix_web::test]
async fn test_start_workflow() {
    let app_state = create_test_app_state().await;
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
    let app_state = create_test_app_state().await;
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
    let app_state = create_test_app_state().await;
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
    let app_state = create_test_app_state().await;
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
    let app_state = create_test_app_state().await;
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
    let app_state = create_test_app_state().await;
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
    let app_state = create_test_app_state().await;
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
    let app_state = create_test_app_state().await;
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
    let app_state = create_test_app_state().await;
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
    let app_state = create_test_app_state().await;
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
    let app_state = create_test_app_state().await;
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
    let app_state = create_test_app_state().await;
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
