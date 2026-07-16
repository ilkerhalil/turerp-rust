//! LDAP Configuration CRUD Integration Tests

use actix_web::{body::to_bytes, http::StatusCode, test, web, App};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

use crate::common::*;

use turerp::api::v1_ldap_configure;
use turerp::domain::ldap::model::{LdapSyncResult, LdapUser};
use turerp::domain::ldap::repository::{BoxLdapConfigRepository, InMemoryLdapConfigRepository};
use turerp::domain::ldap::service::{LdapClient, LdapSyncService};
use turerp::domain::user::model::CreateUser;

fn build_test_app_with_ldap(
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
        .app_data(state.commerce.cari_service.clone())
        .app_data(state.commerce.stock_service.clone())
        .app_data(state.commerce.invoice_service.clone())
        .app_data(state.commerce.sales_service.clone())
        .app_data(state.hr.hr_service.clone())
        .app_data(state.finance.accounting_service.clone())
        .app_data(state.project.project_service.clone())
        .app_data(state.project.manufacturing_service.clone())
        .app_data(state.project.crm_service.clone())
        .app_data(state.admin.tenant_service.clone())
        .app_data(state.admin.tenant_config_service.clone())
        .app_data(state.i18n.clone())
        .app_data(state.assets_service.clone())
        .app_data(state.feature_service.clone())
        .app_data(state.commerce.product_service.clone())
        .app_data(state.commerce.purchase_service.clone())
        .app_data(state.chart_of_accounts_service.clone())
        .app_data(state.custom_field_service.clone())
        .app_data(state.finance.tax_service.clone())
        .app_data(state.integration.customer_portal_service.clone())
        .app_data(state.integration.webhook_service.clone())
        .app_data(state.infra.search_service.clone())
        .app_data(state.infra.report_engine.clone())
        .app_data(state.infra.job_scheduler.clone())
        .app_data(state.infra.notification_service.clone())
        .app_data(state.analytics.audit_service.clone())
        .app_data(state.finance.bank_service.clone())
        .app_data(state.finance.cost_center_service.clone())
        .app_data(state.document.dashboard_service.clone())
        .app_data(state.document.file_storage.clone())
        .app_data(state.project.qc_service.clone())
        .app_data(state.admin.settings_service.clone())
        .app_data(state.admin.api_key_service.clone())
        .app_data(state.admin.ip_whitelist_service.clone())
        .app_data(state.commerce.barcode_service.clone())
        .app_data(state.analytics.subscription_service.clone())
        .app_data(state.integration.workflow_service.clone())
        .app_data(state.finance.currency_service.clone())
        .app_data(state.infra.import_service.clone())
        .app_data(state.commerce.inter_company_service.clone())
        .app_data(state.integration.efatura_service.clone())
        .app_data(state.integration.edefter_service.clone())
        .app_data(state.commerce.company_service.clone())
        .app_data(state.ldap_service.clone())
        .service(
            web::scope("/api")
                .configure(configure_all_routes)
                .configure(configure_v1_routes)
                .configure(v1_ldap_configure),
        )
}

fn test_key() -> [u8; 32] {
    [
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
        26, 27, 28, 29, 30, 31, 32,
    ]
}

struct MockLdapClient {
    users: Vec<LdapUser>,
}

#[async_trait]
impl LdapClient for MockLdapClient {
    async fn test_connection(
        &self,
        _config: &turerp::domain::ldap::model::LdapConfig,
        _encryption_key: &[u8],
    ) -> Result<bool, turerp::error::ApiError> {
        Ok(true)
    }

    async fn search_users(
        &self,
        _config: &turerp::domain::ldap::model::LdapConfig,
        _encryption_key: &[u8],
    ) -> Result<Vec<LdapUser>, turerp::error::ApiError> {
        Ok(self.users.clone())
    }
}

async fn create_app_state_with_mock_ldap(users: Vec<LdapUser>) -> turerp::app::AppState {
    let mut state = create_test_app_state().await;

    let ldap_repo = Arc::new(InMemoryLdapConfigRepository::new()) as BoxLdapConfigRepository;
    let user_service = Arc::new(state.auth.user_service.get_ref().clone());

    let ldap_service = LdapSyncService::new(ldap_repo, user_service, test_key())
        .with_client(Arc::new(MockLdapClient { users }));

    state.ldap_service = web::Data::new(ldap_service);
    state
}

fn ldap_create_payload() -> serde_json::Value {
    json!({
        "ldap_url": "ldap://localhost:389",
        "bind_dn": "cn=admin,dc=example,dc=com",
        "bind_password": "secret123",
        "base_dn": "dc=example,dc=com",
        "user_filter": "(objectClass=person)"
    })
}

// ============================================================================
// Create
// ============================================================================

