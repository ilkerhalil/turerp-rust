//! Turerp ERP - Multi-tenant SaaS ERP system
//!
//! This is the core library for the Turerp ERP system built with Rust,
//! Actix-web, and SQLx.

pub mod api;
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
        EventBus, InMemoryEventBus, InMemoryJobScheduler, InMemoryNotificationService,
        JobScheduler, NotificationService,
    };
    use crate::common::{InMemoryReportEngine, ReportEngine};
    use crate::common::{InMemoryTracingService, TracingService};
    use crate::config::Config;
    use crate::domain::accounting::repository::{
        BoxAccountRepository, BoxJournalEntryRepository, BoxJournalLineRepository,
    };
    use crate::domain::accounting::service::AccountingService;
    use crate::domain::assets::repository::BoxAssetsRepository;
    use crate::domain::assets::service::AssetsService;
    use crate::domain::assets::AssetsRepository;
    use crate::domain::audit::repository::BoxAuditLogRepository;
    use crate::domain::audit::service::AuditService;
    use crate::domain::auth::AuthService;
    use crate::domain::cari::repository::BoxCariRepository;
    use crate::domain::cari::service::CariService;
    use crate::domain::chart_of_accounts::repository::BoxChartAccountRepository;
    use crate::domain::chart_of_accounts::service::ChartOfAccountsService;
    use crate::domain::crm::repository::{
        BoxCampaignRepository, BoxLeadRepository, BoxOpportunityRepository, BoxTicketRepository,
    };
    use crate::domain::crm::service::CrmService;
    use crate::domain::custom_field::repository::BoxCustomFieldRepository;
    use crate::domain::custom_field::service::CustomFieldService;
    use crate::domain::feature::service::FeatureFlagService;
    use crate::domain::feature::FeatureFlagRepository;
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
    use crate::domain::stock::repository::{
        BoxStockLevelRepository, BoxStockMovementRepository, BoxWarehouseRepository,
    };
    use crate::domain::stock::service::StockService;
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
    use crate::i18n::I18n;
    use crate::utils::jwt::JwtService;

    // In-memory repository imports
    use crate::domain::accounting::repository::{
        InMemoryAccountRepository, InMemoryJournalEntryRepository, InMemoryJournalLineRepository,
    };
    use crate::domain::assets::repository::InMemoryAssetsRepository;
    use crate::domain::audit::repository::InMemoryAuditLogRepository;
    use crate::domain::cari::repository::InMemoryCariRepository;
    use crate::domain::chart_of_accounts::repository::InMemoryChartAccountRepository;
    use crate::domain::crm::repository::{
        InMemoryCampaignRepository, InMemoryLeadRepository, InMemoryOpportunityRepository,
        InMemoryTicketRepository,
    };
    use crate::domain::custom_field::repository::InMemoryCustomFieldRepository;
    use crate::domain::feature::repository::InMemoryFeatureFlagRepository;
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
    use crate::domain::stock::repository::{
        InMemoryStockLevelRepository, InMemoryStockMovementRepository, InMemoryWarehouseRepository,
    };
    use crate::domain::tax::repository::{InMemoryTaxPeriodRepository, InMemoryTaxRateRepository};
    use crate::domain::tenant::repository::InMemoryTenantRepository;
    use crate::domain::user::repository::InMemoryUserRepository;
    use crate::domain::webhook::repository::{
        InMemoryWebhookDeliveryRepository, InMemoryWebhookRepository,
    };

    #[cfg(feature = "postgres")]
    use crate::db;
    #[cfg(feature = "postgres")]
    use crate::domain::accounting::postgres_repository::{
        PostgresAccountRepository, PostgresJournalEntryRepository, PostgresJournalLineRepository,
    };
    #[cfg(feature = "postgres")]
    use crate::domain::api_key::PostgresApiKeyRepository;
    #[cfg(feature = "postgres")]
    use crate::domain::assets::postgres_repository::{
        PostgresAssetCategoryRepository, PostgresAssetsRepository,
    };
    #[cfg(feature = "postgres")]
    use crate::domain::audit::postgres_repository::PostgresAuditLogRepository;
    #[cfg(feature = "postgres")]
    use crate::domain::cari::postgres_repository::PostgresCariRepository;
    #[cfg(feature = "postgres")]
    use crate::domain::crm::postgres_repository::{
        PostgresCampaignRepository, PostgresLeadRepository, PostgresOpportunityRepository,
        PostgresTicketRepository,
    };
    #[cfg(feature = "postgres")]
    use crate::domain::edefter::postgres_repository::PostgresEDefterRepository;
    #[cfg(feature = "postgres")]
    use crate::domain::edefter::repository::BoxEDefterRepository;
    #[cfg(feature = "postgres")]
    use crate::domain::efatura::postgres_repository::PostgresEFaturaRepository;
    #[cfg(feature = "postgres")]
    use crate::domain::feature::postgres_repository::PostgresFeatureFlagRepository;
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
    use crate::domain::stock::postgres_repository::{
        PostgresStockLevelRepository, PostgresStockMovementRepository, PostgresWarehouseRepository,
    };
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
    use sqlx::PgPool;

    /// Application state data
    #[derive(Clone)]
    pub struct AppState {
        pub auth_service: web::Data<AuthService>,
        pub user_service: web::Data<UserService>,
        pub jwt_service: web::Data<JwtService>,
        pub cari_service: web::Data<CariService>,
        pub stock_service: web::Data<StockService>,
        pub invoice_service: web::Data<InvoiceService>,
        pub sales_service: web::Data<SalesService>,
        pub hr_service: web::Data<HrService>,
        pub accounting_service: web::Data<AccountingService>,
        pub project_service: web::Data<ProjectService>,
        pub manufacturing_service: web::Data<ManufacturingService>,
        pub crm_service: web::Data<CrmService>,
        pub chart_of_accounts_service: web::Data<ChartOfAccountsService>,
        pub custom_field_service: web::Data<CustomFieldService>,
        pub tenant_service: web::Data<TenantService>,
        pub tenant_config_service: web::Data<TenantConfigService>,
        pub assets_service: web::Data<AssetsService>,
        pub feature_service: web::Data<FeatureFlagService>,
        pub product_service: web::Data<ProductService>,
        pub purchase_service: web::Data<PurchaseService>,
        pub audit_service: web::Data<AuditService>,
        pub qc_service: web::Data<crate::domain::manufacturing::QualityControlService>,
        pub settings_service: web::Data<crate::domain::settings::SettingsService>,
        pub api_key_service: web::Data<crate::domain::api_key::ApiKeyService>,
        pub job_scheduler: web::Data<dyn JobScheduler>,
        pub event_bus: web::Data<dyn EventBus>,
        pub notification_service: web::Data<dyn NotificationService>,
        pub report_engine: web::Data<dyn ReportEngine>,
        pub tracing_service: web::Data<dyn TracingService>,
        pub db_router: web::Data<dyn DbRouter>,
        pub i18n: web::Data<I18n>,
        pub tax_service: web::Data<TaxService>,
        pub efatura_service: web::Data<crate::domain::efatura::EFaturaService>,
        pub edefter_service: web::Data<crate::domain::edefter::EDefterService>,
        pub webhook_service: web::Data<WebhookService>,
        #[cfg(feature = "postgres")]
        pub db_pool: web::Data<Arc<PgPool>>,
    }

    /// Create all in-memory services
    macro_rules! create_in_memory_services {
        ($config:expr) => {{
            let config = $config;
            // Auth & User
            let user_repo = Arc::new(InMemoryUserRepository::new()) as BoxUserRepository;
            let user_service = UserService::new(user_repo);
            let jwt_service = JwtService::new(
                config.jwt.secret.clone(),
                config.jwt.access_token_expiration,
                config.jwt.refresh_token_expiration,
            );
            let auth_service = AuthService::new(user_service.clone(), jwt_service.clone());

            // Cari
            let cari_repo = Arc::new(InMemoryCariRepository::new()) as BoxCariRepository;
            let cari_service = CariService::new(cari_repo);

            // Stock
            let warehouse_repo =
                Arc::new(InMemoryWarehouseRepository::new()) as BoxWarehouseRepository;
            let stock_level_repo =
                Arc::new(InMemoryStockLevelRepository::new()) as BoxStockLevelRepository;
            let stock_movement_repo =
                Arc::new(InMemoryStockMovementRepository::new()) as BoxStockMovementRepository;
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
            let tenant_config_service = TenantConfigService::new(tenant_config_repo);

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
            let category_repo =
                Arc::new(InMemoryCategoryRepository::new()) as BoxCategoryRepository;
            let unit_repo = Arc::new(InMemoryUnitRepository::new()) as BoxUnitRepository;
            let variant_repo =
                Arc::new(InMemoryProductVariantRepository::new()) as BoxProductVariantRepository;
            let product_service =
                ProductService::with_variants(product_repo, category_repo, unit_repo, variant_repo);

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
            let notification_service =
                Arc::new(InMemoryNotificationService::new()) as Arc<dyn NotificationService>;

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

            (
                auth_service,
                user_service,
                jwt_service,
                cari_service,
                stock_service,
                invoice_service,
                sales_service,
                hr_service,
                accounting_service,
                project_service,
                manufacturing_service,
                crm_service,
                chart_of_accounts_service,
                custom_field_service,
                tenant_service,
                tenant_config_service,
                assets_service,
                feature_service,
                product_service,
                purchase_service,
                audit_service,
                qc_service,
                settings_service,
                api_key_service,
                job_scheduler,
                event_bus,
                notification_service,
                report_engine,
                tracing_service,
                db_router,
                tax_service,
                efatura_service,
                edefter_service,
                webhook_service,
            )
        }};
    }

    /// Create application state with in-memory storage (for development/testing)
    #[cfg(not(feature = "postgres"))]
    pub fn create_app_state(config: &Config) -> AppState {
        let (
            auth_service,
            user_service,
            jwt_service,
            cari_service,
            stock_service,
            invoice_service,
            sales_service,
            hr_service,
            accounting_service,
            project_service,
            manufacturing_service,
            crm_service,
            chart_of_accounts_service,
            custom_field_service,
            tenant_service,
            tenant_config_service,
            assets_service,
            feature_service,
            product_service,
            purchase_service,
            audit_service,
            qc_service,
            settings_service,
            api_key_service,
            job_scheduler,
            event_bus,
            notification_service,
            report_engine,
            tracing_service,
            db_router,
            tax_service,
            efatura_service,
            edefter_service,
            webhook_service,
        ) = create_in_memory_services!(config);

        let i18n = I18n::init();

        AppState {
            auth_service: web::Data::new(auth_service),
            user_service: web::Data::new(user_service),
            jwt_service: web::Data::new(jwt_service),
            cari_service: web::Data::new(cari_service),
            stock_service: web::Data::new(stock_service),
            invoice_service: web::Data::new(invoice_service),
            sales_service: web::Data::new(sales_service),
            hr_service: web::Data::new(hr_service),
            accounting_service: web::Data::new(accounting_service),
            project_service: web::Data::new(project_service),
            manufacturing_service: web::Data::new(manufacturing_service),
            crm_service: web::Data::new(crm_service),
            chart_of_accounts_service: web::Data::new(chart_of_accounts_service),
            custom_field_service: web::Data::new(custom_field_service),
            tenant_service: web::Data::new(tenant_service),
            tenant_config_service: web::Data::new(tenant_config_service),
            assets_service: web::Data::new(assets_service),
            feature_service: web::Data::new(feature_service),
            product_service: web::Data::new(product_service),
            purchase_service: web::Data::new(purchase_service),
            audit_service: web::Data::new(audit_service),
            qc_service: web::Data::new(qc_service),
            settings_service: web::Data::new(settings_service),
            api_key_service: web::Data::new(api_key_service),
            job_scheduler: web::Data::from(job_scheduler),
            event_bus: web::Data::from(event_bus),
            notification_service: web::Data::from(notification_service),
            report_engine: web::Data::from(report_engine),
            tracing_service: web::Data::from(tracing_service),
            db_router: web::Data::from(db_router),
            i18n: web::Data::new(i18n),
            tax_service: web::Data::new(tax_service),
            efatura_service: web::Data::new(efatura_service),
            edefter_service: web::Data::new(edefter_service),
            webhook_service: web::Data::new(webhook_service),
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

        // Run migrations
        db::run_migrations(&pool)
            .await
            .expect("Failed to run migrations");

        // Auth & User - PostgreSQL
        let user_repo = PostgresUserRepository::new(pool.clone()).into_boxed();
        let user_service = UserService::new(user_repo);
        let jwt_service = JwtService::new(
            config.jwt.secret.clone(),
            config.jwt.access_token_expiration,
            config.jwt.refresh_token_expiration,
        );
        let auth_service = AuthService::new(user_service.clone(), jwt_service.clone());

        // Cari - PostgreSQL
        let cari_repo = PostgresCariRepository::new(pool.clone()).into_boxed();
        let cari_service = CariService::new(cari_repo);

        // Stock - PostgreSQL
        let warehouse_repo = PostgresWarehouseRepository::new(pool.clone()).into_boxed();
        let stock_level_repo = PostgresStockLevelRepository::new(pool.clone()).into_boxed();
        let stock_movement_repo = PostgresStockMovementRepository::new(pool.clone()).into_boxed();
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

        // Chart of Accounts - in-memory (no postgres repo yet)
        let chart_account_repo =
            Arc::new(InMemoryChartAccountRepository::new()) as BoxChartAccountRepository;
        let chart_of_accounts_service = ChartOfAccountsService::new(chart_account_repo);

        // Custom Fields - in-memory (no postgres repo yet)
        let custom_field_repo =
            Arc::new(InMemoryCustomFieldRepository::new()) as BoxCustomFieldRepository;
        let custom_field_service = CustomFieldService::new(custom_field_repo);

        // Tenant - PostgreSQL
        let tenant_repo = PostgresTenantRepository::new(pool.clone()).into_boxed();
        let tenant_service = TenantService::new(tenant_repo);
        let tenant_config_repo =
            Arc::new(InMemoryTenantConfigRepository::new()) as BoxTenantConfigRepository;
        let tenant_config_service = TenantConfigService::new(tenant_config_repo);
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
        let asset_category_repo = PostgresAssetCategoryRepository::new(pool.clone());
        let assets_service = AssetsService::new(Arc::new(asset_repo) as Arc<dyn AssetsRepository>);

        // Feature Flags - PostgreSQL
        let feature_repo = PostgresFeatureFlagRepository::new(pool.clone()).into_boxed();
        let feature_service = FeatureFlagService::new(feature_repo);

        // Settings - PostgreSQL
        let settings_repo = PostgresSettingsRepository::new(pool.clone()).into_boxed();
        let settings_service = crate::domain::settings::SettingsService::new(settings_repo);

        // Product - PostgreSQL
        let product_repo = PostgresProductRepository::new(pool.clone()).into_boxed();
        let category_repo = PostgresCategoryRepository::new(pool.clone()).into_boxed();
        let unit_repo = PostgresUnitRepository::new(pool.clone()).into_boxed();
        let variant_repo = PostgresProductVariantRepository::new(pool.clone()).into_boxed();
        let product_service =
            ProductService::with_variants(product_repo, category_repo, unit_repo, variant_repo);

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

        // API Keys - PostgreSQL
        let api_key_repo = PostgresApiKeyRepository::new(pool.clone()).into_boxed();
        let api_key_service = crate::domain::api_key::ApiKeyService::new(api_key_repo);

        // Job Scheduler - in-memory
        let job_scheduler = Arc::new(InMemoryJobScheduler::new()) as Arc<dyn JobScheduler>;

        // Event Bus - in-memory
        let event_bus = Arc::new(InMemoryEventBus::new()) as Arc<dyn EventBus>;

        // Notification Service - in-memory
        let notification_service =
            Arc::new(InMemoryNotificationService::new()) as Arc<dyn NotificationService>;

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

        // e-Fatura - PostgreSQL
        let efatura_repo = PostgresEFaturaRepository::new(pool.clone()).into_boxed();
        let gib_gateway =
            Arc::new(crate::common::InMemoryGibGateway::new()) as crate::common::BoxGibGateway;
        let efatura_service =
            crate::domain::efatura::EFaturaService::new(efatura_repo, gib_gateway);

        // e-Defter - in-memory (no postgres repo yet)
        let edefter_repo = Arc::new(crate::domain::edefter::InMemoryEDefterRepository::new())
            as crate::domain::edefter::BoxEDefterRepository;
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

        let i18n = I18n::init();

        AppState {
            auth_service: web::Data::new(auth_service),
            user_service: web::Data::new(user_service),
            jwt_service: web::Data::new(jwt_service),
            cari_service: web::Data::new(cari_service),
            stock_service: web::Data::new(stock_service),
            invoice_service: web::Data::new(invoice_service),
            sales_service: web::Data::new(sales_service),
            hr_service: web::Data::new(hr_service),
            accounting_service: web::Data::new(accounting_service),
            project_service: web::Data::new(project_service),
            manufacturing_service: web::Data::new(manufacturing_service),
            crm_service: web::Data::new(crm_service),
            chart_of_accounts_service: web::Data::new(chart_of_accounts_service),
            custom_field_service: web::Data::new(custom_field_service),
            tenant_service: web::Data::new(tenant_service),
            tenant_config_service: web::Data::new(tenant_config_service),
            assets_service: web::Data::new(assets_service),
            feature_service: web::Data::new(feature_service),
            product_service: web::Data::new(product_service),
            purchase_service: web::Data::new(purchase_service),
            audit_service: web::Data::new(audit_service),
            qc_service: web::Data::new(qc_service),
            settings_service: web::Data::new(settings_service),
            api_key_service: web::Data::new(api_key_service),
            job_scheduler: web::Data::from(job_scheduler),
            event_bus: web::Data::from(event_bus),
            notification_service: web::Data::from(notification_service),
            report_engine: web::Data::from(report_engine),
            tracing_service: web::Data::from(tracing_service),
            db_router: web::Data::from(db_router),
            tax_service: web::Data::new(tax_service),
            efatura_service: web::Data::new(efatura_service),
            edefter_service: web::Data::new(edefter_service),
            webhook_service: web::Data::new(webhook_service),
            db_pool: web::Data::new(pool),
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
            auth_service,
            user_service,
            jwt_service,
            cari_service,
            stock_service,
            invoice_service,
            sales_service,
            hr_service,
            accounting_service,
            project_service,
            manufacturing_service,
            crm_service,
            chart_of_accounts_service,
            custom_field_service,
            tenant_service,
            tenant_config_service,
            assets_service,
            feature_service,
            product_service,
            purchase_service,
            audit_service,
            qc_service,
            settings_service,
            api_key_service,
            job_scheduler,
            event_bus,
            notification_service,
            report_engine,
            tracing_service,
            db_router,
            tax_service,
            efatura_service,
            edefter_service,
            webhook_service,
        ) = create_in_memory_services!(config);

        // For in-memory testing with postgres feature, create a mock pool
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        let pool = rt.block_on(async {
            sqlx::postgres::PgPoolOptions::new()
                .max_connections(1)
                .connect_lazy("postgres://localhost/dummy")
                .expect("Failed to create lazy pool")
        });
        let db_pool = web::Data::new(Arc::new(pool));

        // Register webhook subscriber on event bus
        event_bus
            .subscribe(Arc::new(
                crate::domain::webhook::subscriber::WebhookSubscriber::new(Arc::new(
                    webhook_service.clone(),
                )),
            ))
            .await
            .ok();

        let i18n = I18n::init();

        AppState {
            auth_service: web::Data::new(auth_service),
            user_service: web::Data::new(user_service),
            jwt_service: web::Data::new(jwt_service),
            cari_service: web::Data::new(cari_service),
            stock_service: web::Data::new(stock_service),
            invoice_service: web::Data::new(invoice_service),
            sales_service: web::Data::new(sales_service),
            hr_service: web::Data::new(hr_service),
            accounting_service: web::Data::new(accounting_service),
            project_service: web::Data::new(project_service),
            manufacturing_service: web::Data::new(manufacturing_service),
            crm_service: web::Data::new(crm_service),
            chart_of_accounts_service: web::Data::new(chart_of_accounts_service),
            custom_field_service: web::Data::new(custom_field_service),
            tenant_service: web::Data::new(tenant_service),
            tenant_config_service: web::Data::new(tenant_config_service),
            assets_service: web::Data::new(assets_service),
            feature_service: web::Data::new(feature_service),
            product_service: web::Data::new(product_service),
            purchase_service: web::Data::new(purchase_service),
            audit_service: web::Data::new(audit_service),
            qc_service: web::Data::new(qc_service),
            settings_service: web::Data::new(settings_service),
            api_key_service: web::Data::new(api_key_service),
            job_scheduler: web::Data::from(job_scheduler),
            event_bus: web::Data::from(event_bus),
            notification_service: web::Data::from(notification_service),
            report_engine: web::Data::from(report_engine),
            tracing_service: web::Data::from(tracing_service),
            db_router: web::Data::from(db_router),
            tax_service: web::Data::new(tax_service),
            efatura_service: web::Data::new(efatura_service),
            edefter_service: web::Data::new(edefter_service),
            webhook_service: web::Data::new(webhook_service),
            db_pool,
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

    #[test]
    fn test_app_state_creation() {
        let config = Config::default();
        let state = app::create_app_state_in_memory(&config);
        // Verify services are created
        assert!(std::sync::Arc::strong_count(&state.auth_service) > 0);
        assert!(std::sync::Arc::strong_count(&state.user_service) > 0);
        assert!(std::sync::Arc::strong_count(&state.jwt_service) > 0);
        assert!(std::sync::Arc::strong_count(&state.cari_service) > 0);
    }
}
