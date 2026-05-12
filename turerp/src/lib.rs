//! Turerp ERP - Multi-tenant SaaS ERP system
//!
//! This is the core library for the Turerp ERP system built with Rust,
//! Actix-web, and SQLx.

pub mod api;
pub mod cache;
pub mod common;
pub mod config;
#[cfg(feature = "postgres")]
pub mod db;
pub mod domain;
pub mod error;
pub mod i18n;
pub mod middleware;
pub mod utils;

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

    use crate::common::{DbRouter, InMemoryDbRouter, ReadAfterWriteMode};
    use crate::common::{
        EventBus, InMemoryEventBus, InMemoryJobScheduler, JobScheduler, NotificationService,
    };
    use crate::common::{InMemoryReportEngine, ReportEngine};
    use crate::common::{InMemorySearchService, SearchService};
    use crate::common::{InMemoryTracingService, TracingService};
    use crate::config::Config;
    use crate::domain::accounting::repository::{
        BoxAccountRepository, BoxJournalEntryRepository, BoxJournalLineRepository,
    };
    use crate::domain::accounting::service::AccountingService;
    use crate::domain::archive::repository::{
        BoxArchiveJobRepository, BoxArchivePolicyRepository, BoxArchiveRecordRepository,
    };
    use crate::domain::archive::service::ArchiveService;
    use crate::domain::assets::repository::BoxAssetsRepository;
    use crate::domain::assets::service::AssetsService;
    use crate::domain::assets::AssetsRepository;
    use crate::domain::audit::repository::BoxAuditLogRepository;
    use crate::domain::audit::service::AuditService;
    use crate::domain::auth::AuthService;
    use crate::domain::bank::repository::BoxBankRepository;
    use crate::domain::bank::service::BankService;
    use crate::domain::cari::repository::BoxCariRepository;
    use crate::domain::cari::service::CariService;
    use crate::domain::chart_of_accounts::repository::BoxChartAccountRepository;
    use crate::domain::chart_of_accounts::service::ChartOfAccountsService;
    use crate::domain::company::repository::BoxCompanyRepository;
    use crate::domain::company::service::CompanyService;
    use crate::domain::cost_center::repository::BoxCostCenterRepository;
    use crate::domain::cost_center::service::CostCenterService;
    use crate::domain::crm::repository::{
        BoxCampaignRepository, BoxLeadRepository, BoxOpportunityRepository, BoxTicketRepository,
    };
    use crate::domain::crm::service::CrmService;
    use crate::domain::custom_field::repository::BoxCustomFieldRepository;
    use crate::domain::custom_field::service::CustomFieldService;
    use crate::domain::dashboard::repository::BoxDashboardRepository;
    use crate::domain::dashboard::service::DashboardService;
    use crate::domain::document::repository::BoxDocumentRepository;
    use crate::domain::document::service::DocumentService;
    use crate::domain::feature::service::FeatureFlagService;
    use crate::domain::feature::FeatureFlagRepository;
    use crate::domain::forecasting::repository::BoxForecastingRepository;
    use crate::domain::forecasting::service::ForecastingService;
    use crate::domain::hr::repository::{
        BoxAttendanceRepository, BoxEmployeeRepository, BoxLeaveRequestRepository,
        BoxLeaveTypeRepository, BoxPayrollRepository,
    };
    use crate::domain::hr::service::HrService;
    use crate::domain::invoice::repository::{
        BoxInvoiceLineRepository, BoxInvoiceRepository, BoxPaymentRepository,
    };
    use crate::domain::invoice::service::InvoiceService;
    use crate::domain::manufacturing::repository::{
        BoxBillOfMaterialsRepository, BoxRoutingRepository, BoxWorkOrderRepository,
    };
    use crate::domain::manufacturing::service::ManufacturingService;
    use crate::domain::product::repository::{
        BoxCategoryRepository, BoxProductRepository, BoxProductVariantRepository, BoxUnitRepository,
    };
    use crate::domain::product::service::ProductService;
    use crate::domain::project::repository::{
        BoxProjectCostRepository, BoxProjectRepository, BoxWbsItemRepository,
    };
    use crate::domain::project::service::ProjectService;
    use crate::domain::purchase::repository::{
        BoxGoodsReceiptLineRepository, BoxGoodsReceiptRepository, BoxPurchaseOrderLineRepository,
        BoxPurchaseOrderRepository, BoxPurchaseRequestLineRepository, BoxPurchaseRequestRepository,
    };
    use crate::domain::purchase::service::PurchaseService;
    use crate::domain::sales::repository::{
        BoxQuotationLineRepository, BoxQuotationRepository, BoxSalesOrderLineRepository,
        BoxSalesOrderRepository,
    };
    use crate::domain::sales::service::SalesService;
    use crate::domain::shift::repository::{
        BoxAttendanceRecordRepository, BoxShiftAssignmentRepository, BoxShiftRepository,
    };
    use crate::domain::shift::service::ShiftService;
    use crate::domain::stock::repository::{
        BoxStockLevelRepository, BoxStockMovementRepository, BoxWarehouseRepository,
    };
    use crate::domain::stock::service::StockService;
    use crate::domain::subscription::repository::BoxSubscriptionRepository;
    use crate::domain::subscription::service::SubscriptionService;
    use crate::domain::tax::repository::{BoxTaxPeriodRepository, BoxTaxRateRepository};
    use crate::domain::tax::service::TaxService;
    use crate::domain::tenant::repository::BoxTenantRepository;
    use crate::domain::tenant::repository::{
        BoxTenantConfigRepository, InMemoryTenantConfigRepository,
    };
    use crate::domain::tenant::service::{TenantConfigService, TenantService};
    use crate::domain::user::repository::BoxUserRepository;
    use crate::domain::user::service::UserService;
    use crate::domain::webhook::repository::{BoxWebhookDeliveryRepository, BoxWebhookRepository};
    use crate::domain::webhook::service::WebhookService;
    use crate::domain::workflow::repository::BoxWorkflowRepository;
    use crate::domain::workflow::service::WorkflowService;
    use crate::i18n::I18n;
    use crate::utils::jwt::JwtService;

    // In-memory repository imports
    use crate::domain::accounting::repository::{
        InMemoryAccountRepository, InMemoryJournalEntryRepository, InMemoryJournalLineRepository,
    };
    use crate::domain::archive::repository::{
        InMemoryArchiveJobRepository, InMemoryArchivePolicyRepository,
        InMemoryArchiveRecordRepository,
    };
    use crate::domain::assets::repository::InMemoryAssetsRepository;
    use crate::domain::audit::repository::InMemoryAuditLogRepository;
    use crate::domain::bank::repository::InMemoryBankRepository;
    use crate::domain::cari::repository::InMemoryCariRepository;
    use crate::domain::chart_of_accounts::repository::InMemoryChartAccountRepository;
    use crate::domain::company::repository::InMemoryCompanyRepository;
    use crate::domain::cost_center::repository::InMemoryCostCenterRepository;
    use crate::domain::crm::repository::{
        InMemoryCampaignRepository, InMemoryLeadRepository, InMemoryOpportunityRepository,
        InMemoryTicketRepository,
    };
    use crate::domain::currency::repository::{
        BoxCurrencyRepository, BoxExchangeRateRepository, InMemoryCurrencyRepository,
        InMemoryExchangeRateRepository,
    };
    use crate::domain::currency::service::CurrencyService;
    use crate::domain::custom_field::repository::InMemoryCustomFieldRepository;
    use crate::domain::dashboard::repository::InMemoryDashboardRepository;
    use crate::domain::document::repository::InMemoryDocumentRepository;
    use crate::domain::feature::repository::InMemoryFeatureFlagRepository;
    use crate::domain::forecasting::repository::InMemoryForecastingRepository;
    use crate::domain::hr::repository::{
        InMemoryAttendanceRepository, InMemoryEmployeeRepository, InMemoryLeaveRequestRepository,
        InMemoryLeaveTypeRepository, InMemoryPayrollRepository,
    };
    use crate::domain::invoice::repository::{
        InMemoryInvoiceLineRepository, InMemoryInvoiceRepository, InMemoryPaymentRepository,
    };
    use crate::domain::manufacturing::repository::{
        InMemoryBillOfMaterialsRepository, InMemoryRoutingRepository, InMemoryWorkOrderRepository,
    };
    use crate::domain::mfa::repository::InMemoryMfaRepository;
    use crate::domain::mfa::service::MfaService;
    use crate::domain::product::repository::{
        InMemoryCategoryRepository, InMemoryProductRepository, InMemoryProductVariantRepository,
        InMemoryUnitRepository,
    };
    use crate::domain::project::repository::{
        InMemoryProjectCostRepository, InMemoryProjectRepository, InMemoryWbsItemRepository,
    };
    use crate::domain::purchase::repository::{
        InMemoryGoodsReceiptLineRepository, InMemoryGoodsReceiptRepository,
        InMemoryPurchaseOrderLineRepository, InMemoryPurchaseOrderRepository,
        InMemoryPurchaseRequestLineRepository, InMemoryPurchaseRequestRepository,
    };
    use crate::domain::sales::repository::{
        InMemoryQuotationLineRepository, InMemoryQuotationRepository,
        InMemorySalesOrderLineRepository, InMemorySalesOrderRepository,
    };
    use crate::domain::shift::repository::{
        InMemoryAttendanceRecordRepository, InMemoryShiftAssignmentRepository,
        InMemoryShiftRepository,
    };
    use crate::domain::stock::repository::{
        InMemoryStockLevelRepository, InMemoryStockMovementRepository, InMemoryWarehouseRepository,
    };
    use crate::domain::subscription::repository::InMemorySubscriptionRepository;
    use crate::domain::tax::repository::{InMemoryTaxPeriodRepository, InMemoryTaxRateRepository};
    use crate::domain::tenant::repository::InMemoryTenantRepository;
    use crate::domain::user::repository::InMemoryUserRepository;
    use crate::domain::webhook::repository::{
        InMemoryWebhookDeliveryRepository, InMemoryWebhookRepository,
    };
    use crate::domain::workflow::repository::InMemoryWorkflowRepository;

    #[cfg(feature = "postgres")]
    use crate::common::PostgresSearchService;
    #[cfg(feature = "postgres")]
    use crate::db;
    #[cfg(feature = "postgres")]
    use crate::domain::accounting::postgres_repository::{
        PostgresAccountRepository, PostgresJournalEntryRepository, PostgresJournalLineRepository,
    };
    #[cfg(feature = "postgres")]
    use crate::domain::api_key::PostgresApiKeyRepository;
    #[cfg(feature = "postgres")]
    use crate::domain::archive::postgres_repository::{
        PostgresArchiveJobRepository, PostgresArchivePolicyRepository,
        PostgresArchiveRecordRepository,
    };
    #[cfg(feature = "postgres")]
    use crate::domain::assets::postgres_repository::{
        PostgresAssetCategoryRepository, PostgresAssetsRepository,
    };
    #[cfg(feature = "postgres")]
    use crate::domain::audit::postgres_repository::PostgresAuditLogRepository;
    #[cfg(feature = "postgres")]
    use crate::domain::bank::postgres_repository::PostgresBankRepository;
    #[cfg(feature = "postgres")]
    use crate::domain::cari::postgres_repository::PostgresCariRepository;
    #[cfg(feature = "postgres")]
    use crate::domain::chart_of_accounts::postgres_repository::PostgresChartAccountRepository;
    #[cfg(feature = "postgres")]
    use crate::domain::company::postgres_repository::PostgresCompanyRepository;
    #[cfg(feature = "postgres")]
    use crate::domain::cost_center::postgres_repository::PostgresCostCenterRepository;
    #[cfg(feature = "postgres")]
    use crate::domain::crm::postgres_repository::{
        PostgresCampaignRepository, PostgresLeadRepository, PostgresOpportunityRepository,
        PostgresTicketRepository,
    };
    #[cfg(feature = "postgres")]
    use crate::domain::currency::postgres_repository::{
        PostgresCurrencyRepository, PostgresExchangeRateRepository,
    };
    #[cfg(feature = "postgres")]
    use crate::domain::custom_field::postgres_repository::PostgresCustomFieldRepository;
    #[cfg(feature = "postgres")]
    use crate::domain::dashboard::postgres_repository::PostgresDashboardRepository;
    #[cfg(feature = "postgres")]
    use crate::domain::document::postgres_repository::PostgresDocumentRepository;
    #[cfg(feature = "postgres")]
    use crate::domain::edefter::postgres_repository::PostgresEDefterRepository;
    #[cfg(feature = "postgres")]
    use crate::domain::efatura::postgres_repository::PostgresEFaturaRepository;
    #[cfg(feature = "postgres")]
    use crate::domain::feature::postgres_repository::PostgresFeatureFlagRepository;
    #[cfg(feature = "postgres")]
    use crate::domain::forecasting::postgres_repository::PostgresForecastingRepository;
    #[cfg(feature = "postgres")]
    use crate::domain::hr::postgres_repository::{
        PostgresAttendanceRepository, PostgresEmployeeRepository, PostgresLeaveRequestRepository,
        PostgresLeaveTypeRepository, PostgresPayrollRepository,
    };
    #[cfg(feature = "postgres")]
    use crate::domain::invoice::postgres_repository::{
        PostgresInvoiceLineRepository, PostgresInvoiceRepository, PostgresPaymentRepository,
    };
    #[cfg(feature = "postgres")]
    use crate::domain::manufacturing::postgres_repository::{
        PostgresBillOfMaterialsRepository, PostgresRoutingRepository, PostgresWorkOrderRepository,
    };
    #[cfg(feature = "postgres")]
    use crate::domain::mfa::postgres_repository::PostgresMfaRepository;
    #[cfg(feature = "postgres")]
    use crate::domain::product::postgres_repository::{
        PostgresCategoryRepository, PostgresProductRepository, PostgresProductVariantRepository,
        PostgresUnitRepository,
    };
    #[cfg(feature = "postgres")]
    use crate::domain::project::postgres_repository::{
        PostgresProjectCostRepository, PostgresProjectRepository, PostgresWbsItemRepository,
    };
    #[cfg(feature = "postgres")]
    use crate::domain::purchase::postgres_repository::{
        PostgresGoodsReceiptLineRepository, PostgresGoodsReceiptRepository,
        PostgresPurchaseOrderLineRepository, PostgresPurchaseOrderRepository,
        PostgresPurchaseRequestLineRepository, PostgresPurchaseRequestRepository,
    };
    #[cfg(feature = "postgres")]
    use crate::domain::sales::postgres_repository::{
        PostgresQuotationLineRepository, PostgresQuotationRepository,
        PostgresSalesOrderLineRepository, PostgresSalesOrderRepository,
    };
    #[cfg(feature = "postgres")]
    use crate::domain::settings::postgres_repository::PostgresSettingsRepository;
    #[cfg(feature = "postgres")]
    use crate::domain::shift::postgres_repository::{
        PostgresAttendanceRecordRepository, PostgresShiftAssignmentRepository,
        PostgresShiftRepository,
    };
    #[cfg(feature = "postgres")]
    use crate::domain::stock::postgres_repository::{
        PostgresStockLevelRepository, PostgresStockMovementRepository, PostgresWarehouseRepository,
    };
    #[cfg(feature = "postgres")]
    use crate::domain::subscription::postgres_repository::PostgresSubscriptionRepository;
    #[cfg(feature = "postgres")]
    use crate::domain::tax::postgres_repository::{
        PostgresTaxPeriodRepository, PostgresTaxRateRepository,
    };
    #[cfg(feature = "postgres")]
    use crate::domain::tenant::postgres_repository::PostgresTenantRepository;
    #[cfg(feature = "postgres")]
    use crate::domain::user::postgres_repository::PostgresUserRepository;
    #[cfg(feature = "postgres")]
    use crate::domain::webhook::postgres_repository::{
        PostgresWebhookDeliveryRepository, PostgresWebhookRepository,
    };
    #[cfg(feature = "postgres")]
    use crate::domain::workflow::postgres_repository::PostgresWorkflowRepository;
    #[cfg(feature = "postgres")]
    use sqlx::PgPool;

    /// Auth domain services
    #[derive(Clone)]
    pub struct AuthState {
        pub auth_service: web::Data<AuthService>,
        pub user_service: web::Data<UserService>,
        pub jwt_service: web::Data<JwtService>,
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
        pub inter_company_service: web::Data<crate::common::inter_company::InterCompanyService>,
    }

    /// HR domain services
    #[derive(Clone)]
    pub struct HrState {
        pub hr_service: web::Data<HrService>,
        pub shift_service: web::Data<ShiftService>,
    }

    /// Admin domain services
    #[derive(Clone)]
    pub struct AdminState {
        pub tenant_service: web::Data<TenantService>,
        pub tenant_config_service: web::Data<TenantConfigService>,
        pub settings_service: web::Data<crate::domain::settings::SettingsService>,
        pub api_key_service: web::Data<crate::domain::api_key::ApiKeyService>,
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
        #[cfg(feature = "postgres")]
        pub db_pool: web::Data<Arc<PgPool>>,
        pub cdc_listener: Option<Arc<crate::common::cdc::CdcListener>>,
        pub import_service: web::Data<dyn crate::common::import::ImportService>,
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
        pub qc_service: web::Data<crate::domain::manufacturing::QualityControlService>,
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
        pub edefter_service: web::Data<crate::domain::edefter::EDefterService>,
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
        pub i18n: web::Data<I18n>,
    }

    /// Create all in-memory services
    macro_rules! create_in_memory_services {
        ($config:expr) => {{
            let config = $config;

            // Cache (created early so all services can attach it)
            let cache_service: Arc<dyn crate::cache::CacheService> =
                Arc::new(crate::cache::InMemoryCacheService::new()) as Arc<dyn crate::cache::CacheService>;

            // Auth & User
            let user_repo = Arc::new(InMemoryUserRepository::new()) as BoxUserRepository;
            let user_service = UserService::new(user_repo).with_cache(cache_service.clone());
            let jwt_service = JwtService::new(
                config.jwt.secret.clone(),
                config.jwt.access_token_expiration,
                config.jwt.refresh_token_expiration,
            );
            let mfa_repo = Arc::new(InMemoryMfaRepository::new())
                as crate::domain::mfa::repository::BoxMfaRepository;
            let mfa_service = MfaService::new(mfa_repo, jwt_service.clone());
            let auth_service = AuthService::new(
                user_service.clone(),
                jwt_service.clone(),
                mfa_service.clone(),
            );

            // Cari
            let cari_repo = Arc::new(InMemoryCariRepository::new()) as BoxCariRepository;
            let cari_repo_import = cari_repo.clone();
            let cari_service = CariService::new(cari_repo);

            // Company
            let company_repo = Arc::new(InMemoryCompanyRepository::new()) as BoxCompanyRepository;
            let company_service = CompanyService::new(company_repo);

            // Stock
            let warehouse_repo =
                Arc::new(InMemoryWarehouseRepository::new()) as BoxWarehouseRepository;
            let stock_level_repo =
                Arc::new(InMemoryStockLevelRepository::new()) as BoxStockLevelRepository;
            let stock_movement_repo =
                Arc::new(InMemoryStockMovementRepository::new()) as BoxStockMovementRepository;
            let stock_movement_repo_import = stock_movement_repo.clone();
            let stock_service =
                StockService::new(warehouse_repo, stock_level_repo, stock_movement_repo);

            // Invoice
            let invoice_repo = Arc::new(InMemoryInvoiceRepository::new()) as BoxInvoiceRepository;
            let invoice_line_repo =
                Arc::new(InMemoryInvoiceLineRepository::new()) as BoxInvoiceLineRepository;
            let payment_repo = Arc::new(InMemoryPaymentRepository::new()) as BoxPaymentRepository;
            let invoice_service =
                InvoiceService::new(invoice_repo, invoice_line_repo, payment_repo);

            // Sales
            let sales_order_repo =
                Arc::new(InMemorySalesOrderRepository::new()) as BoxSalesOrderRepository;
            let sales_order_line_repo =
                Arc::new(InMemorySalesOrderLineRepository::new()) as BoxSalesOrderLineRepository;
            let quotation_repo =
                Arc::new(InMemoryQuotationRepository::new()) as BoxQuotationRepository;
            let quotation_line_repo =
                Arc::new(InMemoryQuotationLineRepository::new()) as BoxQuotationLineRepository;
            let sales_service = SalesService::new(
                sales_order_repo,
                sales_order_line_repo,
                quotation_repo,
                quotation_line_repo,
            );

            // HR
            let employee_repo =
                Arc::new(InMemoryEmployeeRepository::new()) as BoxEmployeeRepository;
            let attendance_repo =
                Arc::new(InMemoryAttendanceRepository::new()) as BoxAttendanceRepository;
            let leave_request_repo =
                Arc::new(InMemoryLeaveRequestRepository::new()) as BoxLeaveRequestRepository;
            let leave_type_repo =
                Arc::new(InMemoryLeaveTypeRepository::new()) as BoxLeaveTypeRepository;
            let payroll_repo = Arc::new(InMemoryPayrollRepository::new()) as BoxPayrollRepository;
            let hr_service = HrService::new(
                employee_repo,
                attendance_repo,
                leave_request_repo,
                leave_type_repo,
                payroll_repo,
            );

            // Accounting
            let account_repo = Arc::new(InMemoryAccountRepository::new()) as BoxAccountRepository;
            let entry_repo =
                Arc::new(InMemoryJournalEntryRepository::new()) as BoxJournalEntryRepository;
            let line_repo =
                Arc::new(InMemoryJournalLineRepository::new()) as BoxJournalLineRepository;
            let accounting_service = AccountingService::new(account_repo, entry_repo, line_repo);

            // Project
            let project_repo = Arc::new(InMemoryProjectRepository::new()) as BoxProjectRepository;
            let wbs_repo = Arc::new(InMemoryWbsItemRepository::new()) as BoxWbsItemRepository;
            let cost_repo =
                Arc::new(InMemoryProjectCostRepository::new()) as BoxProjectCostRepository;
            let project_service = ProjectService::new(project_repo, wbs_repo, cost_repo);

            // Manufacturing
            let work_order_repo =
                Arc::new(InMemoryWorkOrderRepository::new()) as BoxWorkOrderRepository;
            let bom_repo =
                Arc::new(InMemoryBillOfMaterialsRepository::new()) as BoxBillOfMaterialsRepository;
            let routing_repo = Arc::new(InMemoryRoutingRepository::new()) as BoxRoutingRepository;
            let manufacturing_service =
                ManufacturingService::new(work_order_repo, bom_repo, routing_repo);

            // CRM
            let lead_repo = Arc::new(InMemoryLeadRepository::new()) as BoxLeadRepository;
            let opportunity_repo =
                Arc::new(InMemoryOpportunityRepository::new()) as BoxOpportunityRepository;
            let campaign_repo =
                Arc::new(InMemoryCampaignRepository::new()) as BoxCampaignRepository;
            let ticket_repo = Arc::new(InMemoryTicketRepository::new()) as BoxTicketRepository;
            let crm_service =
                CrmService::new(lead_repo, opportunity_repo, campaign_repo, ticket_repo);

            // Chart of Accounts
            let chart_account_repo =
                Arc::new(InMemoryChartAccountRepository::new()) as BoxChartAccountRepository;
            let chart_account_repo_import = chart_account_repo.clone();
            let chart_of_accounts_service = ChartOfAccountsService::new(chart_account_repo);

            // Custom Fields
            let custom_field_repo =
                Arc::new(InMemoryCustomFieldRepository::new()) as BoxCustomFieldRepository;
            let custom_field_service = CustomFieldService::new(custom_field_repo);

            // Tenant
            let tenant_repo = Arc::new(InMemoryTenantRepository::new()) as BoxTenantRepository;
            let tenant_service = TenantService::new(tenant_repo);
            let tenant_config_repo =
                Arc::new(InMemoryTenantConfigRepository::new()) as BoxTenantConfigRepository;
            let tenant_config_service =
                TenantConfigService::new(tenant_config_repo).with_cache(cache_service.clone());

            // Assets
            let asset_repo = Arc::new(InMemoryAssetsRepository::new()) as BoxAssetsRepository;
            let assets_service =
                AssetsService::new(Arc::from(asset_repo) as Arc<dyn AssetsRepository>);

            // Feature Flags
            let feature_repo =
                Arc::new(InMemoryFeatureFlagRepository::new()) as Arc<dyn FeatureFlagRepository>;
            let feature_service = FeatureFlagService::new(feature_repo);

            // Product
            let product_repo = Arc::new(InMemoryProductRepository::new()) as BoxProductRepository;
            let product_repo_import = product_repo.clone();
            let category_repo =
                Arc::new(InMemoryCategoryRepository::new()) as BoxCategoryRepository;
            let unit_repo = Arc::new(InMemoryUnitRepository::new()) as BoxUnitRepository;
            let variant_repo =
                Arc::new(InMemoryProductVariantRepository::new()) as BoxProductVariantRepository;
            let product_service =
                ProductService::with_variants(product_repo, category_repo, unit_repo, variant_repo)
                    .with_cache(cache_service.clone());

            // Purchase
            let order_repo =
                Arc::new(InMemoryPurchaseOrderRepository::new()) as BoxPurchaseOrderRepository;
            let order_line_repo = Arc::new(InMemoryPurchaseOrderLineRepository::new())
                as BoxPurchaseOrderLineRepository;
            let receipt_repo =
                Arc::new(InMemoryGoodsReceiptRepository::new()) as BoxGoodsReceiptRepository;
            let receipt_line_repo = Arc::new(InMemoryGoodsReceiptLineRepository::new())
                as BoxGoodsReceiptLineRepository;
            let request_repo =
                Arc::new(InMemoryPurchaseRequestRepository::new()) as BoxPurchaseRequestRepository;
            let request_line_repo = Arc::new(InMemoryPurchaseRequestLineRepository::new())
                as BoxPurchaseRequestLineRepository;
            let purchase_service = PurchaseService::with_requests(
                order_repo,
                order_line_repo,
                receipt_repo,
                receipt_line_repo,
                request_repo,
                request_line_repo,
            );

            // Audit
            let audit_repo = Arc::new(InMemoryAuditLogRepository::new()) as BoxAuditLogRepository;
            let audit_service = AuditService::new(audit_repo);

            // Archive
            let archive_policy_repo = Arc::new(InMemoryArchivePolicyRepository::new()) as BoxArchivePolicyRepository;
            let archive_job_repo = Arc::new(InMemoryArchiveJobRepository::new()) as BoxArchiveJobRepository;
            let archive_record_repo = Arc::new(InMemoryArchiveRecordRepository::new()) as BoxArchiveRecordRepository;
            let archive_service = ArchiveService::new(archive_policy_repo, archive_job_repo, archive_record_repo);

            // Quality Control
            let inspection_repo =
                Arc::new(crate::domain::manufacturing::InMemoryInspectionRepository::new())
                    as crate::domain::manufacturing::BoxInspectionRepository;
            let ncr_repo = Arc::new(crate::domain::manufacturing::InMemoryNcrRepository::new())
                as crate::domain::manufacturing::BoxNcrRepository;
            let qc_service =
                crate::domain::manufacturing::QualityControlService::new(inspection_repo, ncr_repo);

            // Settings
            let settings_repo = Arc::new(crate::domain::settings::InMemorySettingsRepository::new())
                as crate::domain::settings::BoxSettingsRepository;
            let settings_service = crate::domain::settings::SettingsService::new(settings_repo);

            // API Keys
            let api_key_repo = Arc::new(crate::domain::api_key::InMemoryApiKeyRepository::new())
                as crate::domain::api_key::BoxApiKeyRepository;
            let api_key_service = crate::domain::api_key::ApiKeyService::new(api_key_repo);

            // Job Scheduler
            let job_scheduler = Arc::new(InMemoryJobScheduler::new()) as Arc<dyn JobScheduler>;

            // Event Bus
            let event_bus = Arc::new(InMemoryEventBus::new()) as Arc<dyn EventBus>;

            // Notification Service
            let notification_repo = Arc::new(
                crate::domain::notification::repository::InMemoryNotificationRepository::new(),
            ) as crate::domain::notification::repository::BoxNotificationRepository;
            let in_app_repo = Arc::new(
                crate::domain::notification::repository::InMemoryInAppNotificationRepository::new(),
            ) as crate::domain::notification::repository::BoxInAppNotificationRepository;
            let pref_repo = Arc::new(
                crate::domain::notification::repository::InMemoryNotificationPreferenceRepository::new(),
            ) as crate::domain::notification::repository::BoxNotificationPreferenceRepository;
            let notification_service = Arc::new(
                crate::domain::notification::service::NotificationService::with_noop_providers(
                    notification_repo,
                    in_app_repo,
                    pref_repo,
                    job_scheduler.clone(),
                ),
            ) as Arc<dyn NotificationService>;

            // Report Engine
            let report_engine = Arc::new(InMemoryReportEngine::new()) as Arc<dyn ReportEngine>;

            // Tracing Service
            let tracing_service =
                Arc::new(InMemoryTracingService::new("turerp-erp")) as Arc<dyn TracingService>;

            // DB Router
            let db_router = Arc::new(InMemoryDbRouter::new(
                "localhost:5432/turerp",
                ReadAfterWriteMode::Eventual,
            )) as Arc<dyn DbRouter>;

            // Tax
            let tax_rate_repo = Arc::new(InMemoryTaxRateRepository::new()) as BoxTaxRateRepository;
            let tax_period_repo =
                Arc::new(InMemoryTaxPeriodRepository::new()) as BoxTaxPeriodRepository;
            let tax_service = TaxService::new(tax_rate_repo, tax_period_repo);

            // e-Fatura
            let efatura_repo = Arc::new(crate::domain::efatura::InMemoryEFaturaRepository::new())
                as crate::domain::efatura::BoxEFaturaRepository;
            let gib_gateway =
                Arc::new(crate::common::InMemoryGibGateway::new()) as crate::common::BoxGibGateway;
            let efatura_service =
                crate::domain::efatura::EFaturaService::new(efatura_repo, gib_gateway);

            // e-Defter
            let edefter_repo = Arc::new(crate::domain::edefter::InMemoryEDefterRepository::new())
                as crate::domain::edefter::BoxEDefterRepository;
            let edefter_service = crate::domain::edefter::EDefterService::new(edefter_repo);

            // Webhooks
            let webhook_repo = Arc::new(InMemoryWebhookRepository::new()) as BoxWebhookRepository;
            let delivery_repo =
                Arc::new(InMemoryWebhookDeliveryRepository::new()) as BoxWebhookDeliveryRepository;
            let webhook_service = WebhookService::new(webhook_repo, delivery_repo);

            // Currency
            let currency_repo =
                Arc::new(InMemoryCurrencyRepository::new()) as BoxCurrencyRepository;
            let exchange_rate_repo =
                Arc::new(InMemoryExchangeRateRepository::new()) as BoxExchangeRateRepository;
            let currency_service = CurrencyService::new(currency_repo, exchange_rate_repo);

            // Search
            let search_service: Arc<dyn SearchService> =
                Arc::new(InMemorySearchService::new()) as Arc<dyn SearchService>;

            // Bank
            let bank_repo = Arc::new(InMemoryBankRepository::new()) as BoxBankRepository;
            let bank_service = BankService::new(bank_repo);

            // Cost Centers
            let cost_center_repo =
                Arc::new(InMemoryCostCenterRepository::new()) as BoxCostCenterRepository;
            let cost_center_service = CostCenterService::new(cost_center_repo);

            // Dashboard
            let dashboard_repo =
                Arc::new(InMemoryDashboardRepository::new()) as BoxDashboardRepository;
            let dashboard_service = DashboardService::new(dashboard_repo, cache_service.clone());

            // Documents
            let document_repo =
                Arc::new(InMemoryDocumentRepository::new()) as BoxDocumentRepository;
            let document_service = DocumentService::new(document_repo);

            // Subscriptions
            let subscription_repo =
                Arc::new(InMemorySubscriptionRepository::new()) as BoxSubscriptionRepository;
            let subscription_service = SubscriptionService::new(subscription_repo);

            // Forecasting
            let forecasting_repo =
                Arc::new(InMemoryForecastingRepository::new()) as BoxForecastingRepository;
            let forecasting_service = ForecastingService::new(forecasting_repo);

            // Shift Planning
            let shift_repo = Arc::new(InMemoryShiftRepository::new()) as BoxShiftRepository;
            let assignment_repo =
                Arc::new(InMemoryShiftAssignmentRepository::new()) as BoxShiftAssignmentRepository;
            let attendance_repo =
                Arc::new(InMemoryAttendanceRecordRepository::new()) as BoxAttendanceRecordRepository;
            let shift_service = ShiftService::new(shift_repo, assignment_repo, attendance_repo);

            // Workflows
            let workflow_repo =
                Arc::new(InMemoryWorkflowRepository::new()) as BoxWorkflowRepository;
            let workflow_service = WorkflowService::new(workflow_repo, notification_service.clone(), job_scheduler.clone());

            // File Storage
            let file_storage: Arc<dyn crate::common::file_storage::FileStorage> =
                Arc::new(crate::common::file_storage::LocalFileStorage::new(format!(
                    "/tmp/turerp-test-files-{}",
                    std::process::id()
                )))
                    as Arc<dyn crate::common::file_storage::FileStorage>;

            // Rate limit stats
            let rate_limit_stats = crate::middleware::rate_limit::RateLimitStatsStore::default();

            // Import Service
            let import_service: Arc<dyn crate::common::import::ImportService> = Arc::new(
                crate::common::import::CsvImportService::new(
                    product_repo_import,
                    cari_repo_import,
                    chart_account_repo_import,
                    stock_movement_repo_import,
                    job_scheduler.clone(),
                ),
            );

            // Inter-Company Service
            let inter_company_service = crate::common::inter_company::InterCompanyService::new(
                Arc::new(company_service.clone()),
                Arc::new(invoice_service.clone()),
                Arc::new(stock_service.clone()),
                Arc::new(product_service.clone()),
            );

            (
                AuthState {
                    auth_service: web::Data::new(auth_service),
                    user_service: web::Data::new(user_service),
                    jwt_service: web::Data::new(jwt_service),
                },
                CommerceState {
                    cari_service: web::Data::new(cari_service),
                    company_service: web::Data::new(company_service),
                    stock_service: web::Data::new(stock_service),
                    invoice_service: web::Data::new(invoice_service),
                    sales_service: web::Data::new(sales_service),
                    purchase_service: web::Data::new(purchase_service),
                    product_service: web::Data::new(product_service),
                    inter_company_service: web::Data::new(inter_company_service),
                },
                HrState {
                    hr_service: web::Data::new(hr_service),
                    shift_service: web::Data::new(shift_service),
                },
                AdminState {
                    tenant_service: web::Data::new(tenant_service),
                    tenant_config_service: web::Data::new(tenant_config_service),
                    settings_service: web::Data::new(settings_service),
                    api_key_service: web::Data::new(api_key_service),
                },
                InfraState {
                    job_scheduler: web::Data::from(job_scheduler),
                    event_bus: web::Data::from(event_bus),
                    notification_service: web::Data::from(notification_service),
                    report_engine: web::Data::from(report_engine),
                    tracing_service: web::Data::from(tracing_service),
                    db_router: web::Data::from(db_router),
                    cache_service: web::Data::from(cache_service),
                    search_service: web::Data::from(search_service),
                    rate_limit_stats: web::Data::new(rate_limit_stats),
                    #[cfg(feature = "postgres")]
                    db_pool: web::Data::new(Arc::new(sqlx::PgPool::connect_lazy("postgres://localhost/dummy").expect("Failed to create lazy pool"))),
                    cdc_listener: None,
                    import_service: web::Data::from(import_service),
                },
                FinanceState {
                    accounting_service: web::Data::new(accounting_service),
                    bank_service: web::Data::new(bank_service),
                    cost_center_service: web::Data::new(cost_center_service),
                    tax_service: web::Data::new(tax_service),
                    currency_service: web::Data::new(currency_service),
                },
                ProjectState {
                    project_service: web::Data::new(project_service),
                    manufacturing_service: web::Data::new(manufacturing_service),
                    crm_service: web::Data::new(crm_service),
                    qc_service: web::Data::new(qc_service),
                },
                DocumentState {
                    document_service: web::Data::new(document_service),
                    file_storage: web::Data::from(file_storage),
                    dashboard_service: web::Data::new(dashboard_service),
                },
                IntegrationState {
                    efatura_service: web::Data::new(efatura_service),
                    edefter_service: web::Data::new(edefter_service),
                    webhook_service: web::Data::new(webhook_service),
                    workflow_service: web::Data::new(workflow_service),
                },
                AnalyticsState {
                    audit_service: web::Data::new(audit_service),
                    archive_service: web::Data::new(archive_service),
                    subscription_service: web::Data::new(subscription_service),
                    forecasting_service: web::Data::new(forecasting_service),
                },
                chart_of_accounts_service,
                custom_field_service,
                assets_service,
                feature_service,
            )
        }};
    }

    /// Create application state with in-memory storage (for development/testing)
    #[cfg(not(feature = "postgres"))]
    pub fn create_app_state(config: &Config) -> AppState {
        let (
            auth,
            commerce,
            hr,
            admin,
            infra,
            finance,
            project,
            document,
            integration,
            analytics,
            chart_of_accounts_service,
            custom_field_service,
            assets_service,
            feature_service,
        ) = create_in_memory_services!(config);

        let i18n = I18n::init();

        AppState {
            auth,
            commerce,
            hr,
            admin,
            infra,
            finance,
            project,
            document,
            integration,
            analytics,
            chart_of_accounts_service: web::Data::new(chart_of_accounts_service),
            custom_field_service: web::Data::new(custom_field_service),
            assets_service: web::Data::new(assets_service),
            feature_service: web::Data::new(feature_service),
            i18n: web::Data::new(i18n),
        }
    }

    /// Create application state with PostgreSQL storage (for production)
    #[cfg(feature = "postgres")]
    pub async fn create_app_state(config: &Config) -> AppState {
        // Create connection pool
        let pool = Arc::new(
            db::create_pool(&config.database)
                .await
                .expect("Failed to create database pool"),
        );

        let cache_service: Arc<dyn crate::cache::CacheService> = if config.redis.enabled {
            match crate::cache::RedisCacheService::new(&config.redis.url, config.redis.ttl_seconds)
                .await
            {
                Ok(redis_cache) => {
                    tracing::info!("Redis cache connected at {}", config.redis.url);
                    redis_cache.into_arc()
                }
                Err(e) => {
                    tracing::warn!("Failed to connect to Redis ({}), using no-op cache", e);
                    Arc::new(crate::cache::NoopCacheService) as Arc<dyn crate::cache::CacheService>
                }
            }
        } else {
            tracing::info!("Redis caching disabled");
            Arc::new(crate::cache::NoopCacheService) as Arc<dyn crate::cache::CacheService>
        };

        // Run migrations
        db::run_migrations(&pool)
            .await
            .expect("Failed to run migrations");

        // Auth & User - PostgreSQL
        let user_repo = PostgresUserRepository::new(pool.clone()).into_boxed();
        let user_service = UserService::new(user_repo).with_cache(cache_service.clone());
        let jwt_service = JwtService::new(
            config.jwt.secret.clone(),
            config.jwt.access_token_expiration,
            config.jwt.refresh_token_expiration,
        );
        let mfa_repo = PostgresMfaRepository::new(pool.clone()).into_boxed();
        let mfa_service = MfaService::new(mfa_repo, jwt_service.clone());
        let auth_service = AuthService::new(
            user_service.clone(),
            jwt_service.clone(),
            mfa_service.clone(),
        );

        // Cari - PostgreSQL
        let cari_repo =
            PostgresCariRepository::new(pool.clone(), cache_service.clone()).into_boxed();
        let cari_repo_import = cari_repo.clone();
        let cari_service = CariService::new(cari_repo);

        // Company - PostgreSQL
        let company_repo = PostgresCompanyRepository::new(pool.clone()).into_boxed();
        let company_service = CompanyService::new(company_repo);

        // Stock - PostgreSQL
        let warehouse_repo = PostgresWarehouseRepository::new(pool.clone()).into_boxed();
        let stock_level_repo = PostgresStockLevelRepository::new(pool.clone()).into_boxed();
        let stock_movement_repo = PostgresStockMovementRepository::new(pool.clone()).into_boxed();
        let stock_movement_repo_import = stock_movement_repo.clone();
        let stock_service =
            StockService::new(warehouse_repo, stock_level_repo, stock_movement_repo);

        // Invoice - PostgreSQL
        let invoice_repo = PostgresInvoiceRepository::new(pool.clone()).into_boxed();
        let invoice_line_repo = PostgresInvoiceLineRepository::new(pool.clone()).into_boxed();
        let payment_repo = PostgresPaymentRepository::new(pool.clone()).into_boxed();
        let invoice_service = InvoiceService::new(invoice_repo, invoice_line_repo, payment_repo);

        // Sales - PostgreSQL
        let sales_order_repo = PostgresSalesOrderRepository::new(pool.clone()).into_boxed();
        let sales_order_line_repo =
            PostgresSalesOrderLineRepository::new(pool.clone()).into_boxed();
        let quotation_repo = PostgresQuotationRepository::new(pool.clone()).into_boxed();
        let quotation_line_repo = PostgresQuotationLineRepository::new(pool.clone()).into_boxed();
        let sales_service = SalesService::new(
            sales_order_repo,
            sales_order_line_repo,
            quotation_repo,
            quotation_line_repo,
        );

        // HR - PostgreSQL
        let employee_repo = PostgresEmployeeRepository::new(pool.clone()).into_boxed();
        let attendance_repo = PostgresAttendanceRepository::new(pool.clone()).into_boxed();
        let leave_request_repo = PostgresLeaveRequestRepository::new(pool.clone()).into_boxed();
        let leave_type_repo = PostgresLeaveTypeRepository::new(pool.clone()).into_boxed();
        let payroll_repo = PostgresPayrollRepository::new(pool.clone()).into_boxed();
        let hr_service = HrService::new(
            employee_repo,
            attendance_repo,
            leave_request_repo,
            leave_type_repo,
            payroll_repo,
        );

        // Accounting - PostgreSQL
        let account_repo = PostgresAccountRepository::new(pool.clone()).into_boxed();
        let entry_repo = PostgresJournalEntryRepository::new(pool.clone()).into_boxed();
        let line_repo = PostgresJournalLineRepository::new(pool.clone()).into_boxed();
        let accounting_service = AccountingService::new(account_repo, entry_repo, line_repo);

        // Project - PostgreSQL
        let project_repo = PostgresProjectRepository::new(pool.clone()).into_boxed();
        let wbs_repo = PostgresWbsItemRepository::new(pool.clone()).into_boxed();
        let cost_repo = PostgresProjectCostRepository::new(pool.clone()).into_boxed();
        let project_service = ProjectService::new(project_repo, wbs_repo, cost_repo);

        // Manufacturing - PostgreSQL
        let work_order_repo = PostgresWorkOrderRepository::new(pool.clone()).into_boxed();
        let bom_repo = PostgresBillOfMaterialsRepository::new(pool.clone()).into_boxed();
        let routing_repo = PostgresRoutingRepository::new(pool.clone()).into_boxed();
        let manufacturing_service =
            ManufacturingService::new(work_order_repo, bom_repo, routing_repo);

        // CRM - PostgreSQL
        let lead_repo = PostgresLeadRepository::new(pool.clone()).into_boxed();
        let opportunity_repo = PostgresOpportunityRepository::new(pool.clone()).into_boxed();
        let campaign_repo = PostgresCampaignRepository::new(pool.clone()).into_boxed();
        let ticket_repo = PostgresTicketRepository::new(pool.clone()).into_boxed();
        let crm_service = CrmService::new(lead_repo, opportunity_repo, campaign_repo, ticket_repo);

        // Chart of Accounts - PostgreSQL
        let chart_account_repo = PostgresChartAccountRepository::new(pool.clone()).into_boxed();
        let chart_account_repo_import = chart_account_repo.clone();
        let chart_of_accounts_service = ChartOfAccountsService::new(chart_account_repo);

        // Custom Fields - PostgreSQL
        let custom_field_repo = PostgresCustomFieldRepository::new(pool.clone()).into_boxed();
        let custom_field_service = CustomFieldService::new(custom_field_repo);

        // Tenant - PostgreSQL
        let tenant_repo = PostgresTenantRepository::new(pool.clone()).into_boxed();
        let tenant_service = TenantService::new(tenant_repo);
        let tenant_config_repo =
            Arc::new(InMemoryTenantConfigRepository::new()) as BoxTenantConfigRepository;
        let tenant_config_service =
            TenantConfigService::new(tenant_config_repo).with_cache(cache_service.clone());
        // Quality Control - using in-memory repos until PostgreSQL repos are implemented
        let inspection_repo =
            Arc::new(crate::domain::manufacturing::InMemoryInspectionRepository::new())
                as crate::domain::manufacturing::BoxInspectionRepository;
        let ncr_repo = Arc::new(crate::domain::manufacturing::InMemoryNcrRepository::new())
            as crate::domain::manufacturing::BoxNcrRepository;
        let qc_service =
            crate::domain::manufacturing::QualityControlService::new(inspection_repo, ncr_repo);

        // Assets - PostgreSQL
        let asset_repo = PostgresAssetsRepository::new(pool.clone());
        let _asset_category_repo = PostgresAssetCategoryRepository::new(pool.clone());
        let assets_service = AssetsService::new(Arc::new(asset_repo) as Arc<dyn AssetsRepository>);

        // Feature Flags - PostgreSQL
        let feature_repo = PostgresFeatureFlagRepository::new(pool.clone()).into_boxed();
        let feature_service = FeatureFlagService::new(feature_repo);

        // Settings - PostgreSQL
        let settings_repo = PostgresSettingsRepository::new(pool.clone()).into_boxed();
        let settings_service = crate::domain::settings::SettingsService::new(settings_repo);

        // Product - PostgreSQL
        let product_repo =
            PostgresProductRepository::new(pool.clone(), cache_service.clone()).into_boxed();
        let product_repo_import = product_repo.clone();
        let category_repo = PostgresCategoryRepository::new(pool.clone()).into_boxed();
        let unit_repo = PostgresUnitRepository::new(pool.clone()).into_boxed();
        let variant_repo = PostgresProductVariantRepository::new(pool.clone()).into_boxed();
        let product_service =
            ProductService::with_variants(product_repo, category_repo, unit_repo, variant_repo)
                .with_cache(cache_service.clone());

        // Purchase - PostgreSQL
        let order_repo = PostgresPurchaseOrderRepository::new(pool.clone()).into_boxed();
        let order_line_repo = PostgresPurchaseOrderLineRepository::new(pool.clone()).into_boxed();
        let receipt_repo = PostgresGoodsReceiptRepository::new(pool.clone()).into_boxed();
        let receipt_line_repo = PostgresGoodsReceiptLineRepository::new(pool.clone()).into_boxed();
        let request_repo = PostgresPurchaseRequestRepository::new(pool.clone()).into_boxed();
        let request_line_repo =
            PostgresPurchaseRequestLineRepository::new(pool.clone()).into_boxed();
        let purchase_service = PurchaseService::with_requests(
            order_repo,
            order_line_repo,
            receipt_repo,
            receipt_line_repo,
            request_repo,
            request_line_repo,
        );

        // Audit - PostgreSQL
        let audit_repo = PostgresAuditLogRepository::new(pool.clone()).into_boxed();
        let audit_service = AuditService::new(audit_repo);

        // Archive - PostgreSQL
        let archive_policy_repo = PostgresArchivePolicyRepository::new(pool.clone()).into_boxed();
        let archive_job_repo = PostgresArchiveJobRepository::new(pool.clone()).into_boxed();
        let archive_record_repo = PostgresArchiveRecordRepository::new(pool.clone()).into_boxed();
        let archive_service =
            ArchiveService::new(archive_policy_repo, archive_job_repo, archive_record_repo);

        // Bank - PostgreSQL
        let bank_repo = PostgresBankRepository::new(pool.clone()).into_boxed();
        let bank_service = BankService::new(bank_repo);

        // Cost Centers - PostgreSQL
        let cost_center_repo = PostgresCostCenterRepository::new(pool.clone()).into_boxed();
        let cost_center_service = CostCenterService::new(cost_center_repo);

        // Dashboard - PostgreSQL
        let dashboard_repo = PostgresDashboardRepository::new(pool.clone()).into_boxed();
        let dashboard_service = DashboardService::new(dashboard_repo, cache_service.clone());

        // Documents - PostgreSQL
        let document_repo = PostgresDocumentRepository::new(pool.clone()).into_boxed();
        let document_service = DocumentService::new(document_repo);

        // Subscriptions - PostgreSQL
        let subscription_repo = PostgresSubscriptionRepository::new(pool.clone()).into_boxed();
        let subscription_service = SubscriptionService::new(subscription_repo);

        // Forecasting - PostgreSQL
        let forecasting_repo = PostgresForecastingRepository::new(pool.clone()).into_boxed();
        let forecasting_service = ForecastingService::new(forecasting_repo);

        // Shift Planning - PostgreSQL
        let shift_repo = PostgresShiftRepository::new(pool.clone()).into_boxed();
        let assignment_repo = PostgresShiftAssignmentRepository::new(pool.clone()).into_boxed();
        let attendance_repo = PostgresAttendanceRecordRepository::new(pool.clone()).into_boxed();
        let shift_service = ShiftService::new(shift_repo, assignment_repo, attendance_repo);

        // Workflows - PostgreSQL
        let workflow_repo = PostgresWorkflowRepository::new(pool.clone()).into_boxed();

        // API Keys - PostgreSQL
        let api_key_repo = PostgresApiKeyRepository::new(pool.clone()).into_boxed();
        let api_key_service = crate::domain::api_key::ApiKeyService::new(api_key_repo);

        // Job Scheduler - PostgreSQL
        let job_scheduler = Arc::new(db::job_repository::PostgresJobScheduler::new(pool.clone()))
            as Arc<dyn JobScheduler>;

        // Event Bus - in-memory
        let event_bus = Arc::new(InMemoryEventBus::new()) as Arc<dyn EventBus>;

        // Notification Service - PostgreSQL
        let notification_repo =
            crate::domain::notification::postgres_repository::PostgresNotificationRepository::new(
                pool.clone(),
            )
            .into_boxed();
        let in_app_repo = crate::domain::notification::postgres_repository::PostgresInAppNotificationRepository::new(pool.clone()).into_boxed();
        let pref_repo = crate::domain::notification::postgres_repository::PostgresNotificationPreferenceRepository::new(pool.clone()).into_boxed();
        let notification_service = Arc::new(
            crate::domain::notification::service::NotificationService::with_noop_providers(
                notification_repo,
                in_app_repo,
                pref_repo,
                job_scheduler.clone(),
            ),
        ) as Arc<dyn NotificationService>;

        let workflow_service = WorkflowService::new(
            workflow_repo,
            notification_service.clone(),
            job_scheduler.clone(),
        );

        // Report Engine - in-memory
        let report_engine = Arc::new(InMemoryReportEngine::new()) as Arc<dyn ReportEngine>;

        // Tracing Service - in-memory
        let tracing_service =
            Arc::new(InMemoryTracingService::new("turerp-erp")) as Arc<dyn TracingService>;

        // DB Router - in-memory
        let db_router = Arc::new(InMemoryDbRouter::new(
            "localhost:5432/turerp",
            ReadAfterWriteMode::Eventual,
        )) as Arc<dyn DbRouter>;

        // Tax - PostgreSQL
        let tax_rate_repo = PostgresTaxRateRepository::new(pool.clone()).into_boxed();
        let tax_period_repo = PostgresTaxPeriodRepository::new(pool.clone()).into_boxed();
        let tax_service = TaxService::new(tax_rate_repo, tax_period_repo);

        // Currency - PostgreSQL
        let currency_repo = PostgresCurrencyRepository::new(pool.clone()).into_boxed();
        let exchange_rate_repo = PostgresExchangeRateRepository::new(pool.clone()).into_boxed();
        let currency_service = CurrencyService::new(currency_repo, exchange_rate_repo);

        // e-Fatura - PostgreSQL
        let efatura_repo = PostgresEFaturaRepository::new(pool.clone()).into_boxed();
        let gib_gateway =
            Arc::new(crate::common::InMemoryGibGateway::new()) as crate::common::BoxGibGateway;
        let efatura_service =
            crate::domain::efatura::EFaturaService::new(efatura_repo, gib_gateway);

        // e-Defter - PostgreSQL
        let edefter_repo = PostgresEDefterRepository::new(pool.clone()).into_boxed();
        let edefter_service = crate::domain::edefter::EDefterService::new(edefter_repo);

        // Webhooks - PostgreSQL
        let webhook_repo = PostgresWebhookRepository::new(pool.clone()).into_boxed();
        let delivery_repo = PostgresWebhookDeliveryRepository::new(pool.clone()).into_boxed();
        let webhook_service = WebhookService::new(webhook_repo, delivery_repo);

        // Register webhook subscriber on event bus
        event_bus
            .subscribe(Arc::new(
                crate::domain::webhook::subscriber::WebhookSubscriber::new(Arc::new(
                    webhook_service.clone(),
                )),
            ))
            .await
            .ok();

        // Search
        #[cfg(feature = "postgres")]
        let search_service: Arc<dyn SearchService> =
            Arc::new(PostgresSearchService::new(pool.clone())) as Arc<dyn SearchService>;
        #[cfg(not(feature = "postgres"))]
        let search_service: Arc<dyn SearchService> =
            Arc::new(InMemorySearchService::new()) as Arc<dyn SearchService>;

        let rate_limit_stats = crate::middleware::rate_limit::RateLimitStatsStore::default();

        let i18n = I18n::init();

        let file_storage: Arc<dyn crate::common::file_storage::FileStorage> =
            Arc::new(crate::common::file_storage::LocalFileStorage::new(format!(
                "/tmp/turerp-test-files-{}",
                std::process::id()
            ))) as Arc<dyn crate::common::file_storage::FileStorage>;

        // Import Service
        let import_service: Arc<dyn crate::common::import::ImportService> =
            Arc::new(crate::common::import::CsvImportService::new(
                product_repo_import,
                cari_repo_import,
                chart_account_repo_import,
                stock_movement_repo_import,
                job_scheduler.clone(),
            ));

        // Inter-Company Service
        let inter_company_service = crate::common::inter_company::InterCompanyService::new(
            Arc::new(company_service.clone()),
            Arc::new(invoice_service.clone()),
            Arc::new(stock_service.clone()),
            Arc::new(product_service.clone()),
        );

        AppState {
            auth: AuthState {
                auth_service: web::Data::new(auth_service),
                user_service: web::Data::new(user_service),
                jwt_service: web::Data::new(jwt_service),
            },
            commerce: CommerceState {
                cari_service: web::Data::new(cari_service),
                company_service: web::Data::new(company_service),
                stock_service: web::Data::new(stock_service),
                invoice_service: web::Data::new(invoice_service),
                sales_service: web::Data::new(sales_service),
                purchase_service: web::Data::new(purchase_service),
                product_service: web::Data::new(product_service),
                inter_company_service: web::Data::new(inter_company_service),
            },
            hr: HrState {
                hr_service: web::Data::new(hr_service),
                shift_service: web::Data::new(shift_service),
            },
            admin: AdminState {
                tenant_service: web::Data::new(tenant_service),
                tenant_config_service: web::Data::new(tenant_config_service),
                settings_service: web::Data::new(settings_service),
                api_key_service: web::Data::new(api_key_service),
            },
            infra: InfraState {
                job_scheduler: web::Data::from(job_scheduler),
                event_bus: web::Data::from(event_bus),
                notification_service: web::Data::from(notification_service),
                report_engine: web::Data::from(report_engine),
                tracing_service: web::Data::from(tracing_service),
                db_router: web::Data::from(db_router),
                cache_service: web::Data::from(cache_service),
                search_service: web::Data::from(search_service),
                rate_limit_stats: web::Data::new(rate_limit_stats),
                db_pool: web::Data::new(pool),
                cdc_listener: None,
                import_service: web::Data::from(import_service),
            },
            finance: FinanceState {
                accounting_service: web::Data::new(accounting_service),
                bank_service: web::Data::new(bank_service),
                cost_center_service: web::Data::new(cost_center_service),
                tax_service: web::Data::new(tax_service),
                currency_service: web::Data::new(currency_service),
            },
            project: ProjectState {
                project_service: web::Data::new(project_service),
                manufacturing_service: web::Data::new(manufacturing_service),
                crm_service: web::Data::new(crm_service),
                qc_service: web::Data::new(qc_service),
            },
            document: DocumentState {
                document_service: web::Data::new(document_service),
                file_storage: web::Data::from(file_storage),
                dashboard_service: web::Data::new(dashboard_service),
            },
            integration: IntegrationState {
                efatura_service: web::Data::new(efatura_service),
                edefter_service: web::Data::new(edefter_service),
                webhook_service: web::Data::new(webhook_service),
                workflow_service: web::Data::new(workflow_service),
            },
            analytics: AnalyticsState {
                audit_service: web::Data::new(audit_service),
                archive_service: web::Data::new(archive_service),
                subscription_service: web::Data::new(subscription_service),
                forecasting_service: web::Data::new(forecasting_service),
            },
            chart_of_accounts_service: web::Data::new(chart_of_accounts_service),
            custom_field_service: web::Data::new(custom_field_service),
            assets_service: web::Data::new(assets_service),
            feature_service: web::Data::new(feature_service),
            i18n: web::Data::new(i18n),
        }
    }

    /// Create application state with in-memory storage
    #[cfg(not(feature = "postgres"))]
    pub fn create_app_state_in_memory(config: &Config) -> AppState {
        create_app_state(config)
    }

    /// Create application state with in-memory storage (postgres mode - for testing)
    #[cfg(feature = "postgres")]
    pub fn create_app_state_in_memory(config: &Config) -> AppState {
        let (
            auth,
            commerce,
            hr,
            admin,
            mut infra,
            finance,
            project,
            document,
            integration,
            analytics,
            chart_of_accounts_service,
            custom_field_service,
            assets_service,
            feature_service,
        ) = create_in_memory_services!(config);

        // For in-memory testing with postgres feature, create a mock pool
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy("postgres://localhost/dummy")
            .expect("Failed to create lazy pool");
        infra.db_pool = web::Data::new(Arc::new(pool));

        // Register webhook subscriber on event bus
        let webhook_service_arc = Arc::new(integration.webhook_service.get_ref().clone());
        tokio::runtime::Handle::current().block_on(async {
            infra
                .event_bus
                .subscribe(Arc::new(
                    crate::domain::webhook::subscriber::WebhookSubscriber::new(webhook_service_arc),
                ))
                .await
                .ok();
        });

        let i18n = I18n::init();

        AppState {
            auth,
            commerce,
            hr,
            admin,
            infra,
            finance,
            project,
            document,
            integration,
            analytics,
            chart_of_accounts_service: web::Data::new(chart_of_accounts_service),
            custom_field_service: web::Data::new(custom_field_service),
            assets_service: web::Data::new(assets_service),
            feature_service: web::Data::new(feature_service),
            i18n: web::Data::new(i18n),
        }
    }
}

/// Setup logging for the application
pub fn setup_logging() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "turerp=debug,actix_web=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
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
        let config = Config::default();
        let state = app::create_app_state_in_memory(&config);
        // Verify services are created
        assert!(std::sync::Arc::strong_count(&state.auth.auth_service) > 0);
        assert!(std::sync::Arc::strong_count(&state.auth.user_service) > 0);
        assert!(std::sync::Arc::strong_count(&state.auth.jwt_service) > 0);
        assert!(std::sync::Arc::strong_count(&state.commerce.cari_service) > 0);
    }
}
