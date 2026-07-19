//! End-to-End Integration Tests for Tenant, Tenant Config, Users, Workflows,
//! IP Whitelist, LDAP, Goods Receipts, and Purchase Requests endpoints.
//!
//! Each test exercises the full request/response cycle for a module's
//! endpoint set, driving the complete business workflow from creation
//! through soft delete / restore / destroy (where applicable).
//!
//! Run with: cargo test --test integration e2e_tenant_users

use actix_web::{body::to_bytes, http::StatusCode, test, web, App};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::common::*;

use turerp::api::v1_ldap_configure;
use turerp::domain::ldap::model::{LdapSyncResult, LdapUser};
use turerp::domain::ldap::repository::{BoxLdapConfigRepository, InMemoryLdapConfigRepository};
use turerp::domain::ldap::service::{LdapClient, LdapSyncService};

// ============================================================================
// Helpers
// ============================================================================

/// Build a test app that additionally registers the LDAP v1 routes and
/// injects `state.ldap_service`. The shared `build_test_app` in
/// `tests/common/mod.rs` does not register LDAP routes (LDAP is optional and
/// tenant-specific), so LDAP e2e coverage needs this extended builder.
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
        .app_data(state.document.document_service.clone())
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
        .app_data(state.integration.earchive_service.clone())
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

/// Mock LDAP client used by the LDAP e2e test so sync/test endpoints do not
/// depend on a live directory server.
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

/// Build an AppState whose `ldap_service` is backed by a mock LDAP client
/// returning the supplied users, so the sync endpoint succeeds without a
/// real directory server.
async fn create_app_state_with_mock_ldap(users: Vec<LdapUser>) -> turerp::app::AppState {
    let mut state = create_test_app_state().await;

    let ldap_repo = Arc::new(InMemoryLdapConfigRepository::new()) as BoxLdapConfigRepository;
    let user_service = Arc::new(state.auth.user_service.get_ref().clone());

    let ldap_service = LdapSyncService::new(ldap_repo, user_service, test_key())
        .with_client(Arc::new(MockLdapClient { users }));

    state.ldap_service = web::Data::new(ldap_service);
    state
}

/// Helper to read a JSON response body.
async fn read_json(
    resp: actix_web::dev::ServiceResponse<actix_web::body::EitherBody<actix_web::body::BoxBody>>,
) -> Value {
    let body = to_bytes(resp.into_body()).await.unwrap();
    serde_json::from_slice(&body).unwrap()
}

// ============================================================================
// Tenant Workflow
// ============================================================================

/// Full tenant lifecycle: create → list → get → update → soft delete →
/// restore → deleted list → destroy.
#[actix_web::test]
async fn e2e_tenants_workflow() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let subdomain = format!("e2e-tenant-{}", uuid::Uuid::new_v4());

    // Create
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/tenants", &token)
        .set_json(json!({
            "name": "E2E Tenant",
            "subdomain": subdomain,
            "base_currency": "TRY",
            "supported_currencies": ["TRY", "USD"]
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "tenant create");
    let json = read_json(resp).await;
    assert!(json["id"].is_number());
    let id = json["id"].as_i64().unwrap();
    assert_eq!(json["name"], "E2E Tenant");
    assert_eq!(json["subdomain"], subdomain);

    // List
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/tenants?page=1&per_page=50",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "tenant list");
    let json = read_json(resp).await;
    assert!(json["items"].is_array());

    // Get
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/tenants/{}", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "tenant get");
    let json = read_json(resp).await;
    assert_eq!(json["id"], id);
    assert_eq!(json["name"], "E2E Tenant");

    // Update
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/tenants/{}", id),
        &token,
    )
    .set_json(json!({ "name": "E2E Tenant Updated", "is_active": false }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "tenant update");
    let json = read_json(resp).await;
    assert_eq!(json["name"], "E2E Tenant Updated");
    assert_eq!(json["is_active"], false);

    // Soft delete
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/tenants/{}", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "tenant soft delete");

    // Verify deleted (GET returns 404)
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/tenants/{}", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "tenant get after delete"
    );

    // Restore
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/tenants/{}/restore", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "tenant restore");
    let json = read_json(resp).await;
    assert_eq!(json["id"], id);

    // Verify restored
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/tenants/{}", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "tenant get after restore");

    // Soft delete again, then list deleted, then destroy
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/tenants/{}", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "tenant second soft delete");

    // Deleted list
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/tenants/deleted",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "tenant deleted list");
    let json = read_json(resp).await;
    let items = json.as_array().expect("deleted list is array");
    assert!(
        items.iter().any(|t| t["id"].as_i64() == Some(id)),
        "deleted list contains the tenant"
    );

    // Destroy
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/tenants/{}/destroy", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "tenant destroy");

    // Verify not restorable
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/tenants/{}/restore", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "tenant restore after destroy"
    );
}

