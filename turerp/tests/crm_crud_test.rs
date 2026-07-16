//! CRM (Leads) CRUD Integration Tests

use actix_web::{body::to_bytes, http::StatusCode, test};
use serde_json::json;

use crate::common::*;

// ============================================================================
// CRUD Tests
// ============================================================================

#[actix_web::test]
async fn test_create_lead_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(actix_web::http::Method::POST, "/api/v1/crm/leads", &token)
        .set_json(json!({
            "tenant_id": 1,
            "name": "John Doe",
            "company": "Acme Inc",
            "email": "john@acme.com",
            "phone": "+1234567890",
            "source": "Website",
            "assigned_to": null,
            "notes": null
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["name"], "John Doe");
    assert_eq!(json["status"], "New");
    assert!(json["id"].is_number());
}

#[actix_web::test]
async fn test_list_leads_paginated() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    for i in 1..=3 {
        let req = auth_request(actix_web::http::Method::POST, "/api/v1/crm/leads", &token)
            .set_json(json!({
                "tenant_id": 1,
                "name": format!("Lead {}", i),
                "source": "Website"
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/crm/leads?page=1&per_page=2",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["items"].as_array().unwrap().len(), 2);
    assert_eq!(json["total"], 3);
    assert_eq!(json["page"], 1);
    assert_eq!(json["per_page"], 2);
}

#[actix_web::test]
async fn test_get_lead_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/crm/leads", &token)
        .set_json(json!({
            "tenant_id": 1,
            "name": "Get Lead Test",
            "source": "Referral"
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/crm/leads/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);

    let body = to_bytes(get_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);
    assert_eq!(json["name"], "Get Lead Test");
}

#[actix_web::test]
async fn test_get_lead_not_found() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/crm/leads/99999",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_update_lead_status_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/crm/leads", &token)
        .set_json(json!({
            "tenant_id": 1,
            "name": "Status Update Lead",
            "source": "Website"
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let update_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/crm/leads/{}/status", id),
        &token,
    )
    .set_json(json!({
        "status": "Contacted"
    }))
    .to_request();
    let update_resp = test::call_service(&app, update_req).await;
    assert_eq!(update_resp.status(), StatusCode::OK);

    let body = to_bytes(update_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "Contacted");
    assert_eq!(json["name"], "Status Update Lead");
}

#[actix_web::test]
async fn test_unauthorized_crm_access() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/crm/leads")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ============================================================================
// Soft Delete Tests
// ============================================================================

#[actix_web::test]
async fn test_delete_and_restore_lead() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/crm/leads", &token)
        .set_json(json!({
            "tenant_id": 1,
            "name": "Delete Lead Test",
            "source": "Email"
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Soft delete
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/crm/leads/{}", id),
        &token,
    )
    .to_request();
    let del_resp = test::call_service(&app, del_req).await;
    assert_eq!(del_resp.status(), StatusCode::OK);

    // Verify deleted - should return 404
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/crm/leads/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);

    // Restore
    let restore_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/crm/leads/{}/restore", id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::OK);

    let body = to_bytes(restore_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);
    assert_eq!(json["name"], "Delete Lead Test");

    // Verify restored
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/crm/leads/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn test_list_deleted_leads() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/crm/leads", &token)
        .set_json(json!({
            "tenant_id": 1,
            "name": "List Deleted Lead",
            "source": "Website"
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Delete
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/crm/leads/{}", id),
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    // List deleted
    let list_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/crm/leads/deleted",
        &token,
    )
    .to_request();
    let list_resp = test::call_service(&app, list_req).await;
    assert_eq!(list_resp.status(), StatusCode::OK);

    let body = to_bytes(list_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json.as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["id"], id);
}

#[actix_web::test]
async fn test_destroy_lead_permanently() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/crm/leads", &token)
        .set_json(json!({
            "tenant_id": 1,
            "name": "Destroy Lead Test",
            "source": "Website"
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Soft delete first
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/crm/leads/{}", id),
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    // Permanently destroy
    let destroy_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/crm/leads/{}/destroy", id),
        &token,
    )
    .to_request();
    let destroy_resp = test::call_service(&app, destroy_req).await;
    assert_eq!(destroy_resp.status(), StatusCode::NO_CONTENT);

    // Should not be restorable
    let restore_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/crm/leads/{}/restore", id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::NOT_FOUND);
}
