//! Common test utilities for integration tests

use actix_web::{test, web, App};

/// Shared lock serializing tests that mutate process-global environment
/// variables (`std::env::set_var`/`remove_var`). When integration tests ran as
/// separate binaries each had its own process, so env mutations were isolated.
/// Now that all integration tests share one binary (`tests/integration.rs`),
/// env-mutating tests (`cors_env_loading`, `db_pool_max_conns_env`,
/// `rate_limit_env_loading`) must hold this lock to avoid racing on shared env.
pub static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

use turerp::api::{
    auth_configure, users_configure, v1_accounting_configure, v1_api_keys_configure,
    v1_assets_configure, v1_audit_configure, v1_auth_configure, v1_bank_configure,
    v1_barcode_configure, v1_cari_configure, v1_chart_of_accounts_configure,
    v1_companies_configure, v1_cost_centers_configure, v1_crm_configure, v1_currency_configure,
    v1_custom_fields_configure, v1_customer_portal_configure, v1_dashboard_configure,
    v1_documents_configure, v1_earchive_configure, v1_edefter_configure, v1_efatura_configure,
    v1_feature_flags_configure, v1_files_configure, v1_goods_receipts_configure, v1_hr_configure,
    v1_import_configure, v1_invoice_configure, v1_ip_whitelist_configure, v1_jobs_configure,
    v1_manufacturing_configure, v1_mfa_configure, v1_notifications_configure,
    v1_observability_configure, v1_product_variants_configure, v1_project_configure,
    v1_purchase_orders_configure, v1_purchase_requests_configure, v1_push_notifications_configure,
    v1_rate_limits_configure, v1_reports_configure, v1_resilience_configure, v1_sales_configure,
    v1_search_configure, v1_settings_configure, v1_stock_configure, v1_subscriptions_configure,
    v1_tax_configure, v1_tenant_configure, v1_users_configure, v1_webhooks_configure,
    v1_workflows_configure,
};
use turerp::app::create_app_state_in_memory;
use turerp::config::Config;
use turerp::middleware::JwtAuthMiddleware;
use turerp::utils::jwt::JwtService;

pub fn configure_all_routes(cfg: &mut web::ServiceConfig) {
    auth_configure(cfg);
    users_configure(cfg);
}

pub fn configure_v1_routes(cfg: &mut web::ServiceConfig) {
    cfg.configure(v1_auth_configure)
        .configure(v1_bank_configure)
        .configure(v1_cari_configure)
        .configure(v1_chart_of_accounts_configure)
        .configure(v1_companies_configure)
        .configure(v1_cost_centers_configure)
        .configure(v1_crm_configure)
        .configure(v1_currency_configure)
        .configure(v1_customer_portal_configure)
        .configure(v1_custom_fields_configure)
        .configure(v1_dashboard_configure)
        .configure(v1_documents_configure)
        .configure(v1_edefter_configure)
        .configure(v1_earchive_configure)
        .configure(v1_efatura_configure)
        .configure(v1_feature_flags_configure)
        .configure(v1_files_configure)
        .configure(v1_goods_receipts_configure)
        .configure(v1_stock_configure)
        .configure(v1_invoice_configure)
        .configure(v1_sales_configure)
        .configure(v1_hr_configure)
        .configure(v1_import_configure)
        .configure(v1_accounting_configure)
        .configure(v1_project_configure)
        .configure(v1_manufacturing_configure)
        .configure(v1_mfa_configure)
        .configure(v1_users_configure)
        .configure(v1_tenant_configure)
        .configure(v1_assets_configure)
        .configure(v1_product_variants_configure)
        .configure(v1_purchase_orders_configure)
        .configure(v1_purchase_requests_configure)
        .configure(v1_tax_configure)
        .configure(v1_webhooks_configure)
        .configure(v1_search_configure)
        .configure(v1_reports_configure)
        .configure(v1_jobs_configure)
        .configure(v1_push_notifications_configure)
        .configure(v1_notifications_configure)
        .configure(v1_observability_configure)
        .configure(v1_rate_limits_configure)
        .configure(v1_resilience_configure)
        .configure(v1_settings_configure)
        .configure(v1_subscriptions_configure)
        .configure(v1_workflows_configure)
        .configure(v1_api_keys_configure)
        .configure(v1_ip_whitelist_configure)
        .configure(v1_barcode_configure)
        .configure(v1_audit_configure);
}

pub async fn create_test_app_state() -> turerp::app::AppState {
    let config = Config {
        encryption_key: "YWJjZGVmZ2hpamtsbW5vcHFyc3R1dnd4eXoxMjM0NTY=".to_string(),
        ..Config::default()
    };
    create_app_state_in_memory(&config)
        .await
        .expect("app state creation failed")
}

pub fn create_test_jwt_service() -> JwtService {
    let config = Config::default();
    JwtService::new(
        config.jwt.secret.clone(),
        config.jwt.access_token_expiration,
        config.jwt.refresh_token_expiration,
    )
}

