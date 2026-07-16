//! MFA Integration Tests

use actix_web::{body::to_bytes, http::StatusCode, test, web, App};
use serde_json::json;

mod common;
use common::*;

use turerp::middleware::JwtAuthMiddleware;

/// Build test app with MFA service wired in
fn build_test_app_with_mfa(
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
        .wrap(JwtAuthMiddleware::new(jwt))
        .app_data(web::Data::new(state.clone()))
        .app_data(state.auth.auth_service.clone())
        .app_data(state.auth.user_service.clone())
        .app_data(state.auth.jwt_service.clone())
        .app_data(state.auth.mfa_service.clone())
        .service(
            web::scope("/api")
                .configure(common::configure_all_routes)
                .configure(common::configure_v1_routes),
        )
}

/// Generate a valid TOTP code from a base32 secret
fn generate_totp_code(secret: &str) -> String {
    use totp_rs::{Algorithm, TOTP};
    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret.as_bytes().to_vec(),
        None,
        "".to_string(),
    )
    .expect("valid TOTP");
    totp.generate_current().expect("valid current TOTP")
}

// ============================================================================
// MFA Tests
// ============================================================================

#[actix_web::test]
async fn test_mfa_setup_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_mfa(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/auth/mfa/setup",
        &token,
    )
    .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["qr_code_uri"].is_string());
    assert!(json["secret"].is_string());
    assert!(!json["secret"].as_str().unwrap().is_empty());
}

#[actix_web::test]
async fn test_mfa_verify_setup_and_status() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_mfa(&state)).await;
    let (token, user_id) = register_admin(&state, 1).await;

    // Setup MFA
    let setup_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/auth/mfa/setup",
        &token,
    )
    .to_request();
    let setup_resp = test::call_service(&app, setup_req).await;
    let body = to_bytes(setup_resp.into_body()).await.unwrap();
    let setup_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let secret = setup_json["secret"].as_str().unwrap().to_string();
    let code = generate_totp_code(&secret);

    // Verify setup
    let verify_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/auth/mfa/verify-setup",
        &token,
    )
    .set_json(json!({ "code": code }))
    .to_request();
    let verify_resp = test::call_service(&app, verify_req).await;
    assert_eq!(verify_resp.status(), StatusCode::OK);

    let body = to_bytes(verify_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["user_id"], user_id);
    assert_eq!(json["mfa_enabled"], true);
    assert_eq!(json["method"], "totp");

    // Check status
    let status_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/auth/mfa/status",
        &token,
    )
    .to_request();
    let status_resp = test::call_service(&app, status_req).await;
    assert_eq!(status_resp.status(), StatusCode::OK);

    let body = to_bytes(status_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["mfa_enabled"], true);
    assert_eq!(json["method"], "totp");
}

#[actix_web::test]
async fn test_mfa_disable_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_mfa(&state)).await;
    let (token, user_id) = register_admin(&state, 1).await;

    // Setup and verify MFA first
    let setup_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/auth/mfa/setup",
        &token,
    )
    .to_request();
    let setup_resp = test::call_service(&app, setup_req).await;
    let body = to_bytes(setup_resp.into_body()).await.unwrap();
    let setup_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let secret = setup_json["secret"].as_str().unwrap().to_string();
    let code = generate_totp_code(&secret);

    let verify_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/auth/mfa/verify-setup",
        &token,
    )
    .set_json(json!({ "code": code }))
    .to_request();
    test::call_service(&app, verify_req).await;

    // Disable MFA with correct password
    let disable_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/auth/mfa/disable",
        &token,
    )
    .set_json(json!({ "password": "Password123!" }))
    .to_request();
    let disable_resp = test::call_service(&app, disable_req).await;
    assert_eq!(disable_resp.status(), StatusCode::OK);

    let body = to_bytes(disable_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["user_id"], user_id);
    assert_eq!(json["mfa_enabled"], false);
    assert_eq!(json["method"], "none");
}

