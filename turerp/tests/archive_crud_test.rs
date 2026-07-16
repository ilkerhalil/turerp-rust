//! Archive CRUD Integration Tests

use actix_web::{body::to_bytes, http::StatusCode, test, web, App};
use serde_json::json;

use crate::common::*;

use turerp::api::{auth_configure, v1_archive_configure};

fn build_test_app_with_archive(
    state: &turerp::app::AppState,
) -> App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse<
            actix_web::body::EitherBody<actix_web::body::BoxBody>,
        >,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    let jwt = create_test_jwt_service();
    App::new()
        .wrap(turerp::middleware::JwtAuthMiddleware::new(jwt))
        .app_data(web::Data::new(state.clone()))
        .app_data(state.auth.auth_service.clone())
        .app_data(state.auth.user_service.clone())
        .app_data(state.auth.jwt_service.clone())
        .app_data(state.analytics.archive_service.clone())
        .service(
            web::scope("/api")
                .configure(auth_configure)
                .configure(v1_archive_configure),
        )
}

// ============================================================================
// CRUD Tests
// ============================================================================

#[actix_web::test]
async fn test_create_archive_policy_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_archive(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/archive/policies",
        &token,
    )
    .set_json(json!({
        "name": "Old Invoices",
        "table_name": "invoices",
        "age_days": 365,
        "is_active": true
    }))
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["name"], "Old Invoices");
    assert_eq!(json["table_name"], "invoices");
    assert_eq!(json["age_days"], 365);
    assert!(json["id"].is_number());
}

#[actix_web::test]
async fn test_list_archive_policies_paginated() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_archive(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    for i in 1..=3 {
        let req = auth_request(
            actix_web::http::Method::POST,
            "/api/v1/archive/policies",
            &token,
        )
        .set_json(json!({
            "name": format!("Policy {}", i),
            "table_name": format!("table_{}", i),
            "age_days": 100 * i,
            "is_active": true
        }))
        .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/archive/policies?page=1&per_page=2",
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
async fn test_get_archive_policy_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_archive(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/archive/policies",
        &token,
    )
    .set_json(json!({
        "name": "Get Test Policy",
        "table_name": "test_table",
        "age_days": 90,
        "is_active": true
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/archive/policies/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);

    let body = to_bytes(get_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);
    assert_eq!(json["name"], "Get Test Policy");
}

#[actix_web::test]
async fn test_get_archive_policy_not_found() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_archive(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/archive/policies/99999",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_update_archive_policy_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_archive(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/archive/policies",
        &token,
    )
    .set_json(json!({
        "name": "Original Name",
        "table_name": "orig_table",
        "age_days": 180,
        "is_active": true
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let update_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/archive/policies/{}", id),
        &token,
    )
    .set_json(json!({
        "name": "Updated Name",
        "age_days": 200,
        "is_active": false
    }))
    .to_request();
    let update_resp = test::call_service(&app, update_req).await;
    assert_eq!(update_resp.status(), StatusCode::OK);

    let body = to_bytes(update_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["name"], "Updated Name");
    assert_eq!(json["age_days"], 200);
    assert_eq!(json["is_active"], false);
}

#[actix_web::test]
async fn test_delete_archive_policy() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_archive(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/archive/policies",
        &token,
    )
    .set_json(json!({
        "name": "Delete Test Policy",
        "table_name": "del_table",
        "age_days": 30,
        "is_active": true
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/archive/policies/{}", id),
        &token,
    )
    .to_request();
    let del_resp = test::call_service(&app, del_req).await;
    assert_eq!(del_resp.status(), StatusCode::NO_CONTENT);

    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/archive/policies/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_list_active_archive_policies() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_archive(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    for i in 1..=2 {
        let req = auth_request(
            actix_web::http::Method::POST,
            "/api/v1/archive/policies",
            &token,
        )
        .set_json(json!({
            "name": format!("Active Policy {}", i),
            "table_name": format!("active_tbl_{}", i),
            "age_days": 100,
            "is_active": i == 1
        }))
        .to_request();
        test::call_service(&app, req).await;
    }

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/archive/policies/active",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json.as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["is_active"], true);
}

// ============================================================================
// Archive Job Tests
// ============================================================================

#[actix_web::test]
async fn test_create_and_get_archive_job() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_archive(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let policy_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/archive/policies",
        &token,
    )
    .set_json(json!({
        "name": "Job Test Policy",
        "table_name": "invoices",
        "age_days": 365,
        "is_active": true
    }))
    .to_request();
    let policy_resp = test::call_service(&app, policy_req).await;
    let body = to_bytes(policy_resp.into_body()).await.unwrap();
    let policy_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let policy_id = policy_json["id"].as_i64().unwrap();

    let job_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/archive/jobs",
        &token,
    )
    .set_json(json!({ "policy_id": policy_id }))
    .to_request();
    let job_resp = test::call_service(&app, job_req).await;
    assert_eq!(job_resp.status(), StatusCode::CREATED);

    let body = to_bytes(job_resp.into_body()).await.unwrap();
    let job_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let job_id = job_json["id"].as_i64().unwrap();
    assert_eq!(job_json["policy_id"], policy_id);

    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/archive/jobs/{}", job_id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);

    let body = to_bytes(get_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], job_id);
}

#[actix_web::test]
async fn test_list_archive_jobs() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_archive(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let policy_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/archive/policies",
        &token,
    )
    .set_json(json!({
        "name": "Job List Policy",
        "table_name": "invoices",
        "age_days": 365,
        "is_active": true
    }))
    .to_request();
    let policy_resp = test::call_service(&app, policy_req).await;
    let body = to_bytes(policy_resp.into_body()).await.unwrap();
    let policy_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let policy_id = policy_json["id"].as_i64().unwrap();

    for _ in 1..=2 {
        let job_req = auth_request(
            actix_web::http::Method::POST,
            "/api/v1/archive/jobs",
            &token,
        )
        .set_json(json!({ "policy_id": policy_id }))
        .to_request();
        test::call_service(&app, job_req).await;
    }

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/archive/jobs?page=1&per_page=10",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["items"].as_array().unwrap().len() >= 2);
}

// ============================================================================
// Archive Record Tests (restore via service)
// ============================================================================

#[actix_web::test]
async fn test_restore_archive_records() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_archive(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let archive_service = state.analytics.archive_service.get_ref();

    let record = archive_service
        .create_record(1, "invoices".to_string(), 42, json!({"amount": 1000}), 1)
        .await
        .unwrap();

    let restore_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/archive/records/restore",
        &token,
    )
    .set_json(json!({ "record_ids": [record.id] }))
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::OK);

    let body = to_bytes(restore_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["restored"], 1);
    assert_eq!(json["failed"].as_array().unwrap().len(), 0);
}

#[actix_web::test]
async fn test_list_archive_records_with_filter() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_archive(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let archive_service = state.analytics.archive_service.get_ref();

    archive_service
        .create_record(1, "invoices".to_string(), 10, json!({"amount": 100}), 1)
        .await
        .unwrap();
    archive_service
        .create_record(1, "payments".to_string(), 20, json!({"amount": 200}), 1)
        .await
        .unwrap();

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/archive/records?source_table=invoices",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["source_table"], "invoices");
}

// ============================================================================
// Unauthorized / Not Found
// ============================================================================

#[actix_web::test]
async fn test_archive_unauthorized_without_token() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_archive(&state)).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/archive/policies")
        .set_json(json!({
            "name": "No Auth",
            "table_name": "test",
            "age_days": 30,
            "is_active": true
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_archive_job_not_found() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_archive(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/archive/jobs/99999",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
