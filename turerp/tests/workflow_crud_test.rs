//! Workflow CRUD Integration Tests

use actix_web::{body::to_bytes, http::StatusCode, test};
use serde_json::json;

mod common;
use common::*;

// ============================================================================
// CRUD Tests
// ============================================================================

#[actix_web::test]
async fn test_create_template_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/workflows/templates",
        &token,
    )
    .set_json(json!({
        "name": "PO Approval",
        "description": "Test workflow",
        "entity_type": "purchase_order",
        "config_json": {
            "steps": [
                {"step_number": 1, "step_name": "Manager Review", "approver_role": "manager"},
                {"step_number": 2, "step_name": "Admin Approval", "approver_role": "admin"}
            ]
        }
    }))
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["name"], "PO Approval");
    assert_eq!(json["entity_type"], "purchase_order");
    assert!(json["id"].is_number());
}

#[actix_web::test]
async fn test_list_templates() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    for i in 1..=3 {
        let req = auth_request(
            actix_web::http::Method::POST,
            "/api/v1/workflows/templates",
            &token,
        )
        .set_json(json!({
            "name": format!("Template {}", i),
            "description": "Test",
            "entity_type": "purchase_order",
            "config_json": {"steps": [{"step_number": 1, "step_name": "Review"}]}
        }))
        .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

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
    let items = json.as_array().unwrap();
    assert_eq!(items.len(), 3);
}

#[actix_web::test]
async fn test_start_workflow_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let template_id = create_workflow_template!(&app, &token);

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/workflows/instances",
        &token,
    )
    .set_json(json!({
        "template_id": template_id,
        "entity_id": 1,
        "entity_type": "purchase_order"
    }))
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["template_id"], template_id);
    assert_eq!(json["entity_id"], 1);
    assert_eq!(json["status"], "pending");
    assert!(json["id"].is_number());
}

#[actix_web::test]
async fn test_get_instance_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let template_id = create_workflow_template!(&app, &token);

    let start_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/workflows/instances",
        &token,
    )
    .set_json(json!({
        "template_id": template_id,
        "entity_id": 1,
        "entity_type": "purchase_order"
    }))
    .to_request();
    let start_resp = test::call_service(&app, start_req).await;
    assert_eq!(start_resp.status(), StatusCode::CREATED);
    let body = to_bytes(start_resp.into_body()).await.unwrap();
    let start_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let instance_id = start_json["id"].as_i64().unwrap();

    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/workflows/instances/{}", instance_id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);

    let body = to_bytes(get_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], instance_id);
    assert_eq!(json["status"], "pending");
    let steps = json["steps"].as_array().unwrap();
    assert!(!steps.is_empty());
    assert_eq!(steps[0]["status"], "pending");
}

#[actix_web::test]
async fn test_get_instance_not_found() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

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
async fn test_approve_step_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let template_id = create_workflow_template!(&app, &token);

    let start_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/workflows/instances",
        &token,
    )
    .set_json(json!({
        "template_id": template_id,
        "entity_id": 1,
        "entity_type": "purchase_order"
    }))
    .to_request();
    let start_resp = test::call_service(&app, start_req).await;
    assert_eq!(start_resp.status(), StatusCode::CREATED);
    let body = to_bytes(start_resp.into_body()).await.unwrap();
    let start_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let instance_id = start_json["id"].as_i64().unwrap();

    let approve_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/workflows/instances/{}/approve", instance_id),
        &token,
    )
    .set_json(json!({ "comment": "Looks good" }))
    .to_request();
    let approve_resp = test::call_service(&app, approve_req).await;
    assert_eq!(approve_resp.status(), StatusCode::OK);

    let body = to_bytes(approve_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], instance_id);
}

#[actix_web::test]
async fn test_reject_step_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let template_id = create_workflow_template!(&app, &token);

    let start_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/workflows/instances",
        &token,
    )
    .set_json(json!({
        "template_id": template_id,
        "entity_id": 1,
        "entity_type": "purchase_order"
    }))
    .to_request();
    let start_resp = test::call_service(&app, start_req).await;
    assert_eq!(start_resp.status(), StatusCode::CREATED);
    let body = to_bytes(start_resp.into_body()).await.unwrap();
    let start_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let instance_id = start_json["id"].as_i64().unwrap();

    let reject_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/workflows/instances/{}/reject", instance_id),
        &token,
    )
    .set_json(json!({ "comment": "Needs revision" }))
    .to_request();
    let reject_resp = test::call_service(&app, reject_req).await;
    assert_eq!(reject_resp.status(), StatusCode::OK);

    let body = to_bytes(reject_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], instance_id);
    assert_eq!(json["status"], "rejected");
}

#[actix_web::test]
async fn test_resubmit_workflow_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let template_id = create_workflow_template!(&app, &token);

    let start_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/workflows/instances",
        &token,
    )
    .set_json(json!({
        "template_id": template_id,
        "entity_id": 1,
        "entity_type": "purchase_order"
    }))
    .to_request();
    let start_resp = test::call_service(&app, start_req).await;
    assert_eq!(start_resp.status(), StatusCode::CREATED);
    let body = to_bytes(start_resp.into_body()).await.unwrap();
    let start_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let instance_id = start_json["id"].as_i64().unwrap();

    // Reject first
    let reject_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/workflows/instances/{}/reject", instance_id),
        &token,
    )
    .set_json(json!({ "comment": "No good" }))
    .to_request();
    test::call_service(&app, reject_req).await;

    // Resubmit
    let resubmit_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/workflows/instances/{}/resubmit", instance_id),
        &token,
    )
    .to_request();
    let resubmit_resp = test::call_service(&app, resubmit_req).await;
    assert_eq!(resubmit_resp.status(), StatusCode::OK);

    let body = to_bytes(resubmit_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], instance_id);
    assert_eq!(json["status"], "pending");
}

#[actix_web::test]
async fn test_get_instance_audit_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let template_id = create_workflow_template!(&app, &token);

    let start_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/workflows/instances",
        &token,
    )
    .set_json(json!({
        "template_id": template_id,
        "entity_id": 1,
        "entity_type": "purchase_order"
    }))
    .to_request();
    let start_resp = test::call_service(&app, start_req).await;
    assert_eq!(start_resp.status(), StatusCode::CREATED);
    let body = to_bytes(start_resp.into_body()).await.unwrap();
    let start_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let instance_id = start_json["id"].as_i64().unwrap();

    let audit_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/workflows/instances/{}/audit", instance_id),
        &token,
    )
    .to_request();
    let audit_resp = test::call_service(&app, audit_req).await;
    assert_eq!(audit_resp.status(), StatusCode::OK);

    let body = to_bytes(audit_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json.as_array().unwrap();
    assert!(!items.is_empty());
    assert_eq!(items[0]["action"], "start");
}

#[actix_web::test]
async fn test_workflow_unauthorized() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/workflows/templates")
        .set_json(json!({
            "name": "Test",
            "description": "Test",
            "entity_type": "purchase_order",
            "config_json": {"steps": []}
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
