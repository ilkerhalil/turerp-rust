use actix_web::{web, App};
use turerp::api::{
    auth_configure, users_configure, v1_accounting_configure, v1_assets_configure,
    v1_bank_configure, v1_cari_configure, v1_chart_of_accounts_configure, v1_cost_centers_configure,
    v1_crm_configure, v1_dashboard_configure, v1_feature_flags_configure, v1_files_configure,
    v1_hr_configure, v1_invoice_configure, v1_jobs_configure, v1_manufacturing_configure,
    v1_notifications_configure, v1_product_variants_configure, v1_project_configure,
    v1_purchase_requests_configure, v1_reports_configure, v1_sales_configure, v1_search_configure,
    v1_stock_configure, v1_subscriptions_configure, v1_tax_configure, v1_tenant_configure,
    v1_webhooks_configure, v1_workflows_configure,
};
use turerp::app::create_app_state_in_memory;
use turerp::config::Config;
use turerp::middleware::JwtAuthMiddleware;
use turerp::utils::jwt::JwtService;

/// Configure all legacy routes (auth + users)
pub fn configure_all_routes(cfg: &mut web::ServiceConfig) {
    auth_configure(cfg);
    users_configure(cfg);
}

/// Configure V1 routes for business modules
pub fn configure_v1_routes(cfg: &mut web::ServiceConfig) {
    cfg.configure(v1_bank_configure)
        .configure(v1_cari_configure)
        .configure(v1_chart_of_accounts_configure)
        .configure(v1_cost_centers_configure)
        .configure(v1_crm_configure)
        .configure(v1_dashboard_configure)
        .configure(v1_files_configure)
        .configure(v1_stock_configure)
        .configure(v1_invoice_configure)
        .configure(v1_sales_configure)
        .configure(v1_hr_configure)
        .configure(v1_accounting_configure)
        .configure(v1_project_configure)
        .configure(v1_manufacturing_configure)
        .configure(v1_tenant_configure)
        .configure(v1_assets_configure)
        .configure(v1_feature_flags_configure)
        .configure(v1_product_variants_configure)
        .configure(v1_purchase_requests_configure)
        .configure(v1_tax_configure)
        .configure(v1_webhooks_configure)
        .configure(v1_search_configure)
        .configure(v1_reports_configure)
        .configure(v1_jobs_configure)
        .configure(v1_notifications_configure)
        .configure(v1_subscriptions_configure)
        .configure(v1_workflows_configure);
}

/// Create app state with default config for testing
pub fn create_test_app_state() -> turerp::app::AppState {
    let config = Config::default();
    create_app_state_in_memory(&config)
}

/// Build a test app with all services and JWT middleware
pub fn build_full_test_app(
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
    let jwt = JwtService::new(
        Config::default().jwt.secret.clone(),
        Config::default().jwt.access_token_expiration,
        Config::default().jwt.refresh_token_expiration,
    );
    App::new()
        .wrap(JwtAuthMiddleware::new(jwt))
        .app_data(state.auth_service.clone())
        .app_data(state.user_service.clone())
        .app_data(state.jwt_service.clone())
        .app_data(state.cari_service.clone())
        .app_data(state.stock_service.clone())
        .app_data(state.invoice_service.clone())
        .app_data(state.sales_service.clone())
        .app_data(state.hr_service.clone())
        .app_data(state.accounting_service.clone())
        .app_data(state.project_service.clone())
        .app_data(state.manufacturing_service.clone())
        .app_data(state.crm_service.clone())
        .app_data(state.tenant_service.clone())
        .app_data(state.tenant_config_service.clone())
        .app_data(state.i18n.clone())
        .app_data(state.assets_service.clone())
        .app_data(state.feature_service.clone())
        .app_data(state.product_service.clone())
        .app_data(state.purchase_service.clone())
        .app_data(state.chart_of_accounts_service.clone())
        .app_data(state.tax_service.clone())
        .app_data(state.webhook_service.clone())
        .app_data(state.search_service.clone())
        .app_data(state.report_engine.clone())
        .app_data(state.job_scheduler.clone())
        .app_data(state.notification_service.clone())
        .app_data(state.audit_service.clone())
        .app_data(state.bank_service.clone())
        .app_data(state.cost_center_service.clone())
        .app_data(state.dashboard_service.clone())
        .app_data(state.file_storage.clone())
        .app_data(state.qc_service.clone())
        .app_data(state.settings_service.clone())
        .app_data(state.api_key_service.clone())
        .app_data(state.subscription_service.clone())
        .app_data(state.workflow_service.clone())
        .service(
            web::scope("/api")
                .configure(configure_all_routes)
                .configure(configure_v1_routes),
        )
}