/// Regression test for the MFA disable password bypass (PR #147 / #314 class).
/// `verify_password` returned `Ok(false)` on a wrong password, which a bare `?`
/// silently dropped, allowing MFA to be disabled with ANY password. This test
/// asserts that a wrong password is rejected with 403 AND that MFA remains
/// enabled afterwards — guarding against a future revert to `verify_password`.
#[actix_web::test]
async fn test_mfa_disable_wrong_password_rejected_and_mfa_stays_enabled() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_mfa(&state)).await;
    let (token, user_id) = register_admin(&state, 1).await;

    // Setup and verify MFA first
    let setup_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/auth/mfa/setup",
        &token,
    )
    .to_request();
    let setup_resp = test::call_service(&app, setup_req).await;
    let body = to_bytes(setup_resp.into_body()).await.unwrap();
    let setup_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let secret = setup_json["secret"].as_str().unwrap().to_string();
    let code = generate_totp_code(&secret);

    let verify_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/auth/mfa/verify-setup",
        &token,
    )
    .set_json(json!({ "code": code }))
    .to_request();
    let verify_resp = test::call_service(&app, verify_req).await;
    assert_eq!(verify_resp.status(), StatusCode::OK);

    // Attempt to disable MFA with a WRONG password — must be rejected (403)
    let disable_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/auth/mfa/disable",
        &token,
    )
    .set_json(json!({ "password": "WrongPassword456!" }))
    .to_request();
    let disable_resp = test::call_service(&app, disable_req).await;
    assert_eq!(
        disable_resp.status(),
        StatusCode::FORBIDDEN,
        "wrong password must be rejected, not allow MFA disable"
    );

    // MFA must still be enabled — the disable did not take effect
    let status_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/auth/mfa/status",
        &token,
    )
    .to_request();
    let status_resp = test::call_service(&app, status_req).await;
    assert_eq!(status_resp.status(), StatusCode::OK);

    let body = to_bytes(status_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["user_id"], user_id);
    assert_eq!(
        json["mfa_enabled"], true,
        "MFA must remain enabled after a wrong-password disable attempt"
    );
    assert_eq!(json["method"], "totp");
}

#[actix_web::test]
async fn test_mfa_regenerate_backup_codes() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_mfa(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    // Setup and verify MFA first
    let setup_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/auth/mfa/setup",
        &token,
    )
    .to_request();
    let setup_resp = test::call_service(&app, setup_req).await;
    let body = to_bytes(setup_resp.into_body()).await.unwrap();
    let setup_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let secret = setup_json["secret"].as_str().unwrap().to_string();
    let code = generate_totp_code(&secret);

    let verify_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/auth/mfa/verify-setup",
        &token,
    )
    .set_json(json!({ "code": code }))
    .to_request();
    test::call_service(&app, verify_req).await;

    // Regenerate backup codes
    let regen_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/auth/mfa/regenerate-backup-codes",
        &token,
    )
    .to_request();
    let regen_resp = test::call_service(&app, regen_req).await;
    assert_eq!(regen_resp.status(), StatusCode::OK);

    let body = to_bytes(regen_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let codes = json["backup_codes"].as_array().unwrap();
    assert_eq!(codes.len(), 10);
}

#[actix_web::test]
async fn test_mfa_status_no_setup() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_mfa(&state)).await;
    let (token, user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/auth/mfa/status",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["user_id"], user_id);
    assert_eq!(json["mfa_enabled"], false);
    assert_eq!(json["method"], "none");
}

#[actix_web::test]
async fn test_mfa_verify_setup_invalid_code() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_mfa(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    // Setup MFA
    let setup_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/auth/mfa/setup",
        &token,
    )
    .to_request();
    test::call_service(&app, setup_req).await;

    // Verify with invalid code
    let verify_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/auth/mfa/verify-setup",
        &token,
    )
    .set_json(json!({ "code": "000000" }))
    .to_request();
    let verify_resp = test::call_service(&app, verify_req).await;
    assert_eq!(verify_resp.status(), StatusCode::BAD_REQUEST);
}

// ============================================================================
// Authorization Tests
// ============================================================================

#[actix_web::test]
async fn test_mfa_unauthorized() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_mfa(&state)).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/mfa/setup")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_mfa_regenerate_without_mfa_enabled() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_mfa(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    // Try to regenerate backup codes without enabling MFA
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/auth/mfa/regenerate-backup-codes",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}