pub fn build_test_app(
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
        .service(
            web::scope("/api")
                .configure(configure_all_routes)
                .configure(configure_v1_routes),
        )
}

pub async fn register_admin(state: &turerp::app::AppState, tenant_id: i64) -> (String, i64) {
    let username = format!("admin_{}", uuid::Uuid::new_v4());
    let user = state
        .auth
        .user_service
        .get_ref()
        .create_user(turerp::CreateUser {
            username: username.clone(),
            email: format!("{}@test.com", username),
            full_name: "Admin User".to_string(),
            password: "Password123!".to_string(),
            tenant_id,
            role: Some(turerp::Role::Admin),
        })
        .await
        .unwrap();
    let tokens = state
        .auth
        .jwt_service
        .get_ref()
        .generate_tokens(
            user.id,
            user.tenant_id,
            user.username.clone(),
            turerp::Role::Admin,
        )
        .unwrap();
    (tokens.access_token, user.id)
}

/// Register a non-admin user in `tenant_id` and return its id (no token — the
/// caller only needs the user to exist as a valid FK referent, e.g. an
/// `inspector_id` / `assigned_to` / `responsible_person_id` precheck target).
/// The InMemory user repo auto-assigns ids from 1, so the first user in a fresh
/// app state is id 1; calling this after `register_admin` yields id 2.
pub async fn register_user(state: &turerp::app::AppState, tenant_id: i64) -> i64 {
    let username = format!("user_{}", uuid::Uuid::new_v4());
    let user = state
        .auth
        .user_service
        .get_ref()
        .create_user(turerp::CreateUser {
            username: username.clone(),
            email: format!("{}@test.com", username),
            full_name: "Test User".to_string(),
            password: "Password123!".to_string(),
            tenant_id,
            role: Some(turerp::Role::User),
        })
        .await
        .unwrap();
    user.id
}

/// Helper macro to register a normal (non-admin) user via the API
/// Usage: `let (token, user_id) = register_user!(&app, 1);`
#[macro_export]
macro_rules! register_user {
    ($app:expr, $tenant_id:expr) => {{
        let username = format!("user_{}", uuid::Uuid::new_v4());
        let req = test::TestRequest::post()
            .uri("/api/auth/register")
            .set_json(json!({
                "username": username,
                "email": format!("{}@test.com", username),
                "full_name": "Normal User",
                "password": "Password123!",
                "tenant_id": $tenant_id
            }))
            .to_request();
        let resp = test::call_service($app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED, "User registration failed");
        let body = to_bytes(resp.into_body()).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let access_token = json["tokens"]["access_token"].as_str().unwrap().to_string();
        let user_id = json["user"]["id"].as_i64().unwrap();
        (access_token, user_id)
    }};
}

/// Helper macro to create a workflow template for testing
#[macro_export]
macro_rules! create_workflow_template {
    ($app:expr, $token:expr) => {{
        let req = auth_request(
            actix_web::http::Method::POST,
            "/api/v1/workflows/templates",
            $token,
        )
        .set_json(json!({
            "name": "Purchase Order Approval",
            "description": "2-step approval workflow",
            "entity_type": "purchase_order",
            "config_json": {
                "steps": [
                    {"step_number": 1, "step_name": "Manager Review", "approver_role": "manager"},
                    {"step_number": 2, "step_name": "Admin Approval", "approver_role": "admin"}
                ]
            }
        }))
        .to_request();

        let resp = test::call_service($app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED, "Template creation failed");

        let body = to_bytes(resp.into_body()).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        json["id"].as_i64().unwrap()
    }};
}

/// Helper macro to create a cari (customer/vendor) and return its id, for
/// integration tests whose create-endpoint prechecks require an owned parent FK.
/// Mirrors the established inline pattern in `invoice_crud_test.rs`.
/// Usage: `let cari_id = seed_cari!(&app, &token, user_id, 1);`
#[macro_export]
macro_rules! seed_cari {
    ($app:expr, $token:expr, $user_id:expr, $tenant_id:expr) => {{
        let req = auth_request(
            actix_web::http::Method::POST,
            "/api/v1/caris",
            $token,
        )
        .set_json(json!({
            "code": format!("CARI-{}", uuid::Uuid::new_v4()),
            "name": "Test Cari",
            "cari_type": "customer",
            "created_by": $user_id,
            "tenant_id": $tenant_id,
        }))
        .to_request();
        let resp = test::call_service($app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED, "Cari creation failed");
        let body = to_bytes(resp.into_body()).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        json["id"].as_i64().unwrap()
    }};
}