// ============================================================================
// Tenant Config Workflow
// ============================================================================

/// Full tenant config lifecycle: create → list → get by key → update → delete.
#[actix_web::test]
async fn e2e_tenant_configs_workflow() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let key = format!("e2e.config.{}", uuid::Uuid::new_v4());

    // Create
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/tenant-configs",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "key": key,
        "value": "initial",
        "is_encrypted": false
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "config create");
    let json = read_json(resp).await;
    assert_eq!(json["key"], key);
    assert_eq!(json["value"], "initial");
    let config_id = json["id"].as_i64().unwrap();

    // List
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/tenant-configs",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "config list");
    let json = read_json(resp).await;
    assert!(json.is_array(), "config list is array");

    // Get by key
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/tenant-configs/{}", key),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "config get by key");
    let json = read_json(resp).await;
    assert_eq!(json["key"], key);
    assert_eq!(json["value"], "initial");

    // Update (by id)
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/tenant-configs/{}", config_id),
        &token,
    )
    .set_json(json!({ "value": "updated", "is_encrypted": false }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "config update");
    let json = read_json(resp).await;
    assert_eq!(json["value"], "updated");

    // Delete
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/tenant-configs/{}", config_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "config delete");

    // Verify deleted
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/tenant-configs/{}", config_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "config get after delete"
    );
}

// ============================================================================
// Users Workflow
// ============================================================================

/// Full user lifecycle: create → list → get → update → soft delete →
/// restore → deleted list → destroy.
#[actix_web::test]
async fn e2e_users_workflow() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let username = format!("e2e_user_{}", uuid::Uuid::new_v4());

    // Create
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/users", &token)
        .set_json(json!({
            "username": username,
            "email": format!("{}@test.com", username),
            "full_name": "E2E User",
            "password": "Password123!",
            "tenant_id": 1,
            "role": "user"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "user create");
    let json = read_json(resp).await;
    assert!(json["id"].is_number());
    let id = json["id"].as_i64().unwrap();
    assert_eq!(json["username"], username);

    // List
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/users?page=1&per_page=50",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "user list");
    let json = read_json(resp).await;
    assert!(json["items"].is_array());

    // Get
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/users/{}", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "user get");
    let json = read_json(resp).await;
    assert_eq!(json["id"], id);
    assert_eq!(json["username"], username);

    // Update
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/users/{}", id),
        &token,
    )
    .set_json(json!({ "full_name": "E2E User Updated", "is_active": false }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "user update");
    let json = read_json(resp).await;
    assert_eq!(json["full_name"], "E2E User Updated");
    assert_eq!(json["is_active"], false);

    // Soft delete
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/users/{}", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "user soft delete");

    // Verify deleted
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/users/{}", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "user get after delete"
    );

    // Restore
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/users/{}/restore", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "user restore");
    let json = read_json(resp).await;
    assert_eq!(json["id"], id);

    // Verify restored
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/users/{}", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "user get after restore");

    // Soft delete again, then list deleted, then destroy
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/users/{}", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "user second soft delete");

    // Deleted list
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/users/deleted",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "user deleted list");
    let json = read_json(resp).await;
    let items = json.as_array().expect("deleted list is array");
    assert!(
        items.iter().any(|u| u["id"].as_i64() == Some(id)),
        "deleted list contains the user"
    );

    // Destroy
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/users/{}/destroy", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "user destroy");

    // Verify not restorable
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/users/{}/restore", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "user restore after destroy"
    );
}

// ============================================================================
// Workflows — Templates
// ============================================================================

