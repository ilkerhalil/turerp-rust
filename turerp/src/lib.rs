//! Turerp ERP - Multi-tenant SaaS ERP system
//!
//! This is the core library for the Turerp ERP system built with Rust,
//! Actix-web, and SQLx.

pub mod api;
pub mod cache;
pub mod common;
pub mod config;
pub mod db;
pub mod domain;
pub mod error;
pub mod graphql;
pub mod i18n;
mod macros;
pub mod middleware;
mod state;
pub mod utils;

pub use state::AppStateBuilder;

// Re-export commonly used types
pub use config::Config;
pub use domain::{
    auth::{AuthService, LoginRequest, RefreshTokenRequest, RegisterRequest},
    cari::{CariResponse, CariService, CariStatus, CariType, CreateCari, UpdateCari},
    tenant::{CreateTenant, Tenant, UpdateTenant},
    user::{CreateUser, Role, UpdateUser, User, UserResponse, UserService},
};
pub use error::{ApiError, ApiResult, ErrorResponse};
pub use i18n::I18n;

/// Application state
pub mod app {
    use actix_web::web;
    use std::sync::Arc;

    use crate::config::Config;
    use crate::db;
    use crate::ApiError;

    use crate::common::circuit_breaker::CircuitBreakerRegistry;
    use crate::common::retry::BoxRetryStats;
    use crate::common::{DbRouter, EventBus, JobScheduler, NotificationService, ReportEngine};
    use crate::common::{SearchService, TracingService};
    use crate::domain::accounting::service::AccountingService;
    use crate::domain::archive::service::ArchiveService;
    use crate::domain::assets::service::AssetsService;
    use crate::domain::audit::service::AuditService;
    use crate::domain::auth::AuthService;
    use crate::domain::bank::service::BankService;
    use crate::domain::barcode::service::BarcodeService;
    use crate::domain::cari::service::CariService;
    use crate::domain::chart_of_accounts::service::ChartOfAccountsService;
    use crate::domain::company::service::CompanyService;
    use crate::domain::cost_center::service::CostCenterService;
    use crate::domain::crm::service::CrmService;
    use crate::domain::custom_field::service::CustomFieldService;
    use crate::domain::customer_portal::service::CustomerPortal;
    use crate::domain::dashboard::service::DashboardService;
    use crate::domain::document::service::DocumentService;
    use crate::domain::feature::service::FeatureFlagService;
    use crate::domain::forecasting::service::ForecastingService;
    use crate::domain::hr::service::HrService;
    use crate::domain::invoice::service::InvoiceService;
    use crate::domain::ldap::LdapSyncService;
    use crate::domain::manufacturing::service::ManufacturingService;
    use crate::domain::mfa::MfaService;
    use crate::domain::observability::service::ObservabilityService;
    use crate::domain::product::service::ProductService;
    use crate::domain::project::service::ProjectService;
    use crate::domain::purchase::service::PurchaseService;
    use crate::domain::sales::service::SalesService;
    use crate::domain::shift::service::ShiftService;
    use crate::domain::stock::service::StockService;
    use crate::domain::subscription::service::SubscriptionService;
    use crate::domain::tax::service::TaxService;
    use crate::domain::tenant::service::{TenantConfigService, TenantService};
    use crate::domain::user::service::UserService;
    use crate::domain::vendor_portal::VendorPortal;
    use crate::domain::webhook::service::WebhookService;
    use crate::domain::workflow::service::WorkflowService;
    use crate::i18n::I18n;
    use crate::utils::jwt::JwtService;
    use sqlx::PgPool;

    pub use crate::state::AppStateBuilder;

    /// Auth domain services
    #[derive(Clone)]
    pub struct AuthState {
        pub auth_service: web::Data<AuthService>,
        pub user_service: web::Data<UserService>,
        pub jwt_service: web::Data<JwtService>,
        pub mfa_service: web::Data<MfaService>,
    }