#[actix_web::test]
async fn test_create_ldap_config_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_ldap(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(actix_web::http::Method::POST, "/api/v1/ldap/config", &token)
        .set_json(ldap_create_payload())
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["tenant_id"], 1);
    assert_eq!(json["ldap_url"], "ldap://localhost:389");
    assert_eq!(json["is_active"], true);
}

#[actix_web::test]
async fn test_create_ldap_config_conflict() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_ldap(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(actix_web::http::Method::POST, "/api/v1/ldap/config", &token)
        .set_json(ldap_create_payload())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let req2 = auth_request(actix_web::http::Method::POST, "/api/v1/ldap/config", &token)
        .set_json(ldap_create_payload())
        .to_request();
    let resp2 = test::call_service(&app, req2).await;
    assert_eq!(resp2.status(), StatusCode::CONFLICT);
}

// ============================================================================
// Read
// ============================================================================

#[actix_web::test]
async fn test_get_ldap_config_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_ldap(&state)).await;
    let (admin_token, _admin_id) = register_admin(&state, 1).await;

    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/ldap/config",
        &admin_token,
    )
    .set_json(ldap_create_payload())
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    assert_eq!(create_resp.status(), StatusCode::CREATED);

    let (user_token, _user_id) = register_user!(&app, 1);
    let get_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/ldap/config",
        &user_token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);

    let body = to_bytes(get_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["tenant_id"], 1);
    assert_eq!(json["ldap_url"], "ldap://localhost:389");
}