/// Workflow template lifecycle: create template → list templates → get.
#[actix_web::test]
async fn e2e_workflows_templates_workflow() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let name = format!("E2E Template {}", uuid::Uuid::new_v4());

    // Create template
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/workflows/templates",
        &token,
    )
    .set_json(json!({
        "name": name,
        "description": "E2E 2-step approval",
        "entity_type": "purchase_order",
        "config_json": {
            "steps": [
                {"step_number": 1, "step_name": "Manager Review", "approver_role": "manager"},
                {"step_number": 2, "step_name": "Admin Approval", "approver_role": "admin"}
            ]
        }
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "template create");
    let json = read_json(resp).await;
    assert!(json["id"].is_i64());
    let template_id = json["id"].as_i64().unwrap();
    assert_eq!(json["name"], name);
    assert_eq!(json["entity_type"], "purchase_order");

    // List templates
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/workflows/templates",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "template list");
    let json = read_json(resp).await;
    let templates = json.as_array().expect("templates is array");
    assert!(
        templates
            .iter()
            .any(|t| t["id"].as_i64() == Some(template_id)),
        "template list contains created template"
    );

    // Get the created template from the list (templates endpoint has no
    // single-resource GET, so we verify via the list).
    let created = templates
        .iter()
        .find(|t| t["id"].as_i64() == Some(template_id))
        .expect("created template present in list");
    assert_eq!(created["name"], name);
}

// ============================================================================
// Workflows — Instances
// ============================================================================

/// Workflow instance lifecycle: create template → create instance → get →
/// pending → approve/reject → resubmit → audit.
#[actix_web::test]
async fn e2e_workflows_instances_workflow() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    // Create template
    let template_id = create_workflow_template!(&app, &token);

    // Create instance
    let entity_id: i64 = 9001;
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/workflows/instances",
        &token,
    )
    .set_json(json!({
        "template_id": template_id,
        "entity_id": entity_id,
        "entity_type": "purchase_order"
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "instance create");
    let json = read_json(resp).await;
    assert!(json["id"].is_i64());
    let instance_id = json["id"].as_i64().unwrap();
    assert_eq!(json["template_id"], template_id);
    assert_eq!(json["entity_id"], entity_id);
    assert_eq!(json["status"], "pending");
    assert_eq!(json["current_step"], 1);

    // Get instance
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/workflows/instances/{}", instance_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "instance get");
    let json = read_json(resp).await;
    assert_eq!(json["id"], instance_id);
    assert!(json["steps"].is_array());
    assert_eq!(json["steps"].as_array().unwrap().len(), 2);

    // Pending approvals
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/workflows/pending",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "pending list");
    let json = read_json(resp).await;
    assert!(json.is_array(), "pending is array");

    // Approve step 1
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/workflows/instances/{}/approve", instance_id),
        &token,
    )
    .set_json(json!({"comment": "E2E step 1 approved"}))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "approve step 1");
    let json = read_json(resp).await;
    assert_eq!(json["status"], "pending");
    assert_eq!(json["current_step"], 2);

    // Reject step 2
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/workflows/instances/{}/reject", instance_id),
        &token,
    )
    .set_json(json!({"comment": "E2E step 2 rejected"}))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "reject step 2");
    let json = read_json(resp).await;
    assert_eq!(json["status"], "rejected");

    // Resubmit
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/workflows/instances/{}/resubmit", instance_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "resubmit");
    let json = read_json(resp).await;
    assert_eq!(json["status"], "pending");
    assert_eq!(json["current_step"], 1);

    // Audit trail
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/workflows/instances/{}/audit", instance_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "audit trail");
    let json = read_json(resp).await;
    let logs = json.as_array().expect("audit is array");
    assert!(!logs.is_empty(), "audit log non-empty");
}

// ============================================================================
// IP Whitelist Workflow
// ============================================================================