    /// Commerce domain services
    #[derive(Clone)]
    pub struct CommerceState {
        pub cari_service: web::Data<CariService>,
        pub company_service: web::Data<CompanyService>,
        pub stock_service: web::Data<StockService>,
        pub invoice_service: web::Data<InvoiceService>,
        pub sales_service: web::Data<SalesService>,
        pub purchase_service: web::Data<PurchaseService>,
        pub product_service: web::Data<ProductService>,
        pub barcode_service: web::Data<BarcodeService>,
        pub inter_company_service:
            web::Data<crate::domain::inter_company::service::InterCompanyService>,
    }

    /// HR domain services
    #[derive(Clone)]
    pub struct HrState {
        pub hr_service: web::Data<HrService>,
        pub shift_service: web::Data<ShiftService>,
        pub sgk_payroll_service: web::Data<crate::domain::hr::sgk::service::SgkPayrollService>,
    }

    /// Admin domain services
    #[derive(Clone)]
    pub struct AdminState {
        pub tenant_service: web::Data<TenantService>,
        pub tenant_config_service: web::Data<TenantConfigService>,
        pub settings_service: web::Data<crate::domain::settings::SettingsService>,
        pub api_key_service: web::Data<crate::domain::api_key::ApiKeyService>,
        pub ip_whitelist_service: web::Data<crate::domain::ip_whitelist::IpWhitelistService>,
    }

    /// Infrastructure services
    #[derive(Clone)]
    pub struct InfraState {
        pub job_scheduler: web::Data<dyn JobScheduler>,
        pub event_bus: web::Data<dyn EventBus>,
        pub notification_service: web::Data<dyn NotificationService>,
        pub report_engine: web::Data<dyn ReportEngine>,
        pub tracing_service: web::Data<dyn TracingService>,
        pub db_router: web::Data<dyn DbRouter>,
        pub cache_service: web::Data<dyn crate::cache::CacheService>,
        pub search_service: web::Data<dyn SearchService>,
        pub rate_limit_stats: web::Data<crate::middleware::rate_limit::RateLimitStatsStore>,
        pub db_pool: Option<web::Data<Arc<PgPool>>>,
        pub cdc_listener: Option<Arc<crate::common::cdc::CdcListener>>,
        pub import_service: web::Data<dyn crate::common::import::ImportService>,
        pub circuit_breaker_registry: web::Data<CircuitBreakerRegistry>,
        pub retry_stats: web::Data<BoxRetryStats>,
    }

    /// Accounting & Finance domain services
    #[derive(Clone)]
    pub struct FinanceState {
        pub accounting_service: web::Data<AccountingService>,
        pub bank_service: web::Data<BankService>,
        pub cost_center_service: web::Data<CostCenterService>,
        pub tax_service: web::Data<TaxService>,
        pub currency_service: web::Data<crate::domain::currency::service::CurrencyService>,
    }

    /// Project & Manufacturing domain services
    #[derive(Clone)]
    pub struct ProjectState {
        pub project_service: web::Data<ProjectService>,
        pub manufacturing_service: web::Data<ManufacturingService>,
        pub crm_service: web::Data<CrmService>,
        pub qc_service: web::Data<crate::domain::quality_control::QualityControlService>,
    }

    /// Document & Content domain services
    #[derive(Clone)]
    pub struct DocumentState {
        pub document_service: web::Data<DocumentService>,
        pub file_storage: web::Data<dyn crate::common::file_storage::FileStorage>,
        pub dashboard_service: web::Data<DashboardService>,
    }