#[actix_web::test]
async fn test_get_ldap_config_not_found() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_ldap(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req =
        auth_request(actix_web::http::Method::GET, "/api/v1/ldap/config", &token).to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// Update
// ============================================================================

#[actix_web::test]
async fn test_update_ldap_config_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_ldap(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/ldap/config", &token)
        .set_json(ldap_create_payload())
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    assert_eq!(create_resp.status(), StatusCode::CREATED);

    let update_req = auth_request(actix_web::http::Method::PUT, "/api/v1/ldap/config", &token)
        .set_json(json!({
            "ldap_url": "ldaps://newhost:636",
            "user_filter": "(objectClass=organizationalPerson)"
        }))
        .to_request();
    let update_resp = test::call_service(&app, update_req).await;
    assert_eq!(update_resp.status(), StatusCode::OK);

    let body = to_bytes(update_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["ldap_url"], "ldaps://newhost:636");
    assert_eq!(json["user_filter"], "(objectClass=organizationalPerson)");
}

// ============================================================================
// Delete
// ============================================================================

#[actix_web::test]
async fn test_delete_ldap_config_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_ldap(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/ldap/config", &token)
        .set_json(ldap_create_payload())
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    assert_eq!(create_resp.status(), StatusCode::CREATED);

    let delete_req = auth_request(
        actix_web::http::Method::DELETE,
        "/api/v1/ldap/config",
        &token,
    )
    .to_request();
    let delete_resp = test::call_service(&app, delete_req).await;
    assert_eq!(delete_resp.status(), StatusCode::OK);

    let get_req =
        auth_request(actix_web::http::Method::GET, "/api/v1/ldap/config", &token).to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_delete_ldap_config_not_found() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_ldap(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(
        actix_web::http::Method::DELETE,
        "/api/v1/ldap/config",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// Test Connection
// ============================================================================

#[actix_web::test]
async fn test_test_ldap_connection_explicit_params() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_ldap(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(actix_web::http::Method::POST, "/api/v1/ldap/test", &token)
        .set_json(json!({
            "ldap_url": "ldap://nonexistent:389",
            "bind_dn": "cn=admin,dc=example,dc=com",
            "bind_password": "secret123",
            "base_dn": "dc=example,dc=com"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    // No real LDAP server, so connection test returns success=false but HTTP 200
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["success"], false);
}

// ============================================================================
// Sync Users
// ============================================================================

#[actix_web::test]
async fn test_sync_users_no_config() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_ldap(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(actix_web::http::Method::POST, "/api/v1/ldap/sync", &token).to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_sync_users_inactive_config() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_ldap(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/ldap/config", &token)
        .set_json(ldap_create_payload())
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    assert_eq!(create_resp.status(), StatusCode::CREATED);

    let update_req = auth_request(actix_web::http::Method::PUT, "/api/v1/ldap/config", &token)
        .set_json(json!({ "is_active": false }))
        .to_request();
    let update_resp = test::call_service(&app, update_req).await;
    assert_eq!(update_resp.status(), StatusCode::OK);

    let sync_req =
        auth_request(actix_web::http::Method::POST, "/api/v1/ldap/sync", &token).to_request();
    let sync_resp = test::call_service(&app, sync_req).await;
    assert_eq!(sync_resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn test_sync_users_with_mock() {
    let mock_users = vec![
        LdapUser {
            dn: "cn=john,dc=example,dc=com".to_string(),
            username: "john".to_string(),
            email: "john@example.com".to_string(),
            full_name: "John Doe".to_string(),
            groups: vec!["users".to_string()],
        },
        LdapUser {
            dn: "cn=jane,dc=example,dc=com".to_string(),
            username: "jane".to_string(),
            email: "jane@example.com".to_string(),
            full_name: "Jane Doe".to_string(),
            groups: vec!["users".to_string()],
        },
    ];

    let state = create_app_state_with_mock_ldap(mock_users).await;
    let app = test::init_service(build_test_app_with_ldap(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/ldap/config", &token)
        .set_json(ldap_create_payload())
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    assert_eq!(create_resp.status(), StatusCode::CREATED);

    let sync_req =
        auth_request(actix_web::http::Method::POST, "/api/v1/ldap/sync", &token).to_request();
    let sync_resp = test::call_service(&app, sync_req).await;
    assert_eq!(sync_resp.status(), StatusCode::OK);

    let body = to_bytes(sync_resp.into_body()).await.unwrap();
    let result: LdapSyncResult = serde_json::from_slice(&body).unwrap();
    assert_eq!(result.imported, 2);
    assert_eq!(result.updated, 0);
    assert_eq!(result.skipped, 0);
    assert_eq!(result.errors, 0);
}

#[actix_web::test]
async fn test_sync_users_update_existing() {
    let mock_users = vec![LdapUser {
        dn: "cn=john,dc=example,dc=com".to_string(),
        username: "john".to_string(),
        email: "new@example.com".to_string(),
        full_name: "New Name".to_string(),
        groups: vec!["users".to_string()],
    }];

    let state = create_app_state_with_mock_ldap(mock_users).await;
    let app = test::init_service(build_test_app_with_ldap(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    // Pre-create a user that will be updated
    state
        .auth
        .user_service
        .get_ref()
        .create_user(CreateUser {
            username: "john".to_string(),
            email: "old@example.com".to_string(),
            full_name: "Old Name".to_string(),
            password: "ValidPassword123!".to_string(),
            tenant_id: 1,
            role: None,
        })
        .await
        .unwrap();

    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/ldap/config", &token)
        .set_json(ldap_create_payload())
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    assert_eq!(create_resp.status(), StatusCode::CREATED);

    let sync_req =
        auth_request(actix_web::http::Method::POST, "/api/v1/ldap/sync", &token).to_request();
    let sync_resp = test::call_service(&app, sync_req).await;
    assert_eq!(sync_resp.status(), StatusCode::OK);

    let body = to_bytes(sync_resp.into_body()).await.unwrap();
    let result: LdapSyncResult = serde_json::from_slice(&body).unwrap();
    assert_eq!(result.imported, 0);
    assert_eq!(result.updated, 1);
    assert_eq!(result.skipped, 0);
    assert_eq!(result.errors, 0);
}

#[actix_web::test]
async fn test_sync_users_skip_unchanged() {
    let mock_users = vec![LdapUser {
        dn: "cn=john,dc=example,dc=com".to_string(),
        username: "john".to_string(),
        email: "john@example.com".to_string(),
        full_name: "John Doe".to_string(),
        groups: vec!["users".to_string()],
    }];

    let state = create_app_state_with_mock_ldap(mock_users).await;
    let app = test::init_service(build_test_app_with_ldap(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    state
        .auth
        .user_service
        .get_ref()
        .create_user(CreateUser {
            username: "john".to_string(),
            email: "john@example.com".to_string(),
            full_name: "John Doe".to_string(),
            password: "ValidPassword123!".to_string(),
            tenant_id: 1,
            role: None,
        })
        .await
        .unwrap();

    let create_req = auth_request(actix_web::http::Method::POST, "/api/v1/ldap/config", &token)
        .set_json(ldap_create_payload())
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    assert_eq!(create_resp.status(), StatusCode::CREATED);

    let sync_req =
        auth_request(actix_web::http::Method::POST, "/api/v1/ldap/sync", &token).to_request();
    let sync_resp = test::call_service(&app, sync_req).await;
    assert_eq!(sync_resp.status(), StatusCode::OK);

    let body = to_bytes(sync_resp.into_body()).await.unwrap();
    let result: LdapSyncResult = serde_json::from_slice(&body).unwrap();
    assert_eq!(result.imported, 0);
    assert_eq!(result.updated, 0);
    assert_eq!(result.skipped, 1);
    assert_eq!(result.errors, 0);
}

// ============================================================================
// Authorization
// ============================================================================

#[actix_web::test]
async fn test_unauthorized_access() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_ldap(&state)).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/ldap/config")
        .set_json(ldap_create_payload())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_normal_user_forbidden() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_ldap(&state)).await;
    let (token, _user_id) = register_user!(&app, 1);

    let req = auth_request(actix_web::http::Method::POST, "/api/v1/ldap/config", &token)
        .set_json(ldap_create_payload())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}
