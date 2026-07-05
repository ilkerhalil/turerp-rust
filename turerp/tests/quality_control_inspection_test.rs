//! Quality Control Inspection CRUD Integration Tests

use actix_web::{body::to_bytes, http::StatusCode, test};
use serde_json::json;

mod common;
use common::*;

#[actix_web::test]
async fn test_create_inspection_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;
    let product_id = seed_product!(&app, &token, 1);
    let work_order_id = seed_work_order!(&app, &token, 1, product_id);

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/manufacturing/inspections",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "work_order_id": work_order_id,
        "product_id": product_id,
        "inspection_type": "Visual",
        "quantity_inspected": "100.00",
        "quantity_passed": "95.00",
        "quantity_failed": "5.00",
        "status": "Passed",
        "inspector_id": 1,
        "notes": "Initial inspection"
    }))
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["product_id"], product_id);
    assert_eq!(json["inspection_type"], "Visual");
    assert_eq!(json["status"], "Passed");
    assert_eq!(json["quantity_inspected"], "100.00");
    assert!(json["id"].is_number());
}

#[actix_web::test]
async fn test_create_inspection_validation_error() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/manufacturing/inspections",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "product_id": 0,
        "inspection_type": "",
        "quantity_inspected": "0.00",
        "quantity_passed": "0.00",
        "quantity_failed": "0.00",
        "status": "Pending"
    }))
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn test_create_inspection_quantity_exceeded() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/manufacturing/inspections",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "product_id": 1,
        "inspection_type": "Visual",
        "quantity_inspected": "10.00",
        "quantity_passed": "8.00",
        "quantity_failed": "5.00",
        "status": "Pending"
    }))
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn test_list_inspections() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;
    let product_id = seed_product!(&app, &token, 1);

    for i in 1..=3 {
        let req = auth_request(
            actix_web::http::Method::POST,
            "/api/v1/manufacturing/inspections",
            &token,
        )
        .set_json(json!({
            "tenant_id": 1,
            "product_id": product_id,
            "inspection_type": format!("Visual-{}", i),
            "quantity_inspected": "100.00",
            "quantity_passed": "95.00",
            "quantity_failed": "5.00",
            "status": "Passed"
        }))
        .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/manufacturing/inspections",
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
async fn test_get_inspection_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;
    let product_id = seed_product!(&app, &token, 1);

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/manufacturing/inspections",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "product_id": product_id,
        "inspection_type": "Dimensional",
        "quantity_inspected": "50.00",
        "quantity_passed": "48.00",
        "quantity_failed": "2.00",
        "status": "Passed"
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/manufacturing/inspections/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);

    let body = to_bytes(get_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);
    assert_eq!(json["inspection_type"], "Dimensional");
}

#[actix_web::test]
async fn test_get_inspection_not_found() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/manufacturing/inspections/99999",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_update_inspection_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;
    let product_id = seed_product!(&app, &token, 1);

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/manufacturing/inspections",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "product_id": product_id,
        "inspection_type": "Visual",
        "quantity_inspected": "100.00",
        "quantity_passed": "90.00",
        "quantity_failed": "10.00",
        "status": "Pending"
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let update_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/manufacturing/inspections/{}", id),
        &token,
    )
    .set_json(json!({
        "status": "Failed",
        "quantity_passed": "85.00",
        "quantity_failed": "15.00",
        "inspector_id": 2,
        "notes": "Updated findings"
    }))
    .to_request();
    let update_resp = test::call_service(&app, update_req).await;
    assert_eq!(update_resp.status(), StatusCode::OK);

    let body = to_bytes(update_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "Failed");
    assert_eq!(json["quantity_passed"], "85.00");
    assert_eq!(json["quantity_failed"], "15.00");
    assert_eq!(json["inspector_id"], 2);
    assert_eq!(json["notes"], "Updated findings");
    assert!(json["inspected_at"].is_string());
}

#[actix_web::test]
async fn test_update_inspection_not_found() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::PUT,
        "/api/v1/manufacturing/inspections/99999",
        &token,
    )
    .set_json(json!({ "status": "Passed" }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_soft_delete_and_restore_inspection() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;
    let product_id = seed_product!(&app, &token, 1);

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/manufacturing/inspections",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "product_id": product_id,
        "inspection_type": "Visual",
        "quantity_inspected": "100.00",
        "quantity_passed": "95.00",
        "quantity_failed": "5.00",
        "status": "Passed"
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/manufacturing/inspections/{}", id),
        &token,
    )
    .to_request();
    let del_resp = test::call_service(&app, del_req).await;
    assert_eq!(del_resp.status(), StatusCode::OK);

    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/manufacturing/inspections/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);

    let restore_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/manufacturing/inspections/{}/restore", id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::OK);

    let body = to_bytes(restore_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);

    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/manufacturing/inspections/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn test_list_deleted_inspections() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;
    let product_id = seed_product!(&app, &token, 1);

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/manufacturing/inspections",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "product_id": product_id,
        "inspection_type": "Visual",
        "quantity_inspected": "100.00",
        "quantity_passed": "95.00",
        "quantity_failed": "5.00",
        "status": "Passed"
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/manufacturing/inspections/{}", id),
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    let list_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/manufacturing/inspections/deleted",
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
async fn test_destroy_inspection_permanently() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;
    let product_id = seed_product!(&app, &token, 1);

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/manufacturing/inspections",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "product_id": product_id,
        "inspection_type": "Visual",
        "quantity_inspected": "100.00",
        "quantity_passed": "95.00",
        "quantity_failed": "5.00",
        "status": "Passed"
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/manufacturing/inspections/{}", id),
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    let destroy_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/manufacturing/inspections/{}/destroy", id),
        &token,
    )
    .to_request();
    let destroy_resp = test::call_service(&app, destroy_req).await;
    assert_eq!(destroy_resp.status(), StatusCode::NO_CONTENT);

    let restore_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/manufacturing/inspections/{}/restore", id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_create_inspection_unauthorized() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/manufacturing/inspections")
        .set_json(json!({
            "tenant_id": 1,
            "product_id": 1,
            "inspection_type": "Visual",
            "quantity_inspected": "100.00",
            "quantity_passed": "95.00",
            "quantity_failed": "5.00",
            "status": "Passed"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
