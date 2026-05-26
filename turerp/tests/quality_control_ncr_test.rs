//! Quality Control NCR CRUD Integration Tests

use actix_web::{body::to_bytes, http::StatusCode, test};
use serde_json::json;

mod common;
use common::*;

#[actix_web::test]
async fn test_create_ncr_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/manufacturing/ncrs",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "inspection_id": 1,
        "product_id": 1,
        "ncr_type": "Minor",
        "description": "Surface scratch",
        "root_cause": "Handling damage",
        "corrective_action": "Improve packaging",
        "raised_by": 1
    }))
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["product_id"], 1);
    assert_eq!(json["description"], "Surface scratch");
    assert_eq!(json["ncr_type"], "Minor");
    assert_eq!(json["status"], "Open");
    assert!(json["id"].is_number());
}

#[actix_web::test]
async fn test_create_ncr_validation_error() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/manufacturing/ncrs",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "product_id": 0,
        "ncr_type": "Minor",
        "description": "",
        "raised_by": 0
    }))
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn test_list_ncrs() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    for i in 1..=3 {
        let req = auth_request(
            actix_web::http::Method::POST,
            "/api/v1/manufacturing/ncrs",
            &token,
        )
        .set_json(json!({
            "tenant_id": 1,
            "product_id": i,
            "ncr_type": "Minor",
            "description": format!("Defect {}", i),
            "raised_by": 1
        }))
        .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/manufacturing/ncrs",
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
async fn test_get_ncr_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/manufacturing/ncrs",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "product_id": 1,
        "ncr_type": "Major",
        "description": "Critical dimension out of tolerance",
        "raised_by": 1
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/manufacturing/ncrs/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);

    let body = to_bytes(get_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);
    assert_eq!(json["description"], "Critical dimension out of tolerance");
}

#[actix_web::test]
async fn test_get_ncr_not_found() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/manufacturing/ncrs/99999",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_update_ncr_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/manufacturing/ncrs",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "product_id": 1,
        "ncr_type": "Minor",
        "description": "Initial description",
        "raised_by": 1
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let update_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/manufacturing/ncrs/{}", id),
        &token,
    )
    .set_json(json!({
        "ncr_type": "Major",
        "description": "Updated description",
        "root_cause": "Machine misalignment",
        "corrective_action": "Recalibrate machine",
        "status": "CorrectiveAction"
    }))
    .to_request();
    let update_resp = test::call_service(&app, update_req).await;
    assert_eq!(update_resp.status(), StatusCode::OK);

    let body = to_bytes(update_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["ncr_type"], "Major");
    assert_eq!(json["description"], "Updated description");
    assert_eq!(json["root_cause"], "Machine misalignment");
    assert_eq!(json["corrective_action"], "Recalibrate machine");
    assert_eq!(json["status"], "CorrectiveAction");
}

#[actix_web::test]
async fn test_update_ncr_to_closed() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/manufacturing/ncrs",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "product_id": 1,
        "ncr_type": "Minor",
        "description": "Test close",
        "raised_by": 1
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let update_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/manufacturing/ncrs/{}", id),
        &token,
    )
    .set_json(json!({ "status": "Closed" }))
    .to_request();
    let update_resp = test::call_service(&app, update_req).await;
    assert_eq!(update_resp.status(), StatusCode::OK);

    let body = to_bytes(update_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "Closed");
    assert!(json["closed_at"].is_string());
}

#[actix_web::test]
async fn test_update_ncr_not_found() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::PUT,
        "/api/v1/manufacturing/ncrs/99999",
        &token,
    )
    .set_json(json!({ "status": "Closed" }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_soft_delete_and_restore_ncr() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/manufacturing/ncrs",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "product_id": 1,
        "ncr_type": "Minor",
        "description": "To be deleted",
        "raised_by": 1
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/manufacturing/ncrs/{}", id),
        &token,
    )
    .to_request();
    let del_resp = test::call_service(&app, del_req).await;
    assert_eq!(del_resp.status(), StatusCode::OK);

    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/manufacturing/ncrs/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);

    let restore_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/manufacturing/ncrs/{}/restore", id),
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
        &format!("/api/v1/manufacturing/ncrs/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn test_list_deleted_ncrs() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/manufacturing/ncrs",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "product_id": 1,
        "ncr_type": "Minor",
        "description": "Deleted NCR",
        "raised_by": 1
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/manufacturing/ncrs/{}", id),
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    let list_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/manufacturing/ncrs/deleted",
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
async fn test_destroy_ncr_permanently() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/manufacturing/ncrs",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "product_id": 1,
        "ncr_type": "Minor",
        "description": "To destroy",
        "raised_by": 1
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/manufacturing/ncrs/{}", id),
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    let destroy_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/manufacturing/ncrs/{}/destroy", id),
        &token,
    )
    .to_request();
    let destroy_resp = test::call_service(&app, destroy_req).await;
    assert_eq!(destroy_resp.status(), StatusCode::NO_CONTENT);

    let restore_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/manufacturing/ncrs/{}/restore", id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_create_ncr_unauthorized() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/manufacturing/ncrs")
        .set_json(json!({
            "tenant_id": 1,
            "product_id": 1,
            "ncr_type": "Minor",
            "description": "Defect",
            "raised_by": 1
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
