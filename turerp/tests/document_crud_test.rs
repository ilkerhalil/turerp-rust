//! Document CRUD Integration Tests

use actix_web::{body::to_bytes, http::StatusCode, test};
use serde_json::json;

use crate::common::*;

// ============================================================================
// CRUD Tests
// ============================================================================

#[actix_web::test]
async fn test_create_document_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(actix_web::http::Method::POST, "/api/v1/documents", &token)
        .set_json(json!({
            "tenant_id": 1,
            "name": "Test Document",
            "filename": "test.pdf",
            "size_bytes": 1024,
            "mime_type": "application/pdf",
            "hash": "abc123",
            "storage_path": "/docs/test.pdf",
            "uploaded_by": null,
            "category_id": null,
            "tags": ["legal", "2024"],
            "description": "A test document"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["name"], "Test Document");
    assert_eq!(json["filename"], "test.pdf");
    assert_eq!(json["mime_type"], "application/pdf");
    assert!(json["id"].is_number());
}

#[actix_web::test]
async fn test_list_documents_paginated() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    for i in 1..=3 {
        let req = auth_request(actix_web::http::Method::POST, "/api/v1/documents", &token)
            .set_json(json!({
                "tenant_id": 1,
                "name": format!("Document {}", i),
                "filename": format!("doc{}.pdf", i),
                "size_bytes": 512,
                "mime_type": "application/pdf",
                "hash": format!("hash{}", i),
                "storage_path": format!("/docs/doc{}.pdf", i)
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/documents?page=1&per_page=2",
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
async fn test_get_document_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/documents", &token)
        .set_json(json!({
            "tenant_id": 1,
            "name": "Get Document Test",
            "filename": "get.pdf",
            "size_bytes": 2048,
            "mime_type": "application/pdf",
            "hash": "gethash",
            "storage_path": "/docs/get.pdf"
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/documents/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);

    let body = to_bytes(get_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);
    assert_eq!(json["name"], "Get Document Test");
}

#[actix_web::test]
async fn test_get_document_not_found() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/documents/99999",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_update_document_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/documents", &token)
        .set_json(json!({
            "tenant_id": 1,
            "name": "Original Document",
            "filename": "orig.pdf",
            "size_bytes": 1024,
            "mime_type": "application/pdf",
            "hash": "orighash",
            "storage_path": "/docs/orig.pdf"
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let update_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/documents/{}", id),
        &token,
    )
    .set_json(json!({
        "name": "Updated Document",
        "description": "Updated description",
        "tags": ["updated", "tag"]
    }))
    .to_request();
    let update_resp = test::call_service(&app, update_req).await;
    assert_eq!(update_resp.status(), StatusCode::OK);

    let body = to_bytes(update_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["name"], "Updated Document");
    assert_eq!(json["description"], "Updated description");
}

#[actix_web::test]
async fn test_search_documents_by_query() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(actix_web::http::Method::POST, "/api/v1/documents", &token)
        .set_json(json!({
            "tenant_id": 1,
            "name": "Contract Alpha",
            "filename": "contract.pdf",
            "size_bytes": 1024,
            "mime_type": "application/pdf",
            "hash": "hash1",
            "storage_path": "/docs/contract.pdf"
        }))
        .to_request();
    test::call_service(&app, req).await;

    let req = auth_request(actix_web::http::Method::POST, "/api/v1/documents", &token)
        .set_json(json!({
            "tenant_id": 1,
            "name": "Report Beta",
            "filename": "report.pdf",
            "size_bytes": 2048,
            "mime_type": "application/pdf",
            "hash": "hash2",
            "storage_path": "/docs/report.pdf"
        }))
        .to_request();
    test::call_service(&app, req).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/documents?query=Contract",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json["items"].as_array().unwrap();
    assert!(!items.is_empty());
    assert_eq!(items[0]["name"], "Contract Alpha");
}

#[actix_web::test]
async fn test_unauthorized_document_access() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/documents")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ============================================================================
// Soft Delete Tests
// ============================================================================

#[actix_web::test]
async fn test_delete_and_restore_document() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/documents", &token)
        .set_json(json!({
            "tenant_id": 1,
            "name": "Delete Document Test",
            "filename": "del.pdf",
            "size_bytes": 512,
            "mime_type": "application/pdf",
            "hash": "delhash",
            "storage_path": "/docs/del.pdf"
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Soft delete
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/documents/{}", id),
        &token,
    )
    .to_request();
    let del_resp = test::call_service(&app, del_req).await;
    assert_eq!(del_resp.status(), StatusCode::NO_CONTENT);

    // Verify deleted - should return 404
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/documents/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);

    // Restore
    let restore_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/documents/{}/restore", id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::OK);

    let body = to_bytes(restore_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);
    assert_eq!(json["name"], "Delete Document Test");

    // Verify restored
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/documents/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn test_list_deleted_documents() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/documents", &token)
        .set_json(json!({
            "tenant_id": 1,
            "name": "List Deleted Document",
            "filename": "listdel.pdf",
            "size_bytes": 256,
            "mime_type": "application/pdf",
            "hash": "listdelhash",
            "storage_path": "/docs/listdel.pdf"
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Delete
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/documents/{}", id),
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    // List deleted
    let list_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/documents/deleted",
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
async fn test_destroy_document_permanently() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/documents", &token)
        .set_json(json!({
            "tenant_id": 1,
            "name": "Destroy Document Test",
            "filename": "destroy.pdf",
            "size_bytes": 128,
            "mime_type": "application/pdf",
            "hash": "destroyhash",
            "storage_path": "/docs/destroy.pdf"
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Soft delete first
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/documents/{}", id),
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    // Permanently destroy (requires admin)
    let destroy_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/documents/{}/destroy", id),
        &token,
    )
    .to_request();
    let destroy_resp = test::call_service(&app, destroy_req).await;
    assert_eq!(destroy_resp.status(), StatusCode::NO_CONTENT);

    // Should not be restorable
    let restore_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/documents/{}/restore", id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::NOT_FOUND);
}
