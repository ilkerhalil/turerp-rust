//! E-Archive Integration Tests

use actix_web::{body::to_bytes, http::StatusCode, test, web, App};
use serde_json::json;

mod common;
use common::*;

use turerp::api::{auth_configure, v1_earchive_configure};

fn build_test_app_with_earchive(
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
        .app_data(state.i18n.clone())
        .app_data(state.integration.earchive_service.clone())
        .service(
            web::scope("/api")
                .configure(auth_configure)
                .configure(v1_earchive_configure),
        )
}

// ============================================================================
// CRUD Tests
// ============================================================================

#[actix_web::test]
async fn test_generate_earchive_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_earchive(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/earchive/generate",
        &token,
    )
    .set_json(json!({
        "invoice_id": 42,
        "document_type": "EArchiveInvoice"
    }))
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["document_type"], "EArchiveInvoice");
    assert_eq!(json["status"], "Generated");
    assert!(json["id"].is_number());
    assert!(json["uuid"].is_string());
}

#[actix_web::test]
async fn test_get_earchive_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_earchive(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/earchive/generate",
        &token,
    )
    .set_json(json!({
        "invoice_id": 1,
        "document_type": "EArchiveInvoice"
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/earchive/{}", id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);

    let body = to_bytes(get_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);
    assert_eq!(json["document_type"], "EArchiveInvoice");
}

#[actix_web::test]
async fn test_get_earchive_not_found() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_earchive(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/earchive/99999",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_generate_earchive_smm() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_earchive(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/earchive/generate",
        &token,
    )
    .set_json(json!({
        "invoice_id": 99,
        "document_type": "ESerbestMeslekMakbuzu"
    }))
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["document_type"], "ESerbestMeslekMakbuzu");
    assert_eq!(json["status"], "Generated");
    assert!(json["id"].is_number());
    assert!(json["uuid"].is_string());
}

#[actix_web::test]
async fn test_sign_earchive() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_earchive(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/earchive/generate",
        &token,
    )
    .set_json(json!({
        "invoice_id": 10,
        "document_type": "EArchiveInvoice"
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let sign_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/earchive/{}/sign", id),
        &token,
    )
    .to_request();
    let sign_resp = test::call_service(&app, sign_req).await;
    assert_eq!(sign_resp.status(), StatusCode::OK);

    let body = to_bytes(sign_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);
    assert_eq!(json["status"], "Signed");
}

#[actix_web::test]
async fn test_send_earchive() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_earchive(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/earchive/generate",
        &token,
    )
    .set_json(json!({
        "invoice_id": 11,
        "document_type": "EArchiveInvoice"
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    // Sign first
    let sign_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/earchive/{}/sign", id),
        &token,
    )
    .to_request();
    test::call_service(&app, sign_req).await;

    let send_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/earchive/{}/send", id),
        &token,
    )
    .to_request();
    let send_resp = test::call_service(&app, send_req).await;
    assert_eq!(send_resp.status(), StatusCode::OK);

    let body = to_bytes(send_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);
    assert_eq!(json["status"], "Sent");
    assert!(json["sent_at"].is_string());
}

#[actix_web::test]
async fn test_cancel_earchive() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_earchive(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/earchive/generate",
        &token,
    )
    .set_json(json!({
        "invoice_id": 12,
        "document_type": "EArchiveInvoice"
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let cancel_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/earchive/{}/cancel", id),
        &token,
    )
    .to_request();
    let cancel_resp = test::call_service(&app, cancel_req).await;
    assert_eq!(cancel_resp.status(), StatusCode::OK);

    let body = to_bytes(cancel_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], id);
    assert_eq!(json["status"], "Cancelled");
}

// ============================================================================
// Authorization Tests
// ============================================================================

#[actix_web::test]
async fn test_earchive_unauthorized() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_earchive(&state)).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/earchive/generate")
        .set_json(json!({
            "invoice_id": 1,
            "document_type": "EArchiveInvoice"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_earchive_normal_user_forbidden() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_earchive(&state)).await;
    let (token, _user_id) = register_user!(&app, 1);

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/earchive/generate",
        &token,
    )
    .set_json(json!({
        "invoice_id": 1,
        "document_type": "EArchiveInvoice"
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}
