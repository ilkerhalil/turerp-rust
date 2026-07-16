//! HR CRUD Integration Tests

use actix_web::{body::to_bytes, http::StatusCode, test};
use serde_json::json;

use crate::common::*;

// ============================================================================
// Employee CRUD Tests
// ============================================================================

#[actix_web::test]
async fn test_create_employee_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let hire_date = chrono::Utc::now();
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/hr/employees",
        &token,
    )
    .set_json(json!({
        "employee_number": "EMP-001",
        "first_name": "John",
        "last_name": "Doe",
        "email": "john.doe@example.com",
        "phone": "+905551234567",
        "department": "IT",
        "position": "Developer",
        "hire_date": hire_date.to_rfc3339(),
        "salary": "5000.00",
        "tc_kimlik_no": "12345678901",
        "children_count": 0,
        "tenant_id": 1
    }))
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["employee_number"], "EMP-001");
    assert_eq!(json["first_name"], "John");
    assert_eq!(json["last_name"], "Doe");
    assert_eq!(json["email"], "john.doe@example.com");
    assert_eq!(json["status"], "Active");
    assert!(json["id"].is_number());
}

#[actix_web::test]
async fn test_list_employees_paginated() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let hire_date = chrono::Utc::now();
    for i in 1..=3 {
        let req = auth_request(
            actix_web::http::Method::POST,
            "/api/v1/hr/employees",
            &token,
        )
        .set_json(json!({
            "employee_number": format!("EMP-00{}", i),
            "first_name": format!("John{}", i),
            "last_name": "Doe",
            "email": format!("john{}@example.com", i),
            "hire_date": hire_date.to_rfc3339(),
            "salary": "5000.00",
            "tc_kimlik_no": format!("1234567890{}", i),
            "children_count": 0,
            "tenant_id": 1
        }))
        .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/hr/employees?page=1&per_page=2",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["items"].as_array().unwrap().len(), 2);
    assert_eq!(json["page"], 1);
    assert_eq!(json["per_page"], 2);
}

#[actix_web::test]
async fn test_get_employee_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let hire_date = chrono::Utc::now();
    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/hr/employees",
        &token,
    )
    .set_json(json!({
        "employee_number": "EMP-GET",
        "first_name": "Jane",
        "last_name": "Smith",
        "email": "jane.smith@example.com",
        "hire_date": hire_date.to_rfc3339(),
        "salary": "6000.00",
        "tc_kimlik_no": "98765432101",
        "children_count": 2,
        "tenant_id": 1
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/hr/employees/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);

    let body = to_bytes(get_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);
    assert_eq!(json["employee_number"], "EMP-GET");
    assert_eq!(json["first_name"], "Jane");
}

#[actix_web::test]
async fn test_get_employee_not_found() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/hr/employees/99999",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_update_employee_status() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let hire_date = chrono::Utc::now();
    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/hr/employees",
        &token,
    )
    .set_json(json!({
        "employee_number": "EMP-UPD",
        "first_name": "Bob",
        "last_name": "Jones",
        "email": "bob.jones@example.com",
        "hire_date": hire_date.to_rfc3339(),
        "salary": "4500.00",
        "tc_kimlik_no": "11223344556",
        "children_count": 1,
        "tenant_id": 1
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let update_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/hr/employees/{}/status", id),
        &token,
    )
    .set_json(json!({ "status": "OnLeave" }))
    .to_request();
    let update_resp = test::call_service(&app, update_req).await;
    assert_eq!(update_resp.status(), StatusCode::OK);

    let body = to_bytes(update_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "OnLeave");
}

// ============================================================================
// Soft Delete Tests
// ============================================================================

#[actix_web::test]
async fn test_soft_delete_and_restore_employee() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let hire_date = chrono::Utc::now();
    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/hr/employees",
        &token,
    )
    .set_json(json!({
        "employee_number": "EMP-DEL",
        "first_name": "Alice",
        "last_name": "Wonder",
        "email": "alice@example.com",
        "hire_date": hire_date.to_rfc3339(),
        "salary": "7000.00",
        "tc_kimlik_no": "55667788990",
        "children_count": 0,
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
        &format!("/api/v1/hr/employees/{}", id),
        &token,
    )
    .to_request();
    let del_resp = test::call_service(&app, del_req).await;
    assert_eq!(del_resp.status(), StatusCode::NO_CONTENT);

    // Verify deleted - should return 404
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/hr/employees/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);

    // Restore
    let restore_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/hr/employees/{}/restore", id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::OK);

    let body = to_bytes(restore_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);
    assert_eq!(json["employee_number"], "EMP-DEL");

    // Verify restored
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/hr/employees/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn test_list_deleted_employees() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let hire_date = chrono::Utc::now();
    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/hr/employees",
        &token,
    )
    .set_json(json!({
        "employee_number": "EMP-LST-DEL",
        "first_name": "Charlie",
        "last_name": "Brown",
        "email": "charlie@example.com",
        "hire_date": hire_date.to_rfc3339(),
        "salary": "5500.00",
        "tc_kimlik_no": "99887766554",
        "children_count": 1,
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
        &format!("/api/v1/hr/employees/{}", id),
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    // List deleted
    let list_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/hr/employees/deleted",
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
async fn test_destroy_employee_permanently() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let hire_date = chrono::Utc::now();
    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/hr/employees",
        &token,
    )
    .set_json(json!({
        "employee_number": "EMP-DEST",
        "first_name": "Dave",
        "last_name": "Miller",
        "email": "dave@example.com",
        "hire_date": hire_date.to_rfc3339(),
        "salary": "8000.00",
        "tc_kimlik_no": "22334455667",
        "children_count": 0,
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
        &format!("/api/v1/hr/employees/{}", id),
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    // Permanently destroy
    let destroy_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/hr/employees/{}/destroy", id),
        &token,
    )
    .to_request();
    let destroy_resp = test::call_service(&app, destroy_req).await;
    assert_eq!(destroy_resp.status(), StatusCode::NO_CONTENT);

    // Should not be restorable
    let restore_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/hr/employees/{}/restore", id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// Security Tests
// ============================================================================

#[actix_web::test]
async fn test_employee_unauthorized() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/hr/employees")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_create_employee_unauthorized() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;

    let hire_date = chrono::Utc::now();
    let req = test::TestRequest::post()
        .uri("/api/v1/hr/employees")
        .set_json(json!({
            "employee_number": "EMP-UNAUTH",
            "first_name": "Unauthorized",
            "last_name": "User",
            "email": "unauth@example.com",
            "hire_date": hire_date.to_rfc3339(),
            "salary": "5000.00",
            "tc_kimlik_no": "11111111111",
            "children_count": 0,
            "tenant_id": 1
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