/// Full IP whitelist lifecycle: create → list → get → update → delete.
#[actix_web::test]
async fn e2e_ip_whitelist_workflow() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let suffix = uuid::Uuid::new_v4().as_u128() % 200;
    let ip = format!("10.20.{}.{}", suffix / 10, suffix % 10 + 1);

    // Create
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/ip-whitelist",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "ip_address": ip,
        "description": "E2E office network",
        "is_active": true
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "ip whitelist create");
    let json = read_json(resp).await;
    assert_eq!(json["ip_address"], ip);
    assert_eq!(json["is_active"], true);
    let id = json["id"].as_i64().unwrap();

    // List
    let req =
        auth_request(actix_web::http::Method::GET, "/api/v1/ip-whitelist", &token).to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "ip whitelist list");
    let json = read_json(resp).await;
    let items = json.as_array().expect("ip whitelist list is array");
    assert!(
        items.iter().any(|i| i["id"].as_i64() == Some(id)),
        "ip whitelist list contains entry"
    );

    // Get
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/ip-whitelist/{}", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "ip whitelist get");
    let json = read_json(resp).await;
    assert_eq!(json["id"], id);
    assert_eq!(json["ip_address"], ip);

    // Update
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/ip-whitelist/{}", id),
        &token,
    )
    .set_json(json!({
        "ip_address": ip,
        "description": "E2E updated description",
        "is_active": false
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "ip whitelist update");
    let json = read_json(resp).await;
    assert_eq!(json["description"], "E2E updated description");
    assert_eq!(json["is_active"], false);

    // Delete
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/ip-whitelist/{}", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT, "ip whitelist delete");

    // Verify deleted
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/ip-whitelist/{}", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "ip whitelist get after delete"
    );
}

// ============================================================================
// LDAP Config Workflow
// ============================================================================

