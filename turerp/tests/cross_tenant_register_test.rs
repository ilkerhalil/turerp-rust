//! Integration tests for cross-tenant registration security fix (issue #319).
//!
//! `POST /api/v1/auth/register` is public (in PUBLIC_PATHS) and accepts a
//! caller-supplied `tenant_id`. Before the fix, an unauthenticated attacker
//! could supply an arbitrary tenant_id and register into any tenant — a
//! cross-tenant data breach with no credentials. The fix validates that the
//! tenant exists and is active before creating the user.

use actix_web::test;
use serde_json::json;

use crate::common::{build_test_app, create_test_app_state};

/// Registering into a non-existent tenant must be rejected (404 NotFound).
#[actix_web::test]
async fn test_register_nonexistent_tenant_rejected() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(json!({
            "username": "attacker",
            "email": "attacker@evil.com",
            "full_name": "Attacker",
            "password": "Password123!",
            "tenant_id": 99999
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        actix_web::http::StatusCode::BAD_REQUEST,
        "Registration into non-existent tenant must return 400 (not 201), \
         without revealing whether the tenant exists",
    );
}

/// Registering into an existing, active tenant must succeed (201 Created).
#[actix_web::test]
async fn test_register_existing_tenant_succeeds() {
    let state = create_test_app_state().await;

    // The in-memory app state seeds a default tenant with id=1 during
    // create_app_state_in_memory (TenantService seeds it on first access).
    // We need to ensure tenant 1 exists — create it via the tenant service.
    use turerp::domain::tenant::model::CreateTenant;
    let _ = state
        .admin
        .tenant_service
        .get_ref()
        .create_tenant(CreateTenant {
            name: "Test Tenant".to_string(),
            subdomain: "test".to_string(),
            base_currency: "TRY".to_string(),
            supported_currencies: vec![],
        })
        .await;

    let app = test::init_service(build_test_app(&state)).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(json!({
            "username": "legit_user",
            "email": "legit@test.com",
            "full_name": "Legit User",
            "password": "Password123!",
            "tenant_id": 1
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        actix_web::http::StatusCode::CREATED,
        "Registration into existing active tenant must succeed"
    );
}

/// Registering into a soft-deleted tenant must be rejected.
#[actix_web::test]
async fn test_register_deleted_tenant_rejected() {
    let state = create_test_app_state().await;

    use turerp::domain::tenant::model::CreateTenant;
    let tenant = state
        .admin
        .tenant_service
        .get_ref()
        .create_tenant(CreateTenant {
            name: "Deleted Tenant".to_string(),
            subdomain: "deleted".to_string(),
            base_currency: "TRY".to_string(),
            supported_currencies: vec![],
        })
        .await
        .unwrap();

    // Soft-delete the tenant
    state
        .admin
        .tenant_service
        .get_ref()
        .delete_tenant(tenant.id)
        .await
        .unwrap();

    let app = test::init_service(build_test_app(&state)).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(json!({
            "username": "ghost_user",
            "email": "ghost@test.com",
            "full_name": "Ghost",
            "password": "Password123!",
            "tenant_id": tenant.id
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    // Soft-deleted tenant: find_by_id returns None (filtered out) → 400
    assert!(
        resp.status() == actix_web::http::StatusCode::BAD_REQUEST,
        "Registration into deleted tenant must be rejected with 400, got {}",
        resp.status()
    );
}