    /// Integration & External domain services
    #[derive(Clone)]
    pub struct IntegrationState {
        pub efatura_service: web::Data<crate::domain::efatura::EFaturaService>,
        pub earchive_service: web::Data<crate::domain::earchive::EarchiveService>,
        pub edefter_service: web::Data<crate::domain::edefter::EDefterService>,
        pub blockchain_ledger_service:
            web::Data<crate::domain::edefter::blockchain::BlockchainLedgerService>,
        pub customer_portal_service: web::Data<dyn CustomerPortal>,
        pub vendor_portal_service: web::Data<dyn VendorPortal>,
        pub webhook_service: web::Data<WebhookService>,
        pub workflow_service: web::Data<WorkflowService>,
    }

    /// Analytics & Reporting domain services
    #[derive(Clone)]
    pub struct AnalyticsState {
        pub audit_service: web::Data<AuditService>,
        pub archive_service: web::Data<ArchiveService>,
        pub subscription_service: web::Data<SubscriptionService>,
        pub forecasting_service: web::Data<ForecastingService>,
    }

    /// Application state data
    #[derive(Clone)]
    pub struct AppState {
        pub auth: AuthState,
        pub commerce: CommerceState,
        pub hr: HrState,
        pub admin: AdminState,
        pub infra: InfraState,
        pub finance: FinanceState,
        pub project: ProjectState,
        pub document: DocumentState,
        pub integration: IntegrationState,
        pub analytics: AnalyticsState,
        pub chart_of_accounts_service: web::Data<ChartOfAccountsService>,
        pub custom_field_service: web::Data<CustomFieldService>,
        pub assets_service: web::Data<AssetsService>,
        pub feature_service: web::Data<FeatureFlagService>,
        pub observability_service: web::Data<ObservabilityService>,
        pub ldap_service: web::Data<LdapSyncService>,
        pub i18n: web::Data<I18n>,
        pub schema: crate::graphql::AppSchema,
    }

    impl AppState {
        /// Register all service data into the Actix application.
        ///
        /// Centralizes the 60+ `.app_data()` calls that were previously
        /// duplicated inside `main.rs`'s `build_app_core!` macro.
        pub fn register_services(&self, cfg: &mut web::ServiceConfig) {
            cfg.app_data(web::Data::new(self.clone())); // Full AppState for health probes
            cfg.app_data(web::JsonConfig::default().limit(1024 * 1024)); // 1MB JSON limit

            cfg.app_data(self.auth.auth_service.clone());
            cfg.app_data(self.auth.user_service.clone());
            cfg.app_data(self.auth.jwt_service.clone());

            cfg.app_data(self.commerce.cari_service.clone());
            cfg.app_data(self.commerce.stock_service.clone());
            cfg.app_data(self.commerce.invoice_service.clone());
            cfg.app_data(self.commerce.sales_service.clone());
            cfg.app_data(self.commerce.barcode_service.clone());
            cfg.app_data(self.commerce.product_service.clone());
            cfg.app_data(self.commerce.purchase_service.clone());
            cfg.app_data(self.commerce.inter_company_service.clone());
            cfg.app_data(self.commerce.company_service.clone());

            cfg.app_data(self.hr.hr_service.clone());
            cfg.app_data(self.hr.sgk_payroll_service.clone());
            cfg.app_data(self.hr.shift_service.clone());

            cfg.app_data(self.finance.accounting_service.clone());
            cfg.app_data(self.finance.tax_service.clone());
            cfg.app_data(self.finance.bank_service.clone());
            cfg.app_data(self.finance.cost_center_service.clone());
            cfg.app_data(self.finance.currency_service.clone());

            cfg.app_data(self.project.project_service.clone());
            cfg.app_data(self.project.manufacturing_service.clone());
            cfg.app_data(self.project.qc_service.clone());
            cfg.app_data(self.project.crm_service.clone());

            cfg.app_data(self.chart_of_accounts_service.clone());
            cfg.app_data(self.custom_field_service.clone());

            cfg.app_data(self.admin.tenant_service.clone());
            cfg.app_data(self.admin.tenant_config_service.clone());
            cfg.app_data(self.admin.settings_service.clone());
            cfg.app_data(self.admin.api_key_service.clone());
            cfg.app_data(self.admin.ip_whitelist_service.clone());

            cfg.app_data(self.assets_service.clone());
            cfg.app_data(self.feature_service.clone());

            cfg.app_data(self.analytics.audit_service.clone());
            cfg.app_data(self.analytics.forecasting_service.clone());
            cfg.app_data(self.analytics.subscription_service.clone());
            cfg.app_data(self.analytics.archive_service.clone());

            cfg.app_data(self.observability_service.clone());
            cfg.app_data(self.i18n.clone());

            cfg.app_data(self.infra.job_scheduler.clone());
            cfg.app_data(self.infra.event_bus.clone());
            cfg.app_data(self.infra.notification_service.clone());
            cfg.app_data(self.infra.report_engine.clone());
            cfg.app_data(self.infra.tracing_service.clone());
            cfg.app_data(self.infra.db_router.clone());
            cfg.app_data(self.infra.cache_service.clone());
            cfg.app_data(self.infra.search_service.clone());
            cfg.app_data(self.infra.rate_limit_stats.clone());
            cfg.app_data(self.infra.import_service.clone());
            cfg.app_data(self.infra.circuit_breaker_registry.clone());
            cfg.app_data(self.infra.retry_stats.clone());

            cfg.app_data(self.integration.efatura_service.clone());
            cfg.app_data(self.integration.earchive_service.clone());
            cfg.app_data(self.integration.edefter_service.clone());
            cfg.app_data(self.integration.blockchain_ledger_service.clone());
            cfg.app_data(self.integration.customer_portal_service.clone());
            cfg.app_data(self.integration.vendor_portal_service.clone());
            cfg.app_data(self.integration.webhook_service.clone());
            cfg.app_data(self.integration.workflow_service.clone());

            cfg.app_data(self.document.document_service.clone());
            cfg.app_data(self.document.dashboard_service.clone());
            cfg.app_data(self.document.file_storage.clone());

            cfg.app_data(self.auth.mfa_service.clone());
            cfg.app_data(self.ldap_service.clone());
        }
    }

