//! Settings CRUD Integration Tests

use actix_web::{body::to_bytes, http::StatusCode, test};
use serde_json::json;

mod common;
use common::*;

// ============================================================================
// Settings CRUD Tests
// ============================================================================

#[actix_web::test]
async fn test_create_setting_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(actix_web::http::Method::POST, "/api/settings", &token)
        .set_json(json!({
            "key": "app.name",
            "value": "Turerp",
            "data_type": "string",
            "group": "general",
            "description": "Application name",
            "is_sensitive": false,
            "is_editable": true,
            "tenant_id": 1
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["key"], "app.name");
    assert_eq!(json["value"], "Turerp");
    assert_eq!(json["data_type"], "string");
    assert_eq!(json["group"], "general");
    assert!(json["id"].is_number());
}

#[actix_web::test]
async fn test_list_settings_paginated() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    for i in 1..=3 {
        let req = auth_request(actix_web::http::Method::POST, "/api/settings", &token)
            .set_json(json!({
                "key": format!("setting_{}", i),
                "value": format!("value{}", i),
                "data_type": "string",
                "group": "general",
                "description": format!("Setting {}", i),
                "is_sensitive": false,
                "is_editable": true,
                "tenant_id": 1
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/settings?page=1&per_page=2",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["data"].as_array().unwrap().len(), 2);
    assert_eq!(json["total"], 3);
    assert_eq!(json["page"], 1);
    assert_eq!(json["per_page"], 2);
}

#[actix_web::test]
async fn test_list_settings_by_group() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(actix_web::http::Method::POST, "/api/settings", &token)
        .set_json(json!({
            "key": "invoice.prefix",
            "value": "INV",
            "data_type": "string",
            "group": "invoice",
            "description": "Invoice prefix",
            "is_sensitive": false,
            "is_editable": true,
            "tenant_id": 1
        }))
        .to_request();
    test::call_service(&app, req).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/settings?group=invoice",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json["data"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["key"], "invoice.prefix");
}

#[actix_web::test]
async fn test_get_setting_by_key_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/settings", &token)
        .set_json(json!({
            "key": "company.name",
            "value": "Acme Corp",
            "data_type": "string",
            "group": "company",
            "description": "Company name",
            "is_sensitive": false,
            "is_editable": true,
            "tenant_id": 1
        }))
        .to_request();
    test::call_service(&app, create_req).await;

    let get_req = auth_request(
        actix_web::http::Method::GET,
        "/api/settings/company.name",
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);

    let body = to_bytes(get_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["key"], "company.name");
    assert_eq!(json["value"], "Acme Corp");
}

#[actix_web::test]
async fn test_get_setting_not_found() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/settings/nonexistent.key",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_update_setting_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/settings", &token)
        .set_json(json!({
            "key": "update.test",
            "value": "original",
            "data_type": "string",
            "group": "general",
            "description": "Original desc",
            "is_sensitive": false,
            "is_editable": true,
            "tenant_id": 1
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let update_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/settings/{}", id),
        &token,
    )
    .set_json(json!({
        "value": "updated",
        "description": "Updated desc"
    }))
    .to_request();
    let update_resp = test::call_service(&app, update_req).await;
    assert_eq!(update_resp.status(), StatusCode::OK);

    let body = to_bytes(update_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["value"], "updated");
    assert_eq!(json["description"], "Updated desc");
    assert_eq!(json["key"], "update.test");
}

// ============================================================================
// Soft Delete Tests
// ============================================================================

#[actix_web::test]
async fn test_soft_delete_and_restore_setting() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/settings", &token)
        .set_json(json!({
            "key": "delete.test",
            "value": "delete me",
            "data_type": "string",
            "group": "general",
            "description": "Delete test",
            "is_sensitive": false,
            "is_editable": true,
            "tenant_id": 1
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/settings/{}/soft", id),
        &token,
    )
    .to_request();
    let del_resp = test::call_service(&app, del_req).await;
    assert_eq!(del_resp.status(), StatusCode::NO_CONTENT);

    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/settings/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);

    let restore_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/settings/{}/restore", id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::NO_CONTENT);

    let get_req = auth_request(
        actix_web::http::Method::GET,
        "/api/settings/delete.test",
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn test_list_deleted_settings() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/settings", &token)
        .set_json(json!({
            "key": "list.deleted",
            "value": "value",
            "data_type": "string",
            "group": "general",
            "description": "List deleted test",
            "is_sensitive": false,
            "is_editable": true,
            "tenant_id": 1
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/settings/{}/soft", id),
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    let list_req = auth_request(
        actix_web::http::Method::GET,
        "/api/settings/deleted",
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
async fn test_destroy_setting_permanently() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/settings", &token)
        .set_json(json!({
            "key": "destroy.test",
            "value": "destroy me",
            "data_type": "string",
            "group": "general",
            "description": "Destroy test",
            "is_sensitive": false,
            "is_editable": true,
            "tenant_id": 1
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/settings/{}/soft", id),
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    let destroy_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/settings/{}/destroy", id),
        &token,
    )
    .to_request();
    let destroy_resp = test::call_service(&app, destroy_req).await;
    assert_eq!(destroy_resp.status(), StatusCode::NO_CONTENT);

    let restore_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/settings/{}/restore", id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// Bulk Update / Seed Tests
// ============================================================================

#[actix_web::test]
async fn test_bulk_update_settings() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    for key in ["bulk.a", "bulk.b"] {
        let req = auth_request(actix_web::http::Method::POST, "/api/settings", &token)
            .set_json(json!({
                "key": key,
                "value": "original",
                "data_type": "string",
                "group": "general",
                "description": "Bulk test",
                "is_sensitive": false,
                "is_editable": true,
                "tenant_id": 1
            }))
            .to_request();
        test::call_service(&app, req).await;
    }

    let req = auth_request(actix_web::http::Method::POST, "/api/settings/bulk", &token)
        .set_json(json!({
            "updates": [
                { "key": "bulk.a", "value": "updated_a" },
                { "key": "bulk.b", "value": "updated_b" }
            ]
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["updated"], 2);
}

#[actix_web::test]
async fn test_seed_settings() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req =
        auth_request(actix_web::http::Method::POST, "/api/settings/seed", &token).to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["created"].is_number());
}

// ============================================================================
// Unauthorized / Not Found
// ============================================================================

#[actix_web::test]
async fn test_settings_unauthorized_without_token() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;

    let req = test::TestRequest::post()
        .uri("/api/settings")
        .set_json(json!({
            "key": "no.auth",
            "value": "test",
            "data_type": "string",
            "group": "general",
            "description": "No auth",
            "tenant_id": 1
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_setting_not_found() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/settings/nonexistent_key",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
