//! Notification CRUD Integration Tests

use actix_web::{body::to_bytes, http::StatusCode, test};
use serde_json::json;

mod common;
use common::*;

// ============================================================================
// Send / List Tests
// ============================================================================

#[actix_web::test]
async fn test_send_notification_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/notifications/send",
        &token,
    )
    .set_json(json!({
        "user_id": _user_id,
        "channel": "email",
        "priority": "normal",
        "template_key": "invoice_created",
        "template_vars": {
            "customer_name": "Acme Corp",
            "invoice_number": "INV-001",
            "amount": "1000.00",
            "currency": "TRY",
            "due_date": "2024-02-01"
        },
        "recipient": "test@example.com"
    }))
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["channel"], "Email");
    assert_eq!(json["priority"], "Normal");
    assert_eq!(json["status"], "Queued");
    assert_eq!(json["template_key"], "invoice_created");
    assert_eq!(json["recipient"], "test@example.com");
    assert!(json["id"].is_number());
}

#[actix_web::test]
async fn test_get_notification_history() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    // Send a notification
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/notifications/send",
        &token,
    )
    .set_json(json!({
        "user_id": _user_id,
        "channel": "email",
        "template_key": "invoice_created",
        "template_vars": {"invoice_number": "INV-002"},
        "recipient": "test2@example.com"
    }))
    .to_request();
    test::call_service(&app, req).await;

    // Get history
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/notifications/history?limit=10&offset=0",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json["items"].as_array().unwrap();
    assert!(!items.is_empty());
    assert!(json["total"].is_number());
    assert!(json["page"].is_number());
    assert!(json["per_page"].is_number());
}

#[actix_web::test]
async fn test_get_unread_count() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    // Send an in-app notification
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/notifications/send",
        &token,
    )
    .set_json(json!({
        "user_id": _user_id,
        "channel": "inapp",
        "template_key": "stock_low",
        "template_vars": {"product_name": "Widget"},
        "recipient": "user@example.com"
    }))
    .to_request();
    test::call_service(&app, req).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/notifications/unread-count",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["count"].is_number());
}

#[actix_web::test]
async fn test_mark_notification_read() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    // Send an in-app notification
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/notifications/send",
        &token,
    )
    .set_json(json!({
        "user_id": _user_id,
        "channel": "inapp",
        "template_key": "payment_received",
        "template_vars": {"payment_id": "PAY-001"},
        "recipient": "user@example.com"
    }))
    .to_request();
    let send_resp = test::call_service(&app, req).await;
    let body = to_bytes(send_resp.into_body()).await.unwrap();
    let send_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = send_json["id"].as_i64().unwrap();

    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/notifications/{}/read", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["message"], "Notification marked as read");
}

// ============================================================================
// Soft Delete Tests
// ============================================================================

#[actix_web::test]
async fn test_delete_and_restore_notification() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    // Send a notification
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/notifications/send",
        &token,
    )
    .set_json(json!({
        "channel": "email",
        "template_key": "invoice_created",
        "template_vars": {"invoice_number": "INV-DEL"},
        "recipient": "del@example.com"
    }))
    .to_request();
    let send_resp = test::call_service(&app, req).await;
    let body = to_bytes(send_resp.into_body()).await.unwrap();
    let send_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = send_json["id"].as_i64().unwrap();

    // Soft delete
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/notifications/{}", id),
        &token,
    )
    .to_request();
    let del_resp = test::call_service(&app, del_req).await;
    assert_eq!(del_resp.status(), StatusCode::NO_CONTENT);

    // List deleted
    let list_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/notifications/deleted",
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

    // Restore
    let restore_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/notifications/{}/restore", id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::OK);

    let body = to_bytes(restore_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["message"], "Notification restored");

    // List deleted should be empty now
    let list_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/notifications/deleted",
        &token,
    )
    .to_request();
    let list_resp = test::call_service(&app, list_req).await;
    let body = to_bytes(list_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json.as_array().unwrap();
    assert_eq!(items.len(), 0);
}

#[actix_web::test]
async fn test_destroy_notification_permanently() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    // Send a notification
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/notifications/send",
        &token,
    )
    .set_json(json!({
        "channel": "email",
        "template_key": "invoice_created",
        "template_vars": {"invoice_number": "INV-DEST"},
        "recipient": "dest@example.com"
    }))
    .to_request();
    let send_resp = test::call_service(&app, req).await;
    let body = to_bytes(send_resp.into_body()).await.unwrap();
    let send_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = send_json["id"].as_i64().unwrap();

    // Soft delete first
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/notifications/{}", id),
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    // Permanently destroy
    let destroy_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/notifications/{}/destroy", id),
        &token,
    )
    .to_request();
    let destroy_resp = test::call_service(&app, destroy_req).await;
    assert_eq!(destroy_resp.status(), StatusCode::NO_CONTENT);

    // Should not be restorable
    let restore_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/notifications/{}/restore", id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// Not Found / Unauthorized
// ============================================================================

#[actix_web::test]
async fn test_mark_notification_read_not_found() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::PUT,
        "/api/v1/notifications/99999/read",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_notification_unauthorized() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/notifications/send")
        .set_json(json!({
            "channel": "email",
            "template_key": "test",
            "template_vars": {},
            "recipient": "test@example.com"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_send_notification_rejects_foreign_user() {
    // Cross-tenant IDOR guard: a tenant-1 admin may NOT attribute a
    // notification to a tenant-2 user (`user_id` REFERENCES users(id)). The
    // parent-ownership precheck returns NotFound before the notification row
    // is written — no cross-tenant orphan write.
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;
    let foreign_user_id = register_user(&state, 2).await;

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/notifications/send",
        &token,
    )
    .set_json(json!({
        "user_id": foreign_user_id,
        "channel": "email",
        "template_key": "invoice_created",
        "template_vars": {"invoice_number": "INV-FOREIGN"},
        "recipient": "foreign@example.com"
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_send_notification_accepts_none_user() {
    // `user_id: None` is a legitimate "no linked user / broadcast" — the
    // precheck must skip it (never reject), and the notification is sent.
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/notifications/send",
        &token,
    )
    .set_json(json!({
        "channel": "email",
        "template_key": "invoice_created",
        "template_vars": {"invoice_number": "INV-NONE"},
        "recipient": "none@example.com"
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
}