    /// Create application state with in-memory storage (postgres mode - for testing)
    pub async fn create_app_state_in_memory(config: &Config) -> Result<AppState, ApiError> {
        AppStateBuilder::new(config)
            .with_in_memory_repositories()
            .build()
            .await
    }

    /// Create application state with PostgreSQL storage (for production)
    pub async fn create_app_state(config: &Config) -> Result<AppState, ApiError> {
        let pool = Arc::new(db::create_pool(&config.database).await?);
        AppStateBuilder::new(config)
            .with_postgres_pool(pool)
            .build()
            .await
    }
    /// Create application state based on runtime configuration
    pub async fn create_app_state_unified(config: &Config) -> Result<AppState, ApiError> {
        if config.database.url.is_empty() {
            tracing::info!("Using in-memory storage (no database URL configured)");
            Ok(create_app_state_in_memory(config).await?)
        } else {
            tracing::info!("Using PostgreSQL storage");
            create_app_state(config).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lib_exists() {
        assert_eq!(42, 42);
    }

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.server.port, 8000);
        assert!(config.is_development());
    }

    #[tokio::test]
    async fn test_app_state_creation() {
        let config = Config {
            encryption_key: "YWJjZGVmZ2hpamtsbW5vcHFyc3R1dnd4eXoxMjM0NTY=".to_string(),
            ..Config::default()
        };
        let state = app::create_app_state_in_memory(&config)
            .await
            .expect("app state creation failed");
        // Verify services are created
        assert!(std::sync::Arc::strong_count(&state.auth.auth_service) > 0);
        assert!(std::sync::Arc::strong_count(&state.auth.user_service) > 0);
        assert!(std::sync::Arc::strong_count(&state.auth.jwt_service) > 0);
        assert!(std::sync::Arc::strong_count(&state.commerce.cari_service) > 0);
    }
}
