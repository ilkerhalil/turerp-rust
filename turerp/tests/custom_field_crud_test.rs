//! Custom Field CRUD Integration Tests

use actix_web::{body::to_bytes, http::StatusCode, test};
use serde_json::json;

use crate::common::*;

// ============================================================================
// CRUD Tests
// ============================================================================

#[actix_web::test]
async fn test_create_custom_field_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/custom-fields",
        &token,
    )
    .set_json(json!({
        "module": "cari",
        "field_name": "tax_region",
        "field_label": "Tax Region",
        "field_type": "string",
        "required": false,
        "options": [],
        "sort_order": 0,
        "tenant_id": 1
    }))
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["field_name"], "tax_region");
    assert_eq!(json["field_label"], "Tax Region");
    assert_eq!(json["field_type"], "string");
    assert_eq!(json["module"], "cari");
    assert!(json["id"].is_number());
}

#[actix_web::test]
async fn test_create_select_custom_field() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/custom-fields",
        &token,
    )
    .set_json(json!({
        "module": "cari",
        "field_name": "industry",
        "field_label": "Industry",
        "field_type": "select",
        "required": true,
        "options": ["Tech", "Finance", "Healthcare"],
        "sort_order": 1,
        "tenant_id": 1
    }))
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["field_type"], "select");
    assert_eq!(json["options"].as_array().unwrap().len(), 3);
}

#[actix_web::test]
async fn test_list_custom_fields() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    // Create 3 custom fields across 2 modules
    for (module, name) in [("cari", "f1"), ("invoice", "f2"), ("cari", "f3")] {
        let req = auth_request(
            actix_web::http::Method::POST,
            "/api/v1/custom-fields",
            &token,
        )
        .set_json(json!({
            "module": module,
            "field_name": name,
            "field_label": name,
            "field_type": "string",
            "required": false,
            "options": [],
            "sort_order": 0,
            "tenant_id": 1
        }))
        .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    // List all
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/custom-fields",
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
async fn test_list_custom_fields_by_module() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    // Create cari field
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/custom-fields",
        &token,
    )
    .set_json(json!({
        "module": "cari",
        "field_name": "region",
        "field_label": "Region",
        "field_type": "string",
        "required": false,
        "options": [],
        "sort_order": 0,
        "tenant_id": 1
    }))
    .to_request();
    test::call_service(&app, req).await;

    // Create invoice field
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/custom-fields",
        &token,
    )
    .set_json(json!({
        "module": "invoice",
        "field_name": "priority",
        "field_label": "Priority",
        "field_type": "string",
        "required": false,
        "options": [],
        "sort_order": 0,
        "tenant_id": 1
    }))
    .to_request();
    test::call_service(&app, req).await;

    // Filter by module
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/custom-fields?module=cari",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json.as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["module"], "cari");
}

#[actix_web::test]
async fn test_get_custom_field_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/custom-fields",
        &token,
    )
    .set_json(json!({
        "module": "cari",
        "field_name": "get_test",
        "field_label": "Get Test",
        "field_type": "string",
        "required": false,
        "options": [],
        "sort_order": 0,
        "tenant_id": 1
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/custom-fields/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);

    let body = to_bytes(get_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);
    assert_eq!(json["field_name"], "get_test");
}

#[actix_web::test]
async fn test_get_custom_field_not_found() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/custom-fields/99999",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_update_custom_field_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/custom-fields",
        &token,
    )
    .set_json(json!({
        "module": "cari",
        "field_name": "upd_test",
        "field_label": "Original Label",
        "field_type": "string",
        "required": false,
        "options": [],
        "sort_order": 0,
        "tenant_id": 1
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let update_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/custom-fields/{}", id),
        &token,
    )
    .set_json(json!({
        "field_label": "Updated Label",
        "required": true,
        "sort_order": 5,
        "is_active": false
    }))
    .to_request();
    let update_resp = test::call_service(&app, update_req).await;
    assert_eq!(update_resp.status(), StatusCode::OK);

    let body = to_bytes(update_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["field_label"], "Updated Label");
    assert_eq!(json["required"], true);
    assert_eq!(json["sort_order"], 5);
    assert_eq!(json["is_active"], false);
}

// ============================================================================
// Soft Delete Tests
// ============================================================================

#[actix_web::test]
async fn test_delete_and_restore_custom_field() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/custom-fields",
        &token,
    )
    .set_json(json!({
        "module": "cari",
        "field_name": "del_test",
        "field_label": "Delete Test",
        "field_type": "string",
        "required": false,
        "options": [],
        "sort_order": 0,
        "tenant_id": 1
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Soft delete
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/custom-fields/{}", id),
        &token,
    )
    .to_request();
    let del_resp = test::call_service(&app, del_req).await;
    assert_eq!(del_resp.status(), StatusCode::OK);

    // Verify deleted - should return 404
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/custom-fields/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);

    // Restore
    let restore_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/custom-fields/{}/restore", id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::OK);

    // Verify restored
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/custom-fields/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn test_list_deleted_custom_fields() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/custom-fields",
        &token,
    )
    .set_json(json!({
        "module": "cari",
        "field_name": "lst_del",
        "field_label": "List Deleted Test",
        "field_type": "string",
        "required": false,
        "options": [],
        "sort_order": 0,
        "tenant_id": 1
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Delete
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/custom-fields/{}", id),
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    // List deleted
    let list_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/custom-fields/deleted",
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
async fn test_destroy_custom_field_permanently() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/custom-fields",
        &token,
    )
    .set_json(json!({
        "module": "cari",
        "field_name": "dest_test",
        "field_label": "Destroy Test",
        "field_type": "string",
        "required": false,
        "options": [],
        "sort_order": 0,
        "tenant_id": 1
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Soft delete first
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/custom-fields/{}", id),
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    // Permanently destroy
    let destroy_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/custom-fields/{}/destroy", id),
        &token,
    )
    .to_request();
    let destroy_resp = test::call_service(&app, destroy_req).await;
    assert_eq!(destroy_resp.status(), StatusCode::NO_CONTENT);

    // Should not be restorable
    let restore_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/custom-fields/{}/restore", id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// Authorization Tests
// ============================================================================

#[actix_web::test]
async fn test_custom_field_unauthorized() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/custom-fields")
        .set_json(json!({
            "module": "cari",
            "field_name": "unauth",
            "field_label": "Unauthorized",
            "field_type": "string",
            "required": false,
            "options": [],
            "sort_order": 0,
            "tenant_id": 1
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