/// Full LDAP config lifecycle: create config → get → update → test → sync →
/// delete. Uses a mock LDAP client so sync/test do not require a live
/// directory server.
#[actix_web::test]
async fn e2e_ldap_config_workflow() {
    let mock_users = vec![
        LdapUser {
            dn: "cn=e2e_john,dc=example,dc=com".to_string(),
            username: format!("e2e_john_{}", uuid::Uuid::new_v4()),
            email: "e2e_john@example.com".to_string(),
            full_name: "E2E John Doe".to_string(),
            groups: vec!["users".to_string()],
        },
        LdapUser {
            dn: "cn=e2e_jane,dc=example,dc=com".to_string(),
            username: format!("e2e_jane_{}", uuid::Uuid::new_v4()),
            email: "e2e_jane@example.com".to_string(),
            full_name: "E2E Jane Doe".to_string(),
            groups: vec!["users".to_string()],
        },
    ];

    let state = create_app_state_with_mock_ldap(mock_users).await;
    let app = test::init_service(build_test_app_with_ldap(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    // Create config
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/ldap/config", &token)
        .set_json(json!({
            "ldap_url": "ldap://localhost:389",
            "bind_dn": "cn=admin,dc=example,dc=com",
            "bind_password": "secret123",
            "base_dn": "dc=example,dc=com",
            "user_filter": "(objectClass=person)"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "ldap config create");
    let json = read_json(resp).await;
    assert_eq!(json["tenant_id"], 1);
    assert_eq!(json["ldap_url"], "ldap://localhost:389");
    assert_eq!(json["is_active"], true);

    // Get config
    let req =
        auth_request(actix_web::http::Method::GET, "/api/v1/ldap/config", &token).to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "ldap config get");
    let json = read_json(resp).await;
    assert_eq!(json["ldap_url"], "ldap://localhost:389");
    assert_eq!(json["bind_dn"], "cn=admin,dc=example,dc=com");

    // Update config
    let req = auth_request(actix_web::http::Method::PUT, "/api/v1/ldap/config", &token)
        .set_json(json!({
            "ldap_url": "ldaps://updated:636",
            "user_filter": "(objectClass=organizationalPerson)"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "ldap config update");
    let json = read_json(resp).await;
    assert_eq!(json["ldap_url"], "ldaps://updated:636");
    assert_eq!(json["user_filter"], "(objectClass=organizationalPerson)");

    // Test connection (explicit params — mock client always returns success=true)
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/ldap/test", &token)
        .set_json(json!({
            "ldap_url": "ldap://nonexistent:389",
            "bind_dn": "cn=admin,dc=example,dc=com",
            "bind_password": "secret123",
            "base_dn": "dc=example,dc=com"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "ldap test connection");
    let json = read_json(resp).await;
    assert_eq!(
        json["success"], true,
        "mock client test connection succeeds"
    );

    // Sync users (uses the stored active config + mock client)
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/ldap/sync", &token).to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "ldap sync");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let result: LdapSyncResult = serde_json::from_slice(&body).unwrap();
    assert_eq!(result.imported, 2, "two mock users imported");
    assert_eq!(result.errors, 0, "no sync errors");

    // Delete config
    let req = auth_request(
        actix_web::http::Method::DELETE,
        "/api/v1/ldap/config",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "ldap config delete");

    // Verify deleted
    let req =
        auth_request(actix_web::http::Method::GET, "/api/v1/ldap/config", &token).to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "ldap config get after delete"
    );
}

// ============================================================================
// Goods Receipts Workflow
// ============================================================================

/// Full goods receipt lifecycle: create purchase order → approve it →
/// create goods receipt → get → list by order → update status →
/// soft delete → restore → deleted list → destroy.
#[actix_web::test]
async fn e2e_goods_receipts_workflow() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, user_id) = register_admin(&state, 1).await;

    // Seed a cari (vendor) for the purchase order precheck.
    let cari_id = seed_cari!(&app, &token, user_id, 1);

    // Create a purchase order (starts in Draft).
    let now = chrono::Utc::now();
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/purchase-orders",
        &token,
    )
    .set_json(json!({
        "cari_id": cari_id,
        "order_date": now.to_rfc3339(),
        "tenant_id": 1,
        "lines": [{
            "description": "E2E goods receipt item",
            "quantity": "10.00",
            "unit_price": "50.00",
            "tax_rate": "18.00",
            "discount_rate": "0.00"
        }]
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "purchase order create");
    let json = read_json(resp).await;
    let order_id = json["id"].as_i64().unwrap();
    let order_line_id = json["lines"][0]["id"].as_i64().expect("order line has id");

    // Approve the purchase order so it is eligible for receiving.
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/purchase-orders/{}/status", order_id),
        &token,
    )
    .set_json(json!({ "status": "Approved" }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "purchase order approve");

    // Create goods receipt
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/goods-receipts",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "company_id": 1,
        "purchase_order_id": order_id,
        "receipt_date": now.to_rfc3339(),
        "notes": "E2E goods receipt",
        "lines": [{
            "order_line_id": order_line_id,
            "product_id": null,
            "quantity": "5.00",
            "condition": "Good",
            "notes": "E2E line"
        }]
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "goods receipt create");
    let json = read_json(resp).await;
    assert!(json["id"].is_number());
    let receipt_id = json["id"].as_i64().unwrap();
    assert_eq!(json["purchase_order_id"], order_id);
    assert_eq!(json["status"], "Pending");

    // Get goods receipt
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/goods-receipts/{}", receipt_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "goods receipt get");
    let json = read_json(resp).await;
    assert_eq!(json["id"], receipt_id);
    assert!(json["lines"].is_array());

    // List by order
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/goods-receipts/order/{}", order_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "goods receipts by order");
    let json = read_json(resp).await;
    let receipts = json.as_array().expect("receipts by order is array");
    assert!(
        receipts
            .iter()
            .any(|r| r["id"].as_i64() == Some(receipt_id)),
        "receipt list by order contains the receipt"
    );

    // Update status
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/goods-receipts/{}/status", receipt_id),
        &token,
    )
    .set_json(json!({ "status": "Completed" }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "goods receipt status update");
    let json = read_json(resp).await;
    assert_eq!(json["status"], "Completed");

    // Soft delete
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/goods-receipts/{}", receipt_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "goods receipt soft delete");

    // Verify deleted
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/goods-receipts/{}", receipt_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "goods receipt get after delete"
    );

    // Restore
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/goods-receipts/{}/restore", receipt_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "goods receipt restore");
    let json = read_json(resp).await;
    assert_eq!(json["id"], receipt_id);

    // Verify restored
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/goods-receipts/{}", receipt_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "goods receipt get after restore"
    );

    // Soft delete again, then list deleted, then destroy
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/goods-receipts/{}", receipt_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "goods receipt second soft delete"
    );

    // Deleted list
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/goods-receipts/deleted",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "goods receipts deleted list");
    let json = read_json(resp).await;
    let items = json.as_array().expect("deleted receipts is array");
    assert!(
        items.iter().any(|r| r["id"].as_i64() == Some(receipt_id)),
        "deleted list contains the receipt"
    );

    // Destroy
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/goods-receipts/{}/destroy", receipt_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "goods receipt destroy"
    );

    // Verify not restorable
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/goods-receipts/{}/restore", receipt_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "goods receipt restore after destroy"
    );
}

