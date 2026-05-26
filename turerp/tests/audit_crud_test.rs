//! Audit Log Integration Tests

use actix_web::{body::to_bytes, http::StatusCode, test};
use chrono::Utc;

mod common;
use common::*;

// ============================================================================
// Audit Log Tests
// ============================================================================

#[actix_web::test]
async fn test_get_audit_logs_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, user_id) = register_admin(&state, 1).await;

    let audit_service = state.analytics.audit_service.get_ref();
    for i in 1..=3 {
        audit_service
            .create_log(turerp::domain::audit::model::CreateAuditLog {
                tenant_id: 1,
                user_id,
                username: "admin".to_string(),
                action: "GET".to_string(),
                path: format!("/api/v1/test/{}", i),
                status_code: 200,
                request_id: format!("req-{}", i),
                ip_address: Some("127.0.0.1".to_string()),
                user_agent: Some("test-agent".to_string()),
                created_at: Utc::now(),
            })
            .await
            .unwrap();
    }

    let req = auth_request(actix_web::http::Method::GET, "/api/v1/audit-logs", &token).to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["items"].as_array().unwrap().len(), 3);
    assert_eq!(json["total"], 3);
    assert_eq!(json["page"], 1);
}

#[actix_web::test]
async fn test_get_audit_logs_paginated() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, user_id) = register_admin(&state, 1).await;

    let audit_service = state.analytics.audit_service.get_ref();
    for i in 1..=5 {
        audit_service
            .create_log(turerp::domain::audit::model::CreateAuditLog {
                tenant_id: 1,
                user_id,
                username: "admin".to_string(),
                action: "GET".to_string(),
                path: format!("/api/v1/test/{}", i),
                status_code: 200,
                request_id: format!("req-{}", i),
                ip_address: Some("127.0.0.1".to_string()),
                user_agent: Some("test-agent".to_string()),
                created_at: Utc::now(),
            })
            .await
            .unwrap();
    }

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/audit-logs?page=1&per_page=2",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["items"].as_array().unwrap().len(), 2);
    assert_eq!(json["total"], 5);
    assert_eq!(json["per_page"], 2);
}

#[actix_web::test]
async fn test_get_audit_logs_filtered_by_path() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, user_id) = register_admin(&state, 1).await;

    let audit_service = state.analytics.audit_service.get_ref();
    audit_service
        .create_log(turerp::domain::audit::model::CreateAuditLog {
            tenant_id: 1,
            user_id,
            username: "admin".to_string(),
            action: "GET".to_string(),
            path: "/api/v1/users".to_string(),
            status_code: 200,
            request_id: "req-1".to_string(),
            ip_address: Some("127.0.0.1".to_string()),
            user_agent: Some("test-agent".to_string()),
            created_at: Utc::now(),
        })
        .await
        .unwrap();
    audit_service
        .create_log(turerp::domain::audit::model::CreateAuditLog {
            tenant_id: 1,
            user_id,
            username: "admin".to_string(),
            action: "POST".to_string(),
            path: "/api/v1/products".to_string(),
            status_code: 201,
            request_id: "req-2".to_string(),
            ip_address: Some("127.0.0.1".to_string()),
            user_agent: Some("test-agent".to_string()),
            created_at: Utc::now(),
        })
        .await
        .unwrap();

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/audit-logs?path=users",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["path"], "/api/v1/users");
}

#[actix_web::test]
async fn test_get_audit_logs_filtered_by_user_id() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, user_id) = register_admin(&state, 1).await;

    let audit_service = state.analytics.audit_service.get_ref();
    audit_service
        .create_log(turerp::domain::audit::model::CreateAuditLog {
            tenant_id: 1,
            user_id,
            username: "admin".to_string(),
            action: "GET".to_string(),
            path: "/api/v1/test".to_string(),
            status_code: 200,
            request_id: "req-1".to_string(),
            ip_address: Some("127.0.0.1".to_string()),
            user_agent: Some("test-agent".to_string()),
            created_at: Utc::now(),
        })
        .await
        .unwrap();
    audit_service
        .create_log(turerp::domain::audit::model::CreateAuditLog {
            tenant_id: 1,
            user_id: user_id + 1,
            username: "other".to_string(),
            action: "GET".to_string(),
            path: "/api/v1/test".to_string(),
            status_code: 200,
            request_id: "req-2".to_string(),
            ip_address: Some("127.0.0.1".to_string()),
            user_agent: Some("test-agent".to_string()),
            created_at: Utc::now(),
        })
        .await
        .unwrap();

    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/audit-logs?user_id={}", user_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["user_id"], user_id);
}

#[actix_web::test]
async fn test_get_audit_logs_tenant_isolation() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, user_id) = register_admin(&state, 1).await;

    let audit_service = state.analytics.audit_service.get_ref();
    audit_service
        .create_log(turerp::domain::audit::model::CreateAuditLog {
            tenant_id: 1,
            user_id,
            username: "admin".to_string(),
            action: "GET".to_string(),
            path: "/api/v1/test".to_string(),
            status_code: 200,
            request_id: "req-1".to_string(),
            ip_address: Some("127.0.0.1".to_string()),
            user_agent: Some("test-agent".to_string()),
            created_at: Utc::now(),
        })
        .await
        .unwrap();
    audit_service
        .create_log(turerp::domain::audit::model::CreateAuditLog {
            tenant_id: 2,
            user_id,
            username: "admin".to_string(),
            action: "GET".to_string(),
            path: "/api/v1/test".to_string(),
            status_code: 200,
            request_id: "req-2".to_string(),
            ip_address: Some("127.0.0.1".to_string()),
            user_agent: Some("test-agent".to_string()),
            created_at: Utc::now(),
        })
        .await
        .unwrap();

    let req = auth_request(actix_web::http::Method::GET, "/api/v1/audit-logs", &token).to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["tenant_id"], 1);
}

// ============================================================================
// Authorization Tests
// ============================================================================

#[actix_web::test]
async fn test_get_audit_logs_unauthorized() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/audit-logs")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