/// Helper macro to create an invoice (seeding an owned cari first) and return
/// its id, for integration tests whose create-endpoint prechecks require an
/// owned parent `invoice_id` FK (e-Fatura / e-Archive). Mirrors the inline
/// invoice-creation pattern in `invoice_crud_test.rs`. The invoice create body
/// omits `company_id` (defaults to the legacy `1` sentinel, skipped by
/// `ensure_company_owned`) and line `product_id` (null → skipped), so only the
/// cari parent-FK precheck applies. `tenant_id` is auth-overwritten by the
/// handler. Usage: `let invoice_id = seed_invoice!(&app, &token, user_id, 1);`
#[macro_export]
macro_rules! seed_invoice {
    ($app:expr, $token:expr, $user_id:expr, $tenant_id:expr) => {{
        let cari_id = seed_cari!($app, $token, $user_id, $tenant_id);
        let req = auth_request(
            actix_web::http::Method::POST,
            "/api/v1/invoices",
            $token,
        )
        .set_json(json!({
            "invoice_type": "SalesInvoice",
            "cari_id": cari_id,
            "issue_date": "2024-01-01T00:00:00Z",
            "due_date": "2024-02-01T00:00:00Z",
            "currency": "TRY",
            "tenant_id": $tenant_id,
            "lines": [{
                "description": "Seed invoice line",
                "quantity": 1.0,
                "unit_price": 10.0,
                "tax_rate": 0.0,
                "discount_rate": 0.0
            }]
        }))
        .to_request();
        let resp = test::call_service($app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED, "Invoice creation failed");
        let body = to_bytes(resp.into_body()).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        json["id"].as_i64().unwrap()
    }};
}

/// Helper macro to create a product and return its id, for integration tests
/// whose create-endpoint prechecks require an owned parent `product_id` FK
/// (stock movements, work orders, BOMs, routings, inspections, NCRs). Mirrors
/// the inline `create_product!` in `inter_company_api_test.rs` but lives in the
/// shared common module and parameterizes the tenant. `CreateProduct` has no
/// `created_by` field, so none is sent.
/// Usage: `let product_id = seed_product!(&app, &token, 1);`
#[macro_export]
macro_rules! seed_product {
    ($app:expr, $token:expr, $tenant_id:expr) => {{
        let req = auth_request(
            actix_web::http::Method::POST,
            "/api/v1/products",
            $token,
        )
        .set_json(json!({
            "code": format!("PROD-{}", uuid::Uuid::new_v4()),
            "name": "Test Product",
            "purchase_price": 50.0,
            "sale_price": 100.0,
            "tax_rate": 18.0,
            "tenant_id": $tenant_id,
        }))
        .to_request();
        let resp = test::call_service($app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED, "Product creation failed");
        let body = to_bytes(resp.into_body()).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        json["id"].as_i64().unwrap()
    }};
}

/// Helper macro to create a manufacturing work order (referencing `product_id`)
/// and return its id, for integration tests whose create-endpoint prechecks
/// require an owned parent `work_order_id` FK (inspections). The work order's
/// own `product_id` precheck is satisfied by the caller-supplied (tenant-owned)
/// product id.
/// Usage: `let work_order_id = seed_work_order!(&app, &token, 1, product_id);`
#[macro_export]
macro_rules! seed_work_order {
    ($app:expr, $token:expr, $tenant_id:expr, $product_id:expr) => {{
        let req = auth_request(
            actix_web::http::Method::POST,
            "/api/v1/manufacturing/work-orders",
            $token,
        )
        .set_json(json!({
            "tenant_id": $tenant_id,
            "name": format!("WO-{}", uuid::Uuid::new_v4()),
            "product_id": $product_id,
            "quantity": "1.00",
            "priority": "Normal",
        }))
        .to_request();
        let resp = test::call_service($app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED, "Work order creation failed");
        let body = to_bytes(resp.into_body()).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        json["id"].as_i64().unwrap()
    }};
}

/// Helper macro to create a quality-control inspection (referencing
/// `product_id`, no work order) and return its id, for integration tests whose
/// create-endpoint prechecks require an owned parent `inspection_id` FK (NCRs).
/// The inspection's own `product_id` precheck is satisfied by the caller-supplied
/// (tenant-owned) product id.
/// Usage: `let inspection_id = seed_inspection!(&app, &token, 1, product_id);`
#[macro_export]
macro_rules! seed_inspection {
    ($app:expr, $token:expr, $tenant_id:expr, $product_id:expr) => {{
        let req = auth_request(
            actix_web::http::Method::POST,
            "/api/v1/manufacturing/inspections",
            $token,
        )
        .set_json(json!({
            "tenant_id": $tenant_id,
            "work_order_id": null,
            "product_id": $product_id,
            "inspection_type": "Visual",
            "quantity_inspected": "100.00",
            "quantity_passed": "95.00",
            "quantity_failed": "5.00",
            "status": "Passed",
        }))
        .to_request();
        let resp = test::call_service($app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED, "Inspection creation failed");
        let body = to_bytes(resp.into_body()).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        json["id"].as_i64().unwrap()
    }};
}

#[allow(dead_code)]
pub fn auth_request(method: actix_web::http::Method, uri: &str, token: &str) -> test::TestRequest {
    test::TestRequest::default()
        .method(method)
        .uri(uri)
        .insert_header(("Authorization", format!("Bearer {}", token)))
}