// ============================================================================
// Purchase Requests Workflow
// ============================================================================

/// Full purchase request lifecycle: create → submit → approve/reject →
/// list → get → update → soft delete → restore → deleted list → destroy.
#[actix_web::test]
async fn e2e_purchase_requests_workflow() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    // Create purchase request (starts in Draft).
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/purchase-requests",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "company_id": 1,
        "requested_by": 1,
        "department": "E2E Dept",
        "priority": "High",
        "reason": "E2E purchase request",
        "lines": [{
            "product_id": null,
            "description": "E2E request line",
            "quantity": "3.00",
            "notes": "E2E notes"
        }]
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "purchase request create"
    );
    let json = read_json(resp).await;
    assert!(json["id"].is_number());
    let pr_id = json["id"].as_i64().unwrap();
    assert_eq!(json["status"], "Draft");
    assert_eq!(json["priority"], "High");

    // Submit for approval (Draft → PendingApproval)
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/purchase-requests/{}/submit", pr_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "purchase request submit");
    let json = read_json(resp).await;
    assert_eq!(json["status"], "PendingApproval");

    // Reject (PendingApproval → Rejected)
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/purchase-requests/{}/reject", pr_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "purchase request reject");
    let json = read_json(resp).await;
    assert_eq!(json["status"], "Rejected");

    // Update (Rejected → Draft via status update, plus field changes)
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/purchase-requests/{}", pr_id),
        &token,
    )
    .set_json(json!({
        "department": "E2E Updated Dept",
        "priority": "Medium",
        "reason": "E2E updated reason",
        "status": "Draft"
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "purchase request update");
    let json = read_json(resp).await;
    assert_eq!(json["status"], "Draft");
    assert_eq!(json["priority"], "Medium");

    // Re-submit and approve this time
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/purchase-requests/{}/submit", pr_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "purchase request re-submit");
    let json = read_json(resp).await;
    assert_eq!(json["status"], "PendingApproval");

    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/purchase-requests/{}/approve", pr_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "purchase request approve");
    let json = read_json(resp).await;
    assert_eq!(json["status"], "Approved");

    // List
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/purchase-requests?page=1&per_page=50",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "purchase request list");
    let json = read_json(resp).await;
    assert!(json["items"].is_array());

    // Get
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/purchase-requests/{}", pr_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "purchase request get");
    let json = read_json(resp).await;
    assert_eq!(json["id"], pr_id);
    assert_eq!(json["status"], "Approved");

    // Soft delete
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/purchase-requests/{}", pr_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "purchase request soft delete"
    );

    // Verify deleted
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/purchase-requests/{}", pr_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "purchase request get after delete"
    );

    // Restore (PUT for purchase requests)
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/purchase-requests/{}/restore", pr_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "purchase request restore");
    let json = read_json(resp).await;
    assert_eq!(json["id"], pr_id);

    // Verify restored
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/purchase-requests/{}", pr_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "purchase request get after restore"
    );

    // Soft delete again, then list deleted, then destroy
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/purchase-requests/{}", pr_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "purchase request second soft delete"
    );

    // Deleted list
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/purchase-requests/deleted",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "purchase request deleted list"
    );
    let json = read_json(resp).await;
    let items = json.as_array().expect("deleted requests is array");
    assert!(
        items.iter().any(|r| r["id"].as_i64() == Some(pr_id)),
        "deleted list contains the request"
    );

    // Destroy
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/purchase-requests/{}/destroy", pr_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "purchase request destroy"
    );

    // Verify not restorable
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/purchase-requests/{}/restore", pr_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "purchase request restore after destroy"
    );
}
