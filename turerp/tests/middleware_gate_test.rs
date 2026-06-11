//! Feature-flag gate middleware integration tests.
//!
//! Verifies the gate middleware behavior: gated routes return 404 when
//! their flag is disabled, 200 when enabled.
//!
//! The gate reads `AuthClaims` from request extensions (set by the
//! upstream `AuthUser` middleware). In tests we set it manually.

use std::sync::Arc;

use actix_web::{test, web, App, HttpMessage, HttpResponse};
use serde_json::json;
use turerp::domain::feature::{
    model::CreateFeatureFlag,
    repository::{FeatureFlagRepository, InMemoryFeatureFlagRepository},
    FeatureFlagService, FeatureFlagStatus,
};
use turerp::middleware::gate::{gate, GateConfig};
use turerp::utils::jwt::AuthClaims;

/// Build a tenant-scoped claim to inject into the request extensions.
fn build_claims(tenant_id: i64) -> AuthClaims {
    AuthClaims {
        sub: "1".to_string(),
        tenant_id,
        username: "testuser".to_string(),
        role: "user".to_string(),
        cari_id: None,
        exp: u64::MAX as i64,
        iat: 0,
        aud: "turerp-api".to_string(),
        iss: "turerp-auth".to_string(),
    }
}

/// Seed a single flag on a fresh in-memory repository. Returns the
/// wrapped service ready to be inserted into `app_data`.
async fn build_service(
    flag_name: &str,
    status: FeatureFlagStatus,
) -> web::Data<FeatureFlagService> {
    let repo = Arc::new(InMemoryFeatureFlagRepository::new());
    repo.create(CreateFeatureFlag {
        name: flag_name.to_string(),
        description: None,
        status: Some(status),
        tenant_id: Some(1),
    })
    .await
    .unwrap();
    web::Data::new(FeatureFlagService::new(repo))
}

#[actix_web::test]
async fn gated_route_returns_404_when_flag_disabled() {
    let app_data = build_service("tier2.manufacturing", FeatureFlagStatus::Disabled).await;

    let cfg = GateConfig {
        flag: "tier2.manufacturing".into(),
    };
    let app = test::init_service(
        App::new().app_data(app_data.clone()).service(
            web::resource("/api/v1/manufacturing/work-orders")
                .wrap(gate(cfg))
                .route(web::get().to(|| async { HttpResponse::Ok().json(json!({"ok": true})) })),
        ),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/manufacturing/work-orders")
        .to_request();
    req.extensions_mut().insert(build_claims(1));

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        404,
        "gated route with disabled flag must 404"
    );
}

#[actix_web::test]
async fn gated_route_passes_through_when_flag_enabled() {
    let app_data = build_service("tier2.manufacturing", FeatureFlagStatus::Enabled).await;

    let cfg = GateConfig {
        flag: "tier2.manufacturing".into(),
    };
    let app = test::init_service(
        App::new().app_data(app_data.clone()).service(
            web::resource("/api/v1/manufacturing/work-orders")
                .wrap(gate(cfg))
                .route(web::get().to(|| async { HttpResponse::Ok().json(json!({"ok": true})) })),
        ),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/manufacturing/work-orders")
        .to_request();
    req.extensions_mut().insert(build_claims(1));

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "gated route with enabled flag must reach handler"
    );
}

#[actix_web::test]
async fn gated_route_returns_404_when_claims_missing() {
    // Even when the flag is enabled, missing AuthClaims should fail-closed.
    let app_data = build_service("tier2.manufacturing", FeatureFlagStatus::Enabled).await;

    let cfg = GateConfig {
        flag: "tier2.manufacturing".into(),
    };
    let app = test::init_service(
        App::new().app_data(app_data.clone()).service(
            web::resource("/api/v1/manufacturing/work-orders")
                .wrap(gate(cfg))
                .route(web::get().to(|| async { HttpResponse::Ok().json(json!({"ok": true})) })),
        ),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/manufacturing/work-orders")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        404,
        "gated route with no AuthClaims must 404 (fail-closed)"
    );
}
