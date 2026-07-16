//! Job Domain CRUD Integration Tests

use actix_web::{body::to_bytes, http::StatusCode, test};
use serde_json::json;

use crate::common::*;

// ============================================================================
// Schedule Tests
// ============================================================================

#[actix_web::test]
async fn test_schedule_job_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(actix_web::http::Method::POST, "/api/v1/jobs", &token)
        .set_json(json!({
            "job_type": "send_reminders",
            "tenant_id": 1,
            "priority": "high",
            "max_attempts": 5
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["job_type"], "send_reminders");
    assert_eq!(json["status"], "pending");
    assert_eq!(json["priority"], "high");
    assert_eq!(json["tenant_id"], 1);
    assert_eq!(json["max_attempts"], 5);
    assert_eq!(json["attempts"], 0);
    assert!(json["id"].is_number());
}

#[actix_web::test]
async fn test_schedule_job_with_asset_id() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(actix_web::http::Method::POST, "/api/v1/jobs", &token)
        .set_json(json!({
            "job_type": "calculate_depreciation",
            "tenant_id": 1,
            "asset_id": 42
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["job_type"], "calculate_depreciation");
}

#[actix_web::test]
async fn test_schedule_job_with_custom_payload() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(actix_web::http::Method::POST, "/api/v1/jobs", &token)
        .set_json(json!({
            "job_type": "custom",
            "tenant_id": 1,
            "custom_name": "my_custom_job",
            "custom_payload": "{\"key\": \"value\"}"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["job_type"], "custom");
}

#[actix_web::test]
async fn test_schedule_job_unknown_type_validation_error() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(actix_web::http::Method::POST, "/api/v1/jobs", &token)
        .set_json(json!({
            "job_type": "nonexistent_type",
            "tenant_id": 1
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn test_schedule_job_defaults() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(actix_web::http::Method::POST, "/api/v1/jobs", &token)
        .set_json(json!({
            "job_type": "archive_logs",
            "tenant_id": 1
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["priority"], "normal");
    assert_eq!(json["max_attempts"], 3);
}

// ============================================================================
// Get / Next Pending Tests
// ============================================================================

#[actix_web::test]
async fn test_get_job_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/jobs", &token)
        .set_json(json!({
            "job_type": "send_reminders",
            "tenant_id": 1
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/jobs/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);

    let body = to_bytes(get_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);
    assert_eq!(json["job_type"], "send_reminders");
}

#[actix_web::test]
async fn test_get_job_not_found() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(actix_web::http::Method::GET, "/api/v1/jobs/99999", &token).to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_next_pending_job() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/jobs", &token)
        .set_json(json!({
            "job_type": "send_reminders",
            "tenant_id": 1,
            "priority": "critical"
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let req = auth_request(actix_web::http::Method::GET, "/api/v1/jobs/next", &token).to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);
    assert_eq!(json["priority"], "critical");
}

#[actix_web::test]
async fn test_next_pending_job_empty() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(actix_web::http::Method::GET, "/api/v1/jobs/next", &token).to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

// ============================================================================
// Lifecycle Tests
// ============================================================================

#[actix_web::test]
async fn test_job_full_lifecycle() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    // Schedule
    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/jobs", &token)
        .set_json(json!({
            "job_type": "archive_logs",
            "tenant_id": 1,
            "older_than_days": 30
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();
    assert_eq!(create_json["status"], "pending");

    // Start
    let start_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/jobs/{}/start", id),
        &token,
    )
    .to_request();
    let start_resp = test::call_service(&app, start_req).await;
    assert_eq!(start_resp.status(), StatusCode::OK);

    let body = to_bytes(start_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["message"], "Job marked as running");

    // Verify running
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/jobs/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    let body = to_bytes(get_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "running");
    assert_eq!(json["attempts"], 1);
    assert!(json["started_at"].is_string());

    // Complete
    let complete_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/jobs/{}/complete", id),
        &token,
    )
    .to_request();
    let complete_resp = test::call_service(&app, complete_req).await;
    assert_eq!(complete_resp.status(), StatusCode::OK);

    let body = to_bytes(complete_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["message"], "Job completed");

    // Verify completed
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/jobs/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    let body = to_bytes(get_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "completed");
    assert!(json["completed_at"].is_string());
}

#[actix_web::test]
async fn test_job_fail_with_retry() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/jobs", &token)
        .set_json(json!({
            "job_type": "send_reminders",
            "tenant_id": 1,
            "max_attempts": 3
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Start and fail
    let start_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/jobs/{}/start", id),
        &token,
    )
    .to_request();
    test::call_service(&app, start_req).await;

    let fail_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/jobs/{}/fail", id),
        &token,
    )
    .set_json(json!({ "error": "Database connection lost" }))
    .to_request();
    let fail_resp = test::call_service(&app, fail_req).await;
    assert_eq!(fail_resp.status(), StatusCode::OK);

    // Should be back to pending (retry) since attempts < max_attempts
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/jobs/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    let body = to_bytes(get_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "pending");
    assert_eq!(json["attempts"], 1);
    assert_eq!(json["last_error"], "Database connection lost");
    assert!(json["scheduled_at"].is_string());
}

#[actix_web::test]
async fn test_job_fail_max_retries() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/jobs", &token)
        .set_json(json!({
            "job_type": "send_reminders",
            "tenant_id": 1,
            "max_attempts": 1
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let start_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/jobs/{}/start", id),
        &token,
    )
    .to_request();
    test::call_service(&app, start_req).await;

    let fail_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/jobs/{}/fail", id),
        &token,
    )
    .set_json(json!({ "error": "Fatal error" }))
    .to_request();
    let fail_resp = test::call_service(&app, fail_req).await;
    assert_eq!(fail_resp.status(), StatusCode::OK);

    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/jobs/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    let body = to_bytes(get_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "failed");
    assert_eq!(json["last_error"], "Fatal error");
    assert!(json["completed_at"].is_string());
}

// ============================================================================
// Cancel / Retry Tests
// ============================================================================

#[actix_web::test]
async fn test_cancel_pending_job() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/jobs", &token)
        .set_json(json!({
            "job_type": "send_reminders",
            "tenant_id": 1
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let cancel_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/jobs/{}/cancel", id),
        &token,
    )
    .to_request();
    let cancel_resp = test::call_service(&app, cancel_req).await;
    assert_eq!(cancel_resp.status(), StatusCode::OK);

    let body = to_bytes(cancel_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["message"], "Job cancelled");

    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/jobs/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    let body = to_bytes(get_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "cancelled");
}

#[actix_web::test]
async fn test_cancel_running_job_fails() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/jobs", &token)
        .set_json(json!({
            "job_type": "send_reminders",
            "tenant_id": 1
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let start_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/jobs/{}/start", id),
        &token,
    )
    .to_request();
    test::call_service(&app, start_req).await;

    let cancel_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/jobs/{}/cancel", id),
        &token,
    )
    .to_request();
    let cancel_resp = test::call_service(&app, cancel_req).await;
    // Scheduler errors map to ApiError::Internal -> 500
    assert_eq!(cancel_resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[actix_web::test]
async fn test_retry_failed_job() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/jobs", &token)
        .set_json(json!({
            "job_type": "send_reminders",
            "tenant_id": 1,
            "max_attempts": 1
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let start_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/jobs/{}/start", id),
        &token,
    )
    .to_request();
    test::call_service(&app, start_req).await;

    let fail_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/jobs/{}/fail", id),
        &token,
    )
    .set_json(json!({ "error": "boom" }))
    .to_request();
    test::call_service(&app, fail_req).await;

    let retry_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/jobs/{}/retry", id),
        &token,
    )
    .to_request();
    let retry_resp = test::call_service(&app, retry_req).await;
    assert_eq!(retry_resp.status(), StatusCode::OK);

    let body = to_bytes(retry_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["message"], "Job queued for retry");

    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/jobs/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    let body = to_bytes(get_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "pending");
    assert_eq!(json["attempts"], 0);
    assert!(json["last_error"].is_null());
}

#[actix_web::test]
async fn test_retry_non_failed_job_fails() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/jobs", &token)
        .set_json(json!({
            "job_type": "send_reminders",
            "tenant_id": 1
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let retry_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/jobs/{}/retry", id),
        &token,
    )
    .to_request();
    let retry_resp = test::call_service(&app, retry_req).await;
    // Scheduler errors map to ApiError::Internal -> 500
    assert_eq!(retry_resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

// ============================================================================
// List / Cleanup Tests
// ============================================================================

#[actix_web::test]
async fn test_list_jobs_by_status() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    for _ in 0..3 {
        let req = auth_request(actix_web::http::Method::POST, "/api/v1/jobs", &token)
            .set_json(json!({
                "job_type": "send_reminders",
                "tenant_id": 1
            }))
            .to_request();
        test::call_service(&app, req).await;
    }

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/jobs/status/pending",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json.as_array().unwrap();
    assert_eq!(items.len(), 3);
    assert_eq!(items[0]["status"], "pending");
    assert_eq!(items[0]["tenant_id"], 1);
}

#[actix_web::test]
async fn test_list_jobs_by_status_empty() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/jobs/status/failed",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json.as_array().unwrap();
    assert!(items.is_empty());
}

#[actix_web::test]
async fn test_cleanup_jobs() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(actix_web::http::Method::POST, "/api/v1/jobs", &token)
        .set_json(json!({
            "job_type": "send_reminders",
            "tenant_id": 1
        }))
        .to_request();
    let create_resp = test::call_service(&app, req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let start_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/jobs/{}/start", id),
        &token,
    )
    .to_request();
    test::call_service(&app, start_req).await;

    let complete_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/jobs/{}/complete", id),
        &token,
    )
    .to_request();
    test::call_service(&app, complete_req).await;

    let cleanup_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/jobs/cleanup/0",
        &token,
    )
    .to_request();
    let cleanup_resp = test::call_service(&app, cleanup_req).await;
    assert_eq!(cleanup_resp.status(), StatusCode::OK);

    let body = to_bytes(cleanup_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["cleaned"], 1);
}

// ============================================================================
// Authorization Tests
// ============================================================================

#[actix_web::test]
async fn test_schedule_job_unauthorized() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/jobs")
        .set_json(json!({
            "job_type": "send_reminders",
            "tenant_id": 1
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_get_job_unauthorized() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;

    let req = test::TestRequest::get().uri("/api/v1/jobs/1").to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[actix_web::test]
async fn test_start_job_not_found() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/jobs/99999/start",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    // Scheduler errors map to ApiError::Internal -> 500
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[actix_web::test]
async fn test_complete_job_not_found() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/jobs/99999/complete",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    // Scheduler errors map to ApiError::Internal -> 500
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[actix_web::test]
async fn test_fail_job_not_found() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/jobs/99999/fail",
        &token,
    )
    .set_json(json!({ "error": "not found" }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    // Scheduler errors map to ApiError::Internal -> 500
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[actix_web::test]
async fn test_multiple_job_types() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let types = vec![
        json!({ "job_type": "send_reminders", "tenant_id": 1 }),
        json!({ "job_type": "archive_logs", "tenant_id": 1, "older_than_days": 7 }),
        json!({ "job_type": "generate_report", "tenant_id": 1, "report_type": "balance", "params": "{}" }),
        json!({ "job_type": "run_payroll", "tenant_id": 1, "period": "2024-01" }),
    ];

    for payload in types {
        let req = auth_request(actix_web::http::Method::POST, "/api/v1/jobs", &token)
            .set_json(payload)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/jobs/status/pending",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json.as_array().unwrap().len(), 4);
}

#[actix_web::test]
async fn test_priority_ordering_next_pending() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    // Create low priority job first
    let low_req = auth_request(actix_web::http::Method::POST, "/api/v1/jobs", &token)
        .set_json(json!({
            "job_type": "send_reminders",
            "tenant_id": 1,
            "priority": "low"
        }))
        .to_request();
    test::call_service(&app, low_req).await;

    // Create critical priority job second
    let critical_req = auth_request(actix_web::http::Method::POST, "/api/v1/jobs", &token)
        .set_json(json!({
            "job_type": "send_reminders",
            "tenant_id": 1,
            "priority": "critical"
        }))
        .to_request();
    test::call_service(&app, critical_req).await;

    // Next pending should return critical priority job
    let req = auth_request(actix_web::http::Method::GET, "/api/v1/jobs/next", &token).to_request();
    let resp = test::call_service(&app, req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["priority"], "critical");
}

#[actix_web::test]
async fn test_tenant_isolation_list_by_status() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;
    let (token_b, _user_id_b) = register_admin(&state, 2).await;

    // Create jobs for tenant 1
    for _ in 0..2 {
        let req = auth_request(actix_web::http::Method::POST, "/api/v1/jobs", &token)
            .set_json(json!({
                "job_type": "send_reminders",
                "tenant_id": 1
            }))
            .to_request();
        test::call_service(&app, req).await;
    }

    // Create job for tenant 2 (via tenant 2's own admin token; the request body
    // tenant_id is no longer trusted — a job is always created under the caller's
    // tenant, so cross-tenant seeding must use a separate token).
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/jobs", &token_b)
        .set_json(json!({
            "job_type": "send_reminders",
            "tenant_id": 2
        }))
        .to_request();
    test::call_service(&app, req).await;

    // List should only return tenant 1 jobs (from admin token tenant_id)
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/jobs/status/pending",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json.as_array().unwrap();
    assert_eq!(items.len(), 2);
    for item in items {
        assert_eq!(item["tenant_id"], 1);
    }
}

#[actix_web::test]
async fn test_list_jobs_by_invalid_status() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/jobs/status/invalid_status",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// ============================================================================
// Cross-tenant API Integration Tests
// ============================================================================
//
// These tests verify that admin tokens for tenant A cannot read, mutate, or
// cancel jobs that belong to tenant B — the heart of the Phase 3 audit fix.

#[actix_web::test]
async fn test_cross_tenant_get_returns_404() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token_a, _uid_a) = register_admin(&state, 1).await;
    let (token_b, _uid_b) = register_admin(&state, 2).await;

    // tenant 1 creates a job
    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/jobs", &token_a)
        .set_json(json!({
            "job_type": "send_reminders",
            "tenant_id": 1,
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // tenant 2 attempts to read it -> should be 404 (not 200, not 403)
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/jobs/{}", id),
        &token_b,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "tenant B must not see tenant A's job"
    );
}

#[actix_web::test]
async fn test_cross_tenant_cancel_is_blocked() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token_a, _) = register_admin(&state, 1).await;
    let (token_b, _) = register_admin(&state, 2).await;

    // tenant 1 creates a job
    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/jobs", &token_a)
        .set_json(json!({
            "job_type": "send_reminders",
            "tenant_id": 1,
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // tenant 2 tries to cancel tenant 1's job -> must fail
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/jobs/{}/cancel", id),
        &token_b,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_client_error() || resp.status().is_server_error(),
        "tenant B must not cancel tenant A's job; got {}",
        resp.status()
    );

    // tenant 1 can still cancel its own job
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/jobs/{}/cancel", id),
        &token_a,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn test_cross_tenant_complete_is_blocked() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token_a, _) = register_admin(&state, 1).await;
    let (token_b, _) = register_admin(&state, 2).await;

    // tenant 1 creates a job
    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/jobs", &token_a)
        .set_json(json!({
            "job_type": "send_reminders",
            "tenant_id": 1,
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Mark running first (as tenant 1, the rightful owner)
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/jobs/{}/start", id),
        &token_a,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // tenant 2 tries to mark tenant 1's job complete -> must fail
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/jobs/{}/complete", id),
        &token_b,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_client_error() || resp.status().is_server_error(),
        "tenant B must not complete tenant A's job; got {}",
        resp.status()
    );

    // Verify status is still 'running' (not 'completed')
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/jobs/{}", id),
        &token_a,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "running", "status must remain unchanged");
}

#[actix_web::test]
async fn test_cross_tenant_list_does_not_leak() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token_a, _) = register_admin(&state, 1).await;
    let (token_b, _) = register_admin(&state, 2).await;

    // 3 jobs for tenant 1
    for _ in 0..3 {
        let req = auth_request(actix_web::http::Method::POST, "/api/v1/jobs", &token_a)
            .set_json(json!({
                "job_type": "send_reminders",
                "tenant_id": 1,
            }))
            .to_request();
        test::call_service(&app, req).await;
    }
    // 2 jobs for tenant 2
    for _ in 0..2 {
        let req = auth_request(actix_web::http::Method::POST, "/api/v1/jobs", &token_b)
            .set_json(json!({
                "job_type": "send_reminders",
                "tenant_id": 2,
            }))
            .to_request();
        test::call_service(&app, req).await;
    }

    // tenant 1 list -> only 3
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/jobs/status/pending",
        &token_a,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json.as_array().unwrap();
    assert_eq!(items.len(), 3);
    for item in items {
        assert_eq!(
            item["tenant_id"], 1,
            "tenant A list must not contain tenant B jobs"
        );
    }

    // tenant 2 list -> only 2
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/jobs/status/pending",
        &token_b,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json.as_array().unwrap();
    assert_eq!(items.len(), 2);
    for item in items {
        assert_eq!(
            item["tenant_id"], 2,
            "tenant B list must not contain tenant A jobs"
        );
    }
}
