//! Application state builder.
//!
//! This module provides [`AppStateBuilder`], the single source of truth for
//! wiring up the 60+ services and repositories that compose [`AppState`](crate::app::AppState).
//!
//! Historically there were two near-identical functions,
//! `create_app_state` (PostgreSQL) and `create_app_state_in_memory`, that
//! duplicated roughly 500 lines of repository construction logic. They are now
//! thin wrappers that delegate to `AppStateBuilder`.
//!
//! # Usage
//!
//! ## Default (in-memory, no DB)
//!
//! ```ignore
//! let state = AppStateBuilder::new(&config).build().await?;
//! ```
//!
//! ## Explicit in-memory mode
//!
//! ```ignore
//! let state = AppStateBuilder::new(&config)
//!     .with_in_memory_repositories()
//!     .build()
//!     .await?;
//! ```
//!
//! ## PostgreSQL
//!
//! ```ignore
//! let pool = db::create_pool(&config.database).await?;
//! let state = AppStateBuilder::new(&config)
//!     .with_postgres_pool(pool)
//!     .build()
//!     .await?;
//! ```
//!
//! ## Optional services
//!
//! `with_redis` (only valid alongside a postgres pool) and `with_cdc_listener`
//! remain opt-in for callers that need them.

use actix_web::web;
use std::sync::Arc;

use crate::app::AppState;
use crate::common::circuit_breaker::CircuitBreakerRegistry;
use crate::common::retry::BoxRetryStats;
use crate::common::{DbRouter, InMemoryDbRouter, ReadAfterWriteMode};
use crate::common::{
    EventBus, InMemoryEventBus, InMemoryJobScheduler, JobScheduler, NotificationService,
};
use crate::common::{InMemoryReportEngine, ReportEngine};
use crate::common::{InMemorySearchService, InMemoryTracingService, SearchService, TracingService};
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
use crate::domain::barcode::repository::BoxBarcodeRepository;
use crate::domain::barcode::service::BarcodeService;
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
use crate::domain::currency::repository::{BoxCurrencyRepository, BoxExchangeRateRepository};
use crate::domain::currency::service::CurrencyService;
use crate::domain::custom_field::repository::BoxCustomFieldRepository;
use crate::domain::custom_field::service::CustomFieldService;
use crate::domain::customer_portal::service::{BoxCustomerPortal, CustomerPortalService};
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
use crate::domain::observability::repository::{
    BoxObservabilityRepository, InMemoryObservabilityRepository,
};
use crate::domain::observability::service::ObservabilityService;
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
use crate::domain::tenant::service::{TenantConfigService, TenantService};
use crate::domain::user::repository::BoxUserRepository;
use crate::domain::user::service::UserService;
use crate::domain::vendor_portal::service::{BoxVendorPortal, VendorPortalService};
use crate::domain::webhook::repository::{BoxWebhookDeliveryRepository, BoxWebhookRepository};
use crate::domain::webhook::service::WebhookService;
use crate::domain::workflow::repository::BoxWorkflowRepository;
use crate::domain::workflow::service::WorkflowService;
use crate::i18n::I18n;
use crate::utils::jwt::JwtService;
use crate::ApiError;

// In-memory repository imports.
use crate::domain::accounting::repository::{
    InMemoryAccountRepository, InMemoryJournalEntryRepository, InMemoryJournalLineRepository,
};
use crate::domain::archive::repository::{
    InMemoryArchiveJobRepository, InMemoryArchivePolicyRepository, InMemoryArchiveRecordRepository,
};
use crate::domain::assets::repository::InMemoryAssetsRepository;
use crate::domain::audit::repository::InMemoryAuditLogRepository;
use crate::domain::bank::repository::InMemoryBankRepository;
use crate::domain::barcode::repository::InMemoryBarcodeRepository;
use crate::domain::cari::repository::InMemoryCariRepository;
use crate::domain::chart_of_accounts::repository::InMemoryChartAccountRepository;
use crate::domain::company::repository::InMemoryCompanyRepository;
use crate::domain::cost_center::repository::InMemoryCostCenterRepository;
use crate::domain::crm::repository::{
    InMemoryCampaignRepository, InMemoryLeadRepository, InMemoryOpportunityRepository,
    InMemoryTicketRepository,
};
use crate::domain::currency::repository::{
    InMemoryCurrencyRepository, InMemoryExchangeRateRepository,
};
use crate::domain::custom_field::repository::InMemoryCustomFieldRepository;
use crate::domain::customer_portal::repository::{
    InMemoryPortalUserRepository, InMemorySupportTicketRepository,
};
use crate::domain::dashboard::repository::InMemoryDashboardRepository;
use crate::domain::document::repository::InMemoryDocumentRepository;
use crate::domain::feature::repository::InMemoryFeatureFlagRepository;
use crate::domain::forecasting::repository::InMemoryForecastingRepository;
use crate::domain::hr::repository::{
    InMemoryAttendanceRepository, InMemoryEmployeeRepository, InMemoryLeaveRequestRepository,
    InMemoryLeaveTypeRepository, InMemoryPayrollRepository,
};
use crate::domain::hr::sgk::repository::{
    InMemoryEmployeeBonusRepository, InMemorySgkConfigRepository,
    InMemorySgkEmployeeRegistrationRepository,
};
use crate::domain::invoice::repository::{
    InMemoryInvoiceLineRepository, InMemoryInvoiceRepository, InMemoryPaymentRepository,
};
use crate::domain::ldap::repository::InMemoryLdapConfigRepository;
use crate::domain::ldap::service::LdapSyncService;
use crate::domain::ldap::BoxLdapConfigRepository;
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
    InMemoryQuotationLineRepository, InMemoryQuotationRepository, InMemorySalesOrderLineRepository,
    InMemorySalesOrderRepository,
};
use crate::domain::shift::repository::{
    InMemoryAttendanceRecordRepository, InMemoryShiftAssignmentRepository, InMemoryShiftRepository,
};
use crate::domain::stock::repository::{
    InMemoryStockLevelRepository, InMemoryStockMovementRepository, InMemoryWarehouseRepository,
};
use crate::domain::subscription::repository::InMemorySubscriptionRepository;
use crate::domain::tax::repository::{InMemoryTaxPeriodRepository, InMemoryTaxRateRepository};
use crate::domain::tenant::repository::InMemoryTenantConfigRepository;
use crate::domain::tenant::repository::InMemoryTenantRepository;
use crate::domain::user::repository::InMemoryUserRepository;
use crate::domain::vendor_portal::repository::{
    InMemoryDeliveryNoteRepository, InMemoryVendorUserRepository,
};
use crate::domain::webhook::repository::{
    InMemoryWebhookDeliveryRepository, InMemoryWebhookRepository,
};
use crate::domain::workflow::repository::InMemoryWorkflowRepository;

use crate::common::PostgresSearchService;
use crate::db;
use crate::domain::accounting::postgres_repository::{
    PostgresAccountRepository, PostgresJournalEntryRepository, PostgresJournalLineRepository,
};
use crate::domain::api_key::PostgresApiKeyRepository;
use crate::domain::archive::postgres_repository::{
    PostgresArchiveJobRepository, PostgresArchivePolicyRepository, PostgresArchiveRecordRepository,
};
use crate::domain::assets::postgres_repository::{
    PostgresAssetCategoryRepository, PostgresAssetsRepository,
};
use crate::domain::audit::postgres_repository::PostgresAuditLogRepository;
use crate::domain::bank::postgres_repository::PostgresBankRepository;
use crate::domain::barcode::postgres_repository::PostgresBarcodeRepository;
use crate::domain::cari::postgres_repository::PostgresCariRepository;
use crate::domain::chart_of_accounts::postgres_repository::PostgresChartAccountRepository;
use crate::domain::company::postgres_repository::PostgresCompanyRepository;
use crate::domain::cost_center::postgres_repository::PostgresCostCenterRepository;
use crate::domain::crm::postgres_repository::{
    PostgresCampaignRepository, PostgresLeadRepository, PostgresOpportunityRepository,
    PostgresTicketRepository,
};
use crate::domain::currency::postgres_repository::{
    PostgresCurrencyRepository, PostgresExchangeRateRepository,
};
use crate::domain::custom_field::postgres_repository::PostgresCustomFieldRepository;
use crate::domain::customer_portal::postgres_repository::{
    PostgresPortalUserRepository, PostgresSupportTicketRepository,
};
use crate::domain::dashboard::postgres_repository::PostgresDashboardRepository;
use crate::domain::document::postgres_repository::PostgresDocumentRepository;
use crate::domain::earchive::postgres_repository::PostgresEarchiveRepository;
use crate::domain::edefter::blockchain::postgres_repository::PostgresBlockchainLedgerRepository;
use crate::domain::edefter::postgres_repository::PostgresEDefterRepository;
use crate::domain::efatura::postgres_repository::PostgresEFaturaRepository;
use crate::domain::feature::postgres_repository::PostgresFeatureFlagRepository;
use crate::domain::forecasting::postgres_repository::PostgresForecastingRepository;
use crate::domain::hr::postgres_repository::{
    PostgresAttendanceRepository, PostgresEmployeeRepository, PostgresLeaveRequestRepository,
    PostgresLeaveTypeRepository, PostgresPayrollRepository,
};
use crate::domain::hr::sgk::postgres_repository::{
    PostgresEmployeeBonusRepository, PostgresSgkConfigRepository,
    PostgresSgkEmployeeRegistrationRepository,
};
use crate::domain::invoice::postgres_repository::{
    PostgresInvoiceLineRepository, PostgresInvoiceRepository, PostgresPaymentRepository,
};
use crate::domain::ip_whitelist::postgres_repository::PostgresIpWhitelistRepository;
use crate::domain::ldap::postgres_repository::PostgresLdapConfigRepository;
use crate::domain::manufacturing::postgres_repository::{
    PostgresBillOfMaterialsRepository, PostgresRoutingRepository, PostgresWorkOrderRepository,
};
use crate::domain::mfa::postgres_repository::PostgresMfaRepository;
use crate::domain::observability::postgres_repository::PostgresObservabilityRepository;
use crate::domain::product::postgres_repository::{
    PostgresCategoryRepository, PostgresProductRepository, PostgresProductVariantRepository,
    PostgresUnitRepository,
};
use crate::domain::project::postgres_repository::{
    PostgresProjectCostRepository, PostgresProjectRepository, PostgresWbsItemRepository,
};
use crate::domain::purchase::postgres_repository::{
    PostgresGoodsReceiptLineRepository, PostgresGoodsReceiptRepository,
    PostgresPurchaseOrderLineRepository, PostgresPurchaseOrderRepository,
    PostgresPurchaseRequestLineRepository, PostgresPurchaseRequestRepository,
};
use crate::domain::quality_control::postgres_repository::{
    PostgresInspectionRepository, PostgresNcrRepository,
};
use crate::domain::sales::postgres_repository::{
    PostgresQuotationLineRepository, PostgresQuotationRepository, PostgresSalesOrderLineRepository,
    PostgresSalesOrderRepository,
};
use crate::domain::settings::postgres_repository::PostgresSettingsRepository;
use crate::domain::shift::postgres_repository::{
    PostgresAttendanceRecordRepository, PostgresShiftAssignmentRepository, PostgresShiftRepository,
};
use crate::domain::stock::postgres_repository::{
    PostgresStockLevelRepository, PostgresStockMovementRepository, PostgresWarehouseRepository,
};
use crate::domain::subscription::postgres_repository::PostgresSubscriptionRepository;
use crate::domain::tax::postgres_repository::{
    PostgresTaxPeriodRepository, PostgresTaxRateRepository,
};
use crate::domain::tenant::postgres_repository::{
    PostgresTenantConfigRepository, PostgresTenantRepository,
};
use crate::domain::user::postgres_repository::PostgresUserRepository;
use crate::domain::vendor_portal::postgres_repository::{
    PostgresDeliveryNoteRepository, PostgresVendorUserRepository,
};
use crate::domain::webhook::postgres_repository::{
    PostgresWebhookDeliveryRepository, PostgresWebhookRepository,
};
use crate::domain::workflow::postgres_repository::PostgresWorkflowRepository;
use sqlx::PgPool;

/// Backend selection for `AppStateBuilder`.
///
/// `Default` is `InMemory` so that callers can write `AppStateBuilder::new(&cfg)`
/// for zero-config testing without ever touching a database. Callers that want
/// real persistence call `with_postgres_pool(...)` to opt in.
#[derive(Clone, Default)]
pub(crate) enum StorageBackend {
    #[default]
    InMemory,
    Postgres(Arc<PgPool>),
}

/// Builder for [`AppState`].
///
/// Replaces the duplicated `create_app_state` and `create_app_state_in_memory`
/// functions. The builder is `async`-aware: `build()` is the only `async`
/// method, and the rest of the API is synchronous, chainable, and cheap.
pub struct AppStateBuilder<'a> {
    config: &'a Config,
    backend: StorageBackend,
    redis_enabled: bool,
    cdc_listener: Option<Arc<crate::common::cdc::CdcListener>>,
    migrations_already_run: bool,
}

impl<'a> AppStateBuilder<'a> {
    /// Construct a builder with the given configuration.
    ///
    /// Defaults to in-memory storage and migrations-already-run. Call
    /// [`with_postgres_pool`](Self::with_postgres_pool) to opt in to a real
    /// database, and pass `migrations_already_run: false` (via
    /// [`skip_migrations`](Self::skip_migrations)) to disable them.
    pub fn new(config: &'a Config) -> Self {
        Self {
            config,
            backend: StorageBackend::InMemory,
            redis_enabled: false,
            cdc_listener: None,
            migrations_already_run: true,
        }
    }

    /// Use in-memory repositories (default; provided for explicit reads).
    pub fn with_in_memory_repositories(mut self) -> Self {
        self.backend = StorageBackend::InMemory;
        self
    }

    /// Use a real PostgreSQL pool. Migrations will be run by `build()` unless
    /// [`skip_migrations`](Self::skip_migrations) was called.
    pub fn with_postgres_pool(mut self, pool: Arc<PgPool>) -> Self {
        self.backend = StorageBackend::Postgres(pool);
        self
    }

    /// Force Redis cache to be enabled. Only meaningful in postgres mode.
    /// In in-memory mode the cache is always an `InMemoryCacheService`.
    pub fn with_redis(mut self) -> Self {
        self.redis_enabled = true;
        self
    }

    /// Attach a CDC listener to the resulting `InfraState`.
    pub fn with_cdc_listener(mut self, listener: Arc<crate::common::cdc::CdcListener>) -> Self {
        self.cdc_listener = Some(listener);
        self
    }

    /// Skip running migrations during `build()`. Useful for tests that share
    /// a database or that pre-run migrations out-of-band.
    pub fn skip_migrations(mut self) -> Self {
        self.migrations_already_run = true;
        self
    }

    /// Consume the builder, wire up every service, and return the final
    /// [`AppState`].
    pub async fn build(self) -> Result<AppState, ApiError> {
        let AppStateBuilder {
            config,
            backend,
            redis_enabled,
            cdc_listener,
            migrations_already_run: _migrations_already_run,
        } = self;

        let pg_pool = match &backend {
            StorageBackend::Postgres(pool) => {
                if !_migrations_already_run {
                    db::run_migrations(pool).await.map_err(|e| {
                        tracing::error!("Failed to run migrations: {}", e);
                        ApiError::Database(format!("Failed to run migrations: {}", e))
                    })?;
                }
                Some(pool.clone())
            }
            StorageBackend::InMemory => None,
        };

        // Cache service: in-memory mode => InMemoryCacheService.
        //             postgres mode => Redis (if enabled and reachable) or Noop.
        let cache_service: Arc<dyn crate::cache::CacheService> = match &backend {
            StorageBackend::InMemory => Arc::new(crate::cache::InMemoryCacheService::new())
                as Arc<dyn crate::cache::CacheService>,
            StorageBackend::Postgres(_) => {
                let want_redis = redis_enabled || config.redis.enabled;
                if want_redis {
                    match crate::cache::RedisCacheService::new(
                        &config.redis.url,
                        config.redis.ttl_seconds,
                    )
                    .await
                    {
                        Ok(redis_cache) => {
                            tracing::info!("Redis cache connected at {}", config.redis.url);
                            redis_cache.into_arc()
                        }
                        Err(e) => {
                            tracing::warn!("Failed to connect to Redis ({}), using no-op cache", e);
                            Arc::new(crate::cache::NoopCacheService)
                                as Arc<dyn crate::cache::CacheService>
                        }
                    }
                } else {
                    tracing::info!("Redis caching disabled");
                    Arc::new(crate::cache::NoopCacheService) as Arc<dyn crate::cache::CacheService>
                }
            }
        };

        // ---- Auth & User ----
        let user_repo: BoxUserRepository = match &backend {
            StorageBackend::InMemory => {
                Arc::new(InMemoryUserRepository::new()) as BoxUserRepository
            }
            StorageBackend::Postgres(pool) => {
                PostgresUserRepository::new(pool.clone()).into_boxed()
            }
        };
        let user_service = UserService::new(user_repo).with_cache(cache_service.clone());

        let jwt_service = JwtService::new(
            config.jwt.secret.clone(),
            config.jwt.access_token_expiration,
            config.jwt.refresh_token_expiration,
        );

        let mfa_service = match &backend {
            StorageBackend::InMemory => {
                let mfa_repo = Arc::new(InMemoryMfaRepository::new())
                    as crate::domain::mfa::repository::BoxMfaRepository;
                MfaService::new(mfa_repo, jwt_service.clone())
            }
            StorageBackend::Postgres(pool) => {
                let mfa_repo = PostgresMfaRepository::new(pool.clone()).into_boxed();
                MfaService::new(mfa_repo, jwt_service.clone())
            }
        };

        let auth_service = match &backend {
            StorageBackend::InMemory => {
                let revoked_token_store =
                    Arc::new(crate::domain::auth::InMemoryRevokedTokenStore::new())
                        as crate::domain::auth::BoxRevokedTokenStore;
                AuthService::new(
                    user_service.clone(),
                    jwt_service.clone(),
                    mfa_service.clone(),
                    None,
                    revoked_token_store,
                )
            }
            StorageBackend::Postgres(pool) => {
                let revoked_token_store =
                    crate::domain::auth::PostgresRevokedTokenStore::new(pool.clone()).into_boxed();
                AuthService::new(
                    user_service.clone(),
                    jwt_service.clone(),
                    mfa_service.clone(),
                    Some(pool.clone()),
                    revoked_token_store,
                )
            }
        };

        // ---- Cari ----
        let cari_repo: BoxCariRepository = match &backend {
            StorageBackend::InMemory => {
                Arc::new(InMemoryCariRepository::new()) as BoxCariRepository
            }
            StorageBackend::Postgres(pool) => {
                PostgresCariRepository::new(pool.clone(), cache_service.clone()).into_boxed()
            }
        };
        let cari_repo_import = cari_repo.clone();
        let cari_service = CariService::new(cari_repo);

        // ---- Company ----
        let company_repo: BoxCompanyRepository = match &backend {
            StorageBackend::InMemory => {
                Arc::new(InMemoryCompanyRepository::new()) as BoxCompanyRepository
            }
            StorageBackend::Postgres(pool) => {
                PostgresCompanyRepository::new(pool.clone()).into_boxed()
            }
        };
        let company_service = CompanyService::new(company_repo);

        // ---- Stock ----
        let stock_service = match &backend {
            StorageBackend::InMemory => {
                let warehouse_repo =
                    Arc::new(InMemoryWarehouseRepository::new()) as BoxWarehouseRepository;
                let stock_level_repo =
                    Arc::new(InMemoryStockLevelRepository::new()) as BoxStockLevelRepository;
                let stock_movement_repo =
                    Arc::new(InMemoryStockMovementRepository::new()) as BoxStockMovementRepository;
                let stock_movement_repo_import = stock_movement_repo.clone();
                let svc = StockService::new(warehouse_repo, stock_level_repo, stock_movement_repo);
                (svc, stock_movement_repo_import)
            }
            StorageBackend::Postgres(pool) => {
                let warehouse_repo = PostgresWarehouseRepository::new(pool.clone()).into_boxed();
                let stock_level_repo = PostgresStockLevelRepository::new(pool.clone()).into_boxed();
                let stock_movement_repo =
                    PostgresStockMovementRepository::new(pool.clone()).into_boxed();
                let stock_movement_repo_import = stock_movement_repo.clone();
                let svc = StockService::new(warehouse_repo, stock_level_repo, stock_movement_repo);
                (svc, stock_movement_repo_import)
            }
        };
        let (stock_service, stock_movement_repo_import) = stock_service;

        // ---- Invoice ----
        let invoice_service = match &backend {
            StorageBackend::InMemory => {
                let invoice_repo =
                    Arc::new(InMemoryInvoiceRepository::new()) as BoxInvoiceRepository;
                let invoice_line_repo =
                    Arc::new(InMemoryInvoiceLineRepository::new()) as BoxInvoiceLineRepository;
                let payment_repo =
                    Arc::new(InMemoryPaymentRepository::new()) as BoxPaymentRepository;
                InvoiceService::new(invoice_repo, invoice_line_repo, payment_repo)
            }
            StorageBackend::Postgres(pool) => {
                let invoice_repo = PostgresInvoiceRepository::new(pool.clone()).into_boxed();
                let invoice_line_repo =
                    PostgresInvoiceLineRepository::new(pool.clone()).into_boxed();
                let payment_repo = PostgresPaymentRepository::new(pool.clone()).into_boxed();
                InvoiceService::new(invoice_repo, invoice_line_repo, payment_repo)
            }
        };

        // ---- Sales ----
        let sales_service = match &backend {
            StorageBackend::InMemory => {
                let so_repo =
                    Arc::new(InMemorySalesOrderRepository::new()) as BoxSalesOrderRepository;
                let so_line = Arc::new(InMemorySalesOrderLineRepository::new())
                    as BoxSalesOrderLineRepository;
                let q_repo = Arc::new(InMemoryQuotationRepository::new()) as BoxQuotationRepository;
                let q_line =
                    Arc::new(InMemoryQuotationLineRepository::new()) as BoxQuotationLineRepository;
                SalesService::new(so_repo, so_line, q_repo, q_line)
            }
            StorageBackend::Postgres(pool) => {
                let so_repo = PostgresSalesOrderRepository::new(pool.clone()).into_boxed();
                let so_line = PostgresSalesOrderLineRepository::new(pool.clone()).into_boxed();
                let q_repo = PostgresQuotationRepository::new(pool.clone()).into_boxed();
                let q_line = PostgresQuotationLineRepository::new(pool.clone()).into_boxed();
                SalesService::new(so_repo, so_line, q_repo, q_line)
            }
        };

        // ---- HR ----
        let hr_service = match &backend {
            StorageBackend::InMemory => {
                let employee_repo =
                    Arc::new(InMemoryEmployeeRepository::new()) as BoxEmployeeRepository;
                let attendance_repo =
                    Arc::new(InMemoryAttendanceRepository::new()) as BoxAttendanceRepository;
                let leave_request_repo =
                    Arc::new(InMemoryLeaveRequestRepository::new()) as BoxLeaveRequestRepository;
                let leave_type_repo =
                    Arc::new(InMemoryLeaveTypeRepository::new()) as BoxLeaveTypeRepository;
                let payroll_repo =
                    Arc::new(InMemoryPayrollRepository::new()) as BoxPayrollRepository;
                HrService::new(
                    employee_repo,
                    attendance_repo,
                    leave_request_repo,
                    leave_type_repo,
                    payroll_repo,
                )
            }
            StorageBackend::Postgres(pool) => {
                let employee_repo = PostgresEmployeeRepository::new(pool.clone()).into_boxed();
                let attendance_repo = PostgresAttendanceRepository::new(pool.clone()).into_boxed();
                let leave_request_repo =
                    PostgresLeaveRequestRepository::new(pool.clone()).into_boxed();
                let leave_type_repo = PostgresLeaveTypeRepository::new(pool.clone()).into_boxed();
                let payroll_repo = PostgresPayrollRepository::new(pool.clone()).into_boxed();
                HrService::new(
                    employee_repo,
                    attendance_repo,
                    leave_request_repo,
                    leave_type_repo,
                    payroll_repo,
                )
            }
        };

        // ---- SGK Payroll ----
        let sgk_payroll_service = match &backend {
            StorageBackend::InMemory => {
                let sgk_reg = Arc::new(InMemorySgkEmployeeRegistrationRepository::new())
                    as crate::domain::hr::sgk::repository::BoxSgkEmployeeRegistrationRepository;
                let sgk_cfg = Arc::new(InMemorySgkConfigRepository::new())
                    as crate::domain::hr::sgk::repository::BoxSgkConfigRepository;
                let bonus = Arc::new(InMemoryEmployeeBonusRepository::new())
                    as crate::domain::hr::sgk::repository::BoxEmployeeBonusRepository;
                crate::domain::hr::sgk::service::SgkPayrollService::new(
                    Arc::new(hr_service.clone()),
                    sgk_reg,
                    sgk_cfg,
                    bonus,
                )
            }
            StorageBackend::Postgres(pool) => {
                let sgk_reg =
                    PostgresSgkEmployeeRegistrationRepository::new(pool.clone()).into_boxed();
                let sgk_cfg = PostgresSgkConfigRepository::new(pool.clone()).into_boxed();
                let bonus = PostgresEmployeeBonusRepository::new(pool.clone()).into_boxed();
                crate::domain::hr::sgk::service::SgkPayrollService::new(
                    Arc::new(hr_service.clone()),
                    sgk_reg,
                    sgk_cfg,
                    bonus,
                )
            }
        };

        // ---- Accounting ----
        let accounting_service = match &backend {
            StorageBackend::InMemory => {
                let account_repo =
                    Arc::new(InMemoryAccountRepository::new()) as BoxAccountRepository;
                let entry_repo =
                    Arc::new(InMemoryJournalEntryRepository::new()) as BoxJournalEntryRepository;
                let line_repo =
                    Arc::new(InMemoryJournalLineRepository::new()) as BoxJournalLineRepository;
                AccountingService::new(account_repo, entry_repo, line_repo)
            }
            StorageBackend::Postgres(pool) => {
                let account_repo = PostgresAccountRepository::new(pool.clone()).into_boxed();
                let entry_repo = PostgresJournalEntryRepository::new(pool.clone()).into_boxed();
                let line_repo = PostgresJournalLineRepository::new(pool.clone()).into_boxed();
                AccountingService::new(account_repo, entry_repo, line_repo)
            }
        };

        // ---- Project ----
        let project_service = match &backend {
            StorageBackend::InMemory => {
                let project_repo =
                    Arc::new(InMemoryProjectRepository::new()) as BoxProjectRepository;
                let wbs_repo = Arc::new(InMemoryWbsItemRepository::new()) as BoxWbsItemRepository;
                let cost_repo =
                    Arc::new(InMemoryProjectCostRepository::new()) as BoxProjectCostRepository;
                ProjectService::new(project_repo, wbs_repo, cost_repo)
            }
            StorageBackend::Postgres(pool) => {
                let project_repo = PostgresProjectRepository::new(pool.clone()).into_boxed();
                let wbs_repo = PostgresWbsItemRepository::new(pool.clone()).into_boxed();
                let cost_repo = PostgresProjectCostRepository::new(pool.clone()).into_boxed();
                ProjectService::new(project_repo, wbs_repo, cost_repo)
            }
        };

        // ---- Manufacturing ----
        let manufacturing_service = match &backend {
            StorageBackend::InMemory => {
                let work_order =
                    Arc::new(InMemoryWorkOrderRepository::new()) as BoxWorkOrderRepository;
                let bom = Arc::new(InMemoryBillOfMaterialsRepository::new())
                    as BoxBillOfMaterialsRepository;
                let routing = Arc::new(InMemoryRoutingRepository::new()) as BoxRoutingRepository;
                ManufacturingService::new(work_order, bom, routing)
            }
            StorageBackend::Postgres(pool) => {
                let work_order = PostgresWorkOrderRepository::new(pool.clone()).into_boxed();
                let bom = PostgresBillOfMaterialsRepository::new(pool.clone()).into_boxed();
                let routing = PostgresRoutingRepository::new(pool.clone()).into_boxed();
                ManufacturingService::new(work_order, bom, routing)
            }
        };

        // ---- CRM ----
        let crm_service = match &backend {
            StorageBackend::InMemory => {
                let lead = Arc::new(InMemoryLeadRepository::new()) as BoxLeadRepository;
                let opp =
                    Arc::new(InMemoryOpportunityRepository::new()) as BoxOpportunityRepository;
                let camp = Arc::new(InMemoryCampaignRepository::new()) as BoxCampaignRepository;
                let ticket = Arc::new(InMemoryTicketRepository::new()) as BoxTicketRepository;
                CrmService::new(lead, opp, camp, ticket)
            }
            StorageBackend::Postgres(pool) => {
                let lead = PostgresLeadRepository::new(pool.clone()).into_boxed();
                let opp = PostgresOpportunityRepository::new(pool.clone()).into_boxed();
                let camp = PostgresCampaignRepository::new(pool.clone()).into_boxed();
                let ticket = PostgresTicketRepository::new(pool.clone()).into_boxed();
                CrmService::new(lead, opp, camp, ticket)
            }
        };

        // ---- Chart of Accounts ----
        let chart_of_accounts_service = match &backend {
            StorageBackend::InMemory => {
                let chart_repo =
                    Arc::new(InMemoryChartAccountRepository::new()) as BoxChartAccountRepository;
                ChartOfAccountsService::new(chart_repo)
            }
            StorageBackend::Postgres(pool) => {
                let chart_repo = PostgresChartAccountRepository::new(pool.clone()).into_boxed();
                ChartOfAccountsService::new(chart_repo)
            }
        };
        let chart_account_repo_import = match &backend {
            StorageBackend::InMemory => {
                Arc::new(InMemoryChartAccountRepository::new()) as BoxChartAccountRepository
            }
            StorageBackend::Postgres(pool) => {
                PostgresChartAccountRepository::new(pool.clone()).into_boxed()
            }
        };

        // ---- Custom Fields ----
        let custom_field_service = match &backend {
            StorageBackend::InMemory => {
                let repo =
                    Arc::new(InMemoryCustomFieldRepository::new()) as BoxCustomFieldRepository;
                CustomFieldService::new(repo)
            }
            StorageBackend::Postgres(pool) => {
                let repo = PostgresCustomFieldRepository::new(pool.clone()).into_boxed();
                CustomFieldService::new(repo)
            }
        };

        // ---- Tenant ----
        let tenant_service = match &backend {
            StorageBackend::InMemory => {
                let repo = Arc::new(InMemoryTenantRepository::new()) as BoxTenantRepository;
                TenantService::new(repo)
            }
            StorageBackend::Postgres(pool) => {
                let repo = PostgresTenantRepository::new(pool.clone()).into_boxed();
                TenantService::new(repo)
            }
        };
        let tenant_config_service = match &backend {
            StorageBackend::InMemory => {
                let repo = Arc::new(InMemoryTenantConfigRepository::new())
                    as crate::domain::tenant::repository::BoxTenantConfigRepository;
                TenantConfigService::new(repo).with_cache(cache_service.clone())
            }
            StorageBackend::Postgres(pool) => {
                let repo = PostgresTenantConfigRepository::new(pool.clone()).into_boxed();
                TenantConfigService::new(repo).with_cache(cache_service.clone())
            }
        };

        // ---- Assets ----
        let assets_service = match &backend {
            StorageBackend::InMemory => {
                let repo = Arc::new(InMemoryAssetsRepository::new()) as BoxAssetsRepository;
                AssetsService::new(repo)
            }
            StorageBackend::Postgres(pool) => {
                let repo = PostgresAssetsRepository::new(pool.clone());
                let _asset_category_repo = PostgresAssetCategoryRepository::new(pool.clone());
                AssetsService::new(Arc::new(repo) as Arc<dyn AssetsRepository>)
            }
        };

        // ---- Feature Flags ----
        let feature_service = match &backend {
            StorageBackend::InMemory => {
                let repo = Arc::new(InMemoryFeatureFlagRepository::new())
                    as Arc<dyn FeatureFlagRepository>;
                FeatureFlagService::new(repo)
            }
            StorageBackend::Postgres(pool) => {
                let repo = PostgresFeatureFlagRepository::new(pool.clone()).into_boxed();
                FeatureFlagService::new(repo)
            }
        };

        // ---- Product ----
        let product_service = match &backend {
            StorageBackend::InMemory => {
                let p_repo = Arc::new(InMemoryProductRepository::new()) as BoxProductRepository;
                let p_repo_import = p_repo.clone();
                let c_repo = Arc::new(InMemoryCategoryRepository::new()) as BoxCategoryRepository;
                let u_repo = Arc::new(InMemoryUnitRepository::new()) as BoxUnitRepository;
                let v_repo = Arc::new(InMemoryProductVariantRepository::new())
                    as BoxProductVariantRepository;
                let svc = ProductService::with_variants(p_repo, c_repo, u_repo, v_repo)
                    .with_cache(cache_service.clone());
                (svc, p_repo_import)
            }
            StorageBackend::Postgres(pool) => {
                let p_repo = PostgresProductRepository::new(pool.clone(), cache_service.clone())
                    .into_boxed();
                let p_repo_import = p_repo.clone();
                let c_repo = PostgresCategoryRepository::new(pool.clone()).into_boxed();
                let u_repo = PostgresUnitRepository::new(pool.clone()).into_boxed();
                let v_repo = PostgresProductVariantRepository::new(pool.clone()).into_boxed();
                let svc = ProductService::with_variants(p_repo, c_repo, u_repo, v_repo)
                    .with_cache(cache_service.clone());
                (svc, p_repo_import)
            }
        };
        let (product_service, product_repo_import) = product_service;

        // ---- Barcode ----
        let barcode_service = match &backend {
            StorageBackend::InMemory => {
                let repo = Arc::new(InMemoryBarcodeRepository::new()) as BoxBarcodeRepository;
                BarcodeService::new(repo)
            }
            StorageBackend::Postgres(pool) => {
                let repo = PostgresBarcodeRepository::new(pool.clone()).into_boxed();
                BarcodeService::new(repo)
            }
        };

        // ---- Purchase ----
        let purchase_service = match &backend {
            StorageBackend::InMemory => {
                let order =
                    Arc::new(InMemoryPurchaseOrderRepository::new()) as BoxPurchaseOrderRepository;
                let order_line = Arc::new(InMemoryPurchaseOrderLineRepository::new())
                    as BoxPurchaseOrderLineRepository;
                let receipt =
                    Arc::new(InMemoryGoodsReceiptRepository::new()) as BoxGoodsReceiptRepository;
                let receipt_line = Arc::new(InMemoryGoodsReceiptLineRepository::new())
                    as BoxGoodsReceiptLineRepository;
                let request = Arc::new(InMemoryPurchaseRequestRepository::new())
                    as BoxPurchaseRequestRepository;
                let request_line = Arc::new(InMemoryPurchaseRequestLineRepository::new())
                    as BoxPurchaseRequestLineRepository;
                PurchaseService::with_requests(
                    order,
                    order_line,
                    receipt,
                    receipt_line,
                    request,
                    request_line,
                )
            }
            StorageBackend::Postgres(pool) => {
                let order = PostgresPurchaseOrderRepository::new(pool.clone()).into_boxed();
                let order_line =
                    PostgresPurchaseOrderLineRepository::new(pool.clone()).into_boxed();
                let receipt = PostgresGoodsReceiptRepository::new(pool.clone()).into_boxed();
                let receipt_line =
                    PostgresGoodsReceiptLineRepository::new(pool.clone()).into_boxed();
                let request = PostgresPurchaseRequestRepository::new(pool.clone()).into_boxed();
                let request_line =
                    PostgresPurchaseRequestLineRepository::new(pool.clone()).into_boxed();
                PurchaseService::with_requests(
                    order,
                    order_line,
                    receipt,
                    receipt_line,
                    request,
                    request_line,
                )
            }
        };

        // ---- Audit ----
        let audit_service = match &backend {
            StorageBackend::InMemory => {
                let repo = Arc::new(InMemoryAuditLogRepository::new()) as BoxAuditLogRepository;
                AuditService::new(repo)
            }
            StorageBackend::Postgres(pool) => {
                let repo = PostgresAuditLogRepository::new(pool.clone()).into_boxed();
                AuditService::new(repo)
            }
        };

        // ---- Archive ----
        let archive_service = match &backend {
            StorageBackend::InMemory => {
                let policy =
                    Arc::new(InMemoryArchivePolicyRepository::new()) as BoxArchivePolicyRepository;
                let job = Arc::new(InMemoryArchiveJobRepository::new()) as BoxArchiveJobRepository;
                let record =
                    Arc::new(InMemoryArchiveRecordRepository::new()) as BoxArchiveRecordRepository;
                ArchiveService::new(policy, job, record)
            }
            StorageBackend::Postgres(pool) => {
                let policy = PostgresArchivePolicyRepository::new(pool.clone()).into_boxed();
                let job = PostgresArchiveJobRepository::new(pool.clone()).into_boxed();
                let record = PostgresArchiveRecordRepository::new(pool.clone()).into_boxed();
                ArchiveService::new(policy, job, record)
            }
        };

        // ---- Bank ----
        let bank_service = match &backend {
            StorageBackend::InMemory => {
                let repo = Arc::new(InMemoryBankRepository::new()) as BoxBankRepository;
                BankService::new(repo)
            }
            StorageBackend::Postgres(pool) => {
                let repo = PostgresBankRepository::new(pool.clone()).into_boxed();
                BankService::new(repo)
            }
        };

        // ---- Cost Centers ----
        let cost_center_service = match &backend {
            StorageBackend::InMemory => {
                let repo = Arc::new(InMemoryCostCenterRepository::new()) as BoxCostCenterRepository;
                CostCenterService::new(repo)
            }
            StorageBackend::Postgres(pool) => {
                let repo = PostgresCostCenterRepository::new(pool.clone()).into_boxed();
                CostCenterService::new(repo)
            }
        };

        // ---- Dashboard ----
        let dashboard_service = match &backend {
            StorageBackend::InMemory => {
                let repo = Arc::new(InMemoryDashboardRepository::new()) as BoxDashboardRepository;
                DashboardService::new(repo, cache_service.clone())
            }
            StorageBackend::Postgres(pool) => {
                let repo = PostgresDashboardRepository::new(pool.clone()).into_boxed();
                DashboardService::new(repo, cache_service.clone())
            }
        };

        // ---- Observability ----
        let observability_service = match &backend {
            StorageBackend::InMemory => {
                let repo =
                    Arc::new(InMemoryObservabilityRepository::new()) as BoxObservabilityRepository;
                ObservabilityService::new(repo, cache_service.clone())
            }
            StorageBackend::Postgres(pool) => {
                let repo = PostgresObservabilityRepository::new(pool.clone()).into_boxed();
                ObservabilityService::new(repo, cache_service.clone())
            }
        };

        // ---- Documents ----
        let document_service = match &backend {
            StorageBackend::InMemory => {
                let repo = Arc::new(InMemoryDocumentRepository::new()) as BoxDocumentRepository;
                DocumentService::new(repo)
            }
            StorageBackend::Postgres(pool) => {
                let repo = PostgresDocumentRepository::new(pool.clone()).into_boxed();
                DocumentService::new(repo)
            }
        };

        // ---- Subscriptions ----
        let subscription_service = match &backend {
            StorageBackend::InMemory => {
                let repo =
                    Arc::new(InMemorySubscriptionRepository::new()) as BoxSubscriptionRepository;
                SubscriptionService::new(repo)
            }
            StorageBackend::Postgres(pool) => {
                let repo = PostgresSubscriptionRepository::new(pool.clone()).into_boxed();
                SubscriptionService::new(repo)
            }
        };

        // ---- Forecasting ----
        let forecasting_service = match &backend {
            StorageBackend::InMemory => {
                let repo =
                    Arc::new(InMemoryForecastingRepository::new()) as BoxForecastingRepository;
                ForecastingService::new(repo)
            }
            StorageBackend::Postgres(pool) => {
                let repo = PostgresForecastingRepository::new(pool.clone()).into_boxed();
                ForecastingService::new(repo)
            }
        };

        // ---- Shift Planning ----
        let shift_service = match &backend {
            StorageBackend::InMemory => {
                let shift = Arc::new(InMemoryShiftRepository::new()) as BoxShiftRepository;
                let assignment = Arc::new(InMemoryShiftAssignmentRepository::new())
                    as BoxShiftAssignmentRepository;
                let attendance = Arc::new(InMemoryAttendanceRecordRepository::new())
                    as BoxAttendanceRecordRepository;
                ShiftService::new(shift, assignment, attendance)
            }
            StorageBackend::Postgres(pool) => {
                let shift = PostgresShiftRepository::new(pool.clone()).into_boxed();
                let assignment = PostgresShiftAssignmentRepository::new(pool.clone()).into_boxed();
                let attendance = PostgresAttendanceRecordRepository::new(pool.clone()).into_boxed();
                ShiftService::new(shift, assignment, attendance)
            }
        };

        // ---- Quality Control ----
        let qc_service = match &backend {
            StorageBackend::InMemory => {
                let inspection =
                    Arc::new(crate::domain::quality_control::InMemoryInspectionRepository::new())
                        as crate::domain::quality_control::BoxInspectionRepository;
                let ncr = Arc::new(crate::domain::quality_control::InMemoryNcrRepository::new())
                    as crate::domain::quality_control::BoxNcrRepository;
                crate::domain::quality_control::QualityControlService::new(inspection, ncr)
            }
            StorageBackend::Postgres(pool) => {
                let inspection = PostgresInspectionRepository::new(pool.clone()).into_boxed();
                let ncr = PostgresNcrRepository::new(pool.clone()).into_boxed();
                crate::domain::quality_control::QualityControlService::new(inspection, ncr)
            }
        };

        // ---- Settings ----
        let settings_service = match &backend {
            StorageBackend::InMemory => {
                let repo = Arc::new(crate::domain::settings::InMemorySettingsRepository::new())
                    as crate::domain::settings::BoxSettingsRepository;
                crate::domain::settings::SettingsService::new(repo)
            }
            StorageBackend::Postgres(pool) => {
                let repo = PostgresSettingsRepository::new(pool.clone()).into_boxed();
                crate::domain::settings::SettingsService::new(repo)
            }
        };

        // ---- API Keys ----
        let api_key_service = match &backend {
            StorageBackend::InMemory => {
                let repo = Arc::new(crate::domain::api_key::InMemoryApiKeyRepository::new())
                    as crate::domain::api_key::BoxApiKeyRepository;
                crate::domain::api_key::ApiKeyService::new(repo)
            }
            StorageBackend::Postgres(pool) => {
                let repo = PostgresApiKeyRepository::new(pool.clone()).into_boxed();
                crate::domain::api_key::ApiKeyService::new(repo)
            }
        };

        // ---- IP Whitelist ----
        let ip_whitelist_service = match &backend {
            StorageBackend::InMemory => {
                let repo =
                    Arc::new(crate::domain::ip_whitelist::InMemoryIpWhitelistRepository::new())
                        as crate::domain::ip_whitelist::BoxIpWhitelistRepository;
                crate::domain::ip_whitelist::IpWhitelistService::new(repo)
            }
            StorageBackend::Postgres(pool) => {
                let repo = PostgresIpWhitelistRepository::new(pool.clone()).into_boxed();
                crate::domain::ip_whitelist::IpWhitelistService::new(repo)
            }
        };

        // ---- Notification Service ----
        let job_scheduler: Arc<dyn JobScheduler> = match &backend {
            StorageBackend::InMemory => {
                Arc::new(InMemoryJobScheduler::new()) as Arc<dyn JobScheduler>
            }
            StorageBackend::Postgres(pool) => {
                Arc::new(db::job_repository::PostgresJobScheduler::new(pool.clone()))
                    as Arc<dyn JobScheduler>
            }
        };

        let event_bus = Arc::new(InMemoryEventBus::new()) as Arc<dyn EventBus>;

        let notification_service: Arc<dyn NotificationService> = match &backend {
            StorageBackend::InMemory => {
                let n_repo = Arc::new(
                    crate::domain::notification::repository::InMemoryNotificationRepository::new(),
                )
                    as crate::domain::notification::repository::BoxNotificationRepository;
                let i_repo = Arc::new(crate::domain::notification::repository::InMemoryInAppNotificationRepository::new())
                    as crate::domain::notification::repository::BoxInAppNotificationRepository;
                let p_repo = Arc::new(crate::domain::notification::repository::InMemoryNotificationPreferenceRepository::new())
                    as crate::domain::notification::repository::BoxNotificationPreferenceRepository;
                Arc::new(
                    crate::domain::notification::service::NotificationService::with_noop_providers(
                        n_repo,
                        i_repo,
                        p_repo,
                        job_scheduler.clone(),
                    ),
                ) as Arc<dyn NotificationService>
            }
            StorageBackend::Postgres(pool) => {
                let n_repo = crate::domain::notification::postgres_repository::PostgresNotificationRepository::new(pool.clone()).into_boxed();
                let i_repo = crate::domain::notification::postgres_repository::PostgresInAppNotificationRepository::new(pool.clone()).into_boxed();
                let p_repo = crate::domain::notification::postgres_repository::PostgresNotificationPreferenceRepository::new(pool.clone()).into_boxed();
                Arc::new(
                    crate::domain::notification::service::NotificationService::with_noop_providers(
                        n_repo,
                        i_repo,
                        p_repo,
                        job_scheduler.clone(),
                    ),
                ) as Arc<dyn NotificationService>
            }
        };

        // ---- Webhooks ----
        let webhook_service = match &backend {
            StorageBackend::InMemory => {
                let repo = Arc::new(InMemoryWebhookRepository::new(
                    config.encryption_key_bytes()?,
                )) as BoxWebhookRepository;
                let delivery = Arc::new(InMemoryWebhookDeliveryRepository::new())
                    as BoxWebhookDeliveryRepository;
                WebhookService::new(repo, delivery)
            }
            StorageBackend::Postgres(pool) => {
                let repo =
                    PostgresWebhookRepository::new(pool.clone(), config.encryption_key_bytes()?)
                        .into_boxed();
                let delivery = PostgresWebhookDeliveryRepository::new(pool.clone()).into_boxed();
                WebhookService::new(repo, delivery)
            }
        };

        // Register webhook subscriber on event bus
        event_bus
            .subscribe(Arc::new(
                crate::domain::webhook::subscriber::WebhookSubscriber::new(Arc::new(
                    webhook_service.clone(),
                )),
            ))
            .await
            .ok();

        // Register business metrics subscribers on event bus
        let metrics_recorder = crate::common::business_metrics::BusinessMetricsRecorder::new();
        event_bus
            .subscribe(Arc::new(
                crate::common::business_metrics::InstrumentedEventSubscriber::new(
                    Arc::new(crate::common::AccountingEntrySubscriber),
                    metrics_recorder.clone(),
                ),
            ))
            .await
            .ok();
        event_bus
            .subscribe(Arc::new(
                crate::common::business_metrics::InstrumentedEventSubscriber::new(
                    Arc::new(crate::common::StockDecrementSubscriber),
                    metrics_recorder.clone(),
                ),
            ))
            .await
            .ok();
        event_bus
            .subscribe(Arc::new(
                crate::common::business_metrics::InstrumentedEventSubscriber::new(
                    Arc::new(crate::common::TaxPeriodSubscriber),
                    metrics_recorder.clone(),
                ),
            ))
            .await
            .ok();

        // ---- Report Engine, Tracing, DB Router (always in-memory) ----
        let report_engine = Arc::new(InMemoryReportEngine::new()) as Arc<dyn ReportEngine>;
        let tracing_service =
            Arc::new(InMemoryTracingService::new("turerp-erp")) as Arc<dyn TracingService>;
        let db_router = Arc::new(InMemoryDbRouter::new(
            "localhost:5432/turerp",
            ReadAfterWriteMode::Eventual,
        )) as Arc<dyn DbRouter>;

        // ---- Tax ----
        let tax_service = match &backend {
            StorageBackend::InMemory => {
                let r = Arc::new(InMemoryTaxRateRepository::new()) as BoxTaxRateRepository;
                let p = Arc::new(InMemoryTaxPeriodRepository::new()) as BoxTaxPeriodRepository;
                TaxService::new(r, p)
            }
            StorageBackend::Postgres(pool) => {
                let r = PostgresTaxRateRepository::new(pool.clone()).into_boxed();
                let p = PostgresTaxPeriodRepository::new(pool.clone()).into_boxed();
                TaxService::new(r, p)
            }
        };

        // ---- Currency ----
        let currency_service = match &backend {
            StorageBackend::InMemory => {
                let c = Arc::new(InMemoryCurrencyRepository::new()) as BoxCurrencyRepository;
                let e =
                    Arc::new(InMemoryExchangeRateRepository::new()) as BoxExchangeRateRepository;
                CurrencyService::new(c, e)
            }
            StorageBackend::Postgres(pool) => {
                let c = PostgresCurrencyRepository::new(pool.clone()).into_boxed();
                let e = PostgresExchangeRateRepository::new(pool.clone()).into_boxed();
                CurrencyService::new(c, e)
            }
        };

        // ---- e-Fatura ----
        let efatura_service = match &backend {
            StorageBackend::InMemory => {
                let repo = Arc::new(crate::domain::efatura::InMemoryEFaturaRepository::new())
                    as crate::domain::efatura::BoxEFaturaRepository;
                let gib_gateway = Arc::new(crate::common::InMemoryGibGateway::new())
                    as crate::common::BoxGibGateway;
                crate::domain::efatura::EFaturaService::new(repo, gib_gateway)
            }
            StorageBackend::Postgres(pool) => {
                let repo = PostgresEFaturaRepository::new(pool.clone()).into_boxed();
                let gib_gateway = Arc::new(crate::common::InMemoryGibGateway::new())
                    as crate::common::BoxGibGateway;
                crate::domain::efatura::EFaturaService::new(repo, gib_gateway)
            }
        };

        // ---- e-Archive ----
        let earchive_service = match &backend {
            StorageBackend::InMemory => {
                let repo = Arc::new(crate::domain::earchive::InMemoryEarchiveRepository::new())
                    as crate::domain::earchive::BoxEarchiveRepository;
                crate::domain::earchive::EarchiveService::new(repo)
            }
            StorageBackend::Postgres(pool) => {
                let repo = PostgresEarchiveRepository::new(pool.clone()).into_boxed();
                crate::domain::earchive::EarchiveService::new(repo)
            }
        };

        // ---- e-Defter ----
        let edefter_service = match &backend {
            StorageBackend::InMemory => {
                let repo = Arc::new(crate::domain::edefter::InMemoryEDefterRepository::new())
                    as crate::domain::edefter::BoxEDefterRepository;
                crate::domain::edefter::EDefterService::new(repo)
            }
            StorageBackend::Postgres(pool) => {
                let repo = PostgresEDefterRepository::new(pool.clone()).into_boxed();
                crate::domain::edefter::EDefterService::new(repo)
            }
        };

        // ---- Blockchain Ledger ----
        let blockchain_ledger_service = match &backend {
            StorageBackend::InMemory => {
                let repo = Arc::new(
                    crate::domain::edefter::blockchain::InMemoryBlockchainLedgerRepository::new(),
                )
                    as crate::domain::edefter::blockchain::BoxBlockchainLedgerRepository;
                crate::domain::edefter::blockchain::BlockchainLedgerService::new(repo)
            }
            StorageBackend::Postgres(pool) => {
                let repo = PostgresBlockchainLedgerRepository::new(pool.clone()).into_boxed();
                crate::domain::edefter::blockchain::BlockchainLedgerService::new(repo)
            }
        };

        // ---- Customer Portal ----
        let customer_portal_service: BoxCustomerPortal = match &backend {
            StorageBackend::InMemory => {
                let p_user = Arc::new(InMemoryPortalUserRepository::new())
                    as crate::domain::customer_portal::BoxPortalUserRepository;
                let p_ticket = Arc::new(InMemorySupportTicketRepository::new())
                    as crate::domain::customer_portal::BoxSupportTicketRepository;
                Arc::new(CustomerPortalService::new(
                    p_user,
                    p_ticket,
                    Arc::new(cari_service.clone()),
                    Arc::new(sales_service.clone()),
                    Arc::new(invoice_service.clone()),
                    Arc::new(jwt_service.clone()),
                    config.jwt.access_token_expiration / 3600,
                ))
            }
            StorageBackend::Postgres(pool) => {
                let p_user = PostgresPortalUserRepository::new(pool.clone()).into_boxed();
                let p_ticket = PostgresSupportTicketRepository::new(pool.clone()).into_boxed();
                Arc::new(CustomerPortalService::new(
                    p_user,
                    p_ticket,
                    Arc::new(cari_service.clone()),
                    Arc::new(sales_service.clone()),
                    Arc::new(invoice_service.clone()),
                    Arc::new(jwt_service.clone()),
                    config.jwt.access_token_expiration / 3600,
                ))
            }
        };

        // ---- Vendor Portal ----
        let vendor_portal_service: BoxVendorPortal = match &backend {
            StorageBackend::InMemory => {
                let v_user = Arc::new(InMemoryVendorUserRepository::new())
                    as crate::domain::vendor_portal::BoxVendorUserRepository;
                let v_note = Arc::new(InMemoryDeliveryNoteRepository::new())
                    as crate::domain::vendor_portal::BoxDeliveryNoteRepository;
                Arc::new(VendorPortalService::new(
                    v_user,
                    v_note,
                    Arc::new(cari_service.clone()),
                    Arc::new(purchase_service.clone()),
                    Arc::new(invoice_service.clone()),
                    Arc::new(jwt_service.clone()),
                    config.jwt.access_token_expiration / 3600,
                ))
            }
            StorageBackend::Postgres(pool) => {
                let v_user = PostgresVendorUserRepository::new(pool.clone()).into_boxed();
                let v_note = PostgresDeliveryNoteRepository::new(pool.clone()).into_boxed();
                Arc::new(VendorPortalService::new(
                    v_user,
                    v_note,
                    Arc::new(cari_service.clone()),
                    Arc::new(purchase_service.clone()),
                    Arc::new(invoice_service.clone()),
                    Arc::new(jwt_service.clone()),
                    config.jwt.access_token_expiration / 3600,
                ))
            }
        };

        // ---- Search ----
        let search_service: Arc<dyn SearchService> = match &backend {
            StorageBackend::InMemory => {
                Arc::new(InMemorySearchService::new()) as Arc<dyn SearchService>
            }
            StorageBackend::Postgres(pool) => {
                if config.database.url.is_empty() {
                    Arc::new(InMemorySearchService::new()) as Arc<dyn SearchService>
                } else {
                    Arc::new(PostgresSearchService::new(pool.clone())) as Arc<dyn SearchService>
                }
            }
        };

        // ---- File Storage ----
        let file_storage: Arc<dyn crate::common::file_storage::FileStorage> = Arc::new(
            crate::common::file_storage::LocalFileStorage::new(format!(
                "/tmp/turerp-test-files-{}",
                std::process::id()
            ))
            .await,
        )
            as Arc<dyn crate::common::file_storage::FileStorage>;

        // ---- Import Service ----
        let import_service: Arc<dyn crate::common::import::ImportService> =
            Arc::new(crate::common::import::CsvImportService::new(
                product_repo_import,
                cari_repo_import,
                chart_account_repo_import,
                stock_movement_repo_import,
                job_scheduler.clone(),
            ));

        // ---- Inter-Company ----
        let inter_company_service = match &backend {
            StorageBackend::InMemory => {
                let repo = Arc::new(
                    crate::domain::inter_company::repository::InMemoryInterCompanyRepository::new(),
                )
                    as crate::domain::inter_company::repository::BoxInterCompanyRepository;
                crate::domain::inter_company::service::InterCompanyService::new(
                    Arc::new(company_service.clone()),
                    Arc::new(invoice_service.clone()),
                    Arc::new(stock_service.clone()),
                    Arc::new(product_service.clone()),
                    repo,
                )
            }
            StorageBackend::Postgres(pool) => {
                let repo =
                    crate::domain::inter_company::PostgresInterCompanyRepository::new(pool.clone())
                        .into_boxed();
                crate::domain::inter_company::service::InterCompanyService::new(
                    Arc::new(company_service.clone()),
                    Arc::new(invoice_service.clone()),
                    Arc::new(stock_service.clone()),
                    Arc::new(product_service.clone()),
                    repo,
                )
            }
        };

        // ---- Workflow ----
        let workflow_service = match &backend {
            StorageBackend::InMemory => {
                let repo = Arc::new(InMemoryWorkflowRepository::new()) as BoxWorkflowRepository;
                WorkflowService::new(repo, notification_service.clone(), job_scheduler.clone())
            }
            StorageBackend::Postgres(pool) => {
                let repo = PostgresWorkflowRepository::new(pool.clone()).into_boxed();
                WorkflowService::new(repo, notification_service.clone(), job_scheduler.clone())
            }
        };

        // ---- LDAP ----
        let ldap_service = match &backend {
            StorageBackend::InMemory => {
                let repo = Arc::new(InMemoryLdapConfigRepository::new()) as BoxLdapConfigRepository;
                LdapSyncService::new(
                    repo,
                    Arc::new(user_service.clone()),
                    config.encryption_key_bytes()?,
                )
            }
            StorageBackend::Postgres(pool) => {
                let repo = PostgresLdapConfigRepository::new(pool.clone()).into_boxed();
                LdapSyncService::new(
                    repo,
                    Arc::new(user_service.clone()),
                    config.encryption_key_bytes()?,
                )
            }
        };

        // ---- Infrastructure ----
        let rate_limit_stats = crate::middleware::rate_limit::RateLimitStatsStore::default();
        let circuit_breaker_registry = CircuitBreakerRegistry::new();
        let retry_stats: BoxRetryStats = Arc::new(crate::common::retry::RetryStats::new());

        let i18n = I18n::init().await;

        // ---- Observability: attach notification service ----
        let observability_service =
            observability_service.with_notification(notification_service.clone());

        // ---- DB pool field ----
        let db_pool_field = pg_pool
            .as_ref()
            .map(|p| web::Data::new(p.clone()) as web::Data<Arc<PgPool>>);

        let schema = crate::graphql::create_schema(config.graphql_introspection);

        Ok(AppState {
            auth: crate::app::AuthState {
                auth_service: web::Data::new(auth_service),
                user_service: web::Data::new(user_service),
                jwt_service: web::Data::new(jwt_service),
                mfa_service: web::Data::new(mfa_service),
            },
            commerce: crate::app::CommerceState {
                cari_service: web::Data::new(cari_service),
                company_service: web::Data::new(company_service),
                stock_service: web::Data::new(stock_service),
                invoice_service: web::Data::new(invoice_service),
                sales_service: web::Data::new(sales_service),
                purchase_service: web::Data::new(purchase_service),
                product_service: web::Data::new(product_service),
                barcode_service: web::Data::new(barcode_service),
                inter_company_service: web::Data::new(inter_company_service),
            },
            hr: crate::app::HrState {
                hr_service: web::Data::new(hr_service),
                shift_service: web::Data::new(shift_service),
                sgk_payroll_service: web::Data::new(sgk_payroll_service),
            },
            admin: crate::app::AdminState {
                tenant_service: web::Data::new(tenant_service),
                tenant_config_service: web::Data::new(tenant_config_service),
                settings_service: web::Data::new(settings_service),
                api_key_service: web::Data::new(api_key_service),
                ip_whitelist_service: web::Data::new(ip_whitelist_service),
            },
            infra: crate::app::InfraState {
                job_scheduler: web::Data::from(job_scheduler),
                event_bus: web::Data::from(event_bus),
                notification_service: web::Data::from(notification_service),
                report_engine: web::Data::from(report_engine),
                tracing_service: web::Data::from(tracing_service),
                db_router: web::Data::from(db_router),
                cache_service: web::Data::from(cache_service),
                search_service: web::Data::from(search_service),
                rate_limit_stats: web::Data::new(rate_limit_stats),
                db_pool: db_pool_field,
                cdc_listener,
                import_service: web::Data::from(import_service),
                circuit_breaker_registry: web::Data::new(circuit_breaker_registry),
                retry_stats: web::Data::new(retry_stats),
            },
            finance: crate::app::FinanceState {
                accounting_service: web::Data::new(accounting_service),
                bank_service: web::Data::new(bank_service),
                cost_center_service: web::Data::new(cost_center_service),
                tax_service: web::Data::new(tax_service),
                currency_service: web::Data::new(currency_service),
            },
            project: crate::app::ProjectState {
                project_service: web::Data::new(project_service),
                manufacturing_service: web::Data::new(manufacturing_service),
                crm_service: web::Data::new(crm_service),
                qc_service: web::Data::new(qc_service),
            },
            document: crate::app::DocumentState {
                document_service: web::Data::new(document_service),
                file_storage: web::Data::from(file_storage),
                dashboard_service: web::Data::new(dashboard_service),
            },
            integration: crate::app::IntegrationState {
                efatura_service: web::Data::new(efatura_service),
                earchive_service: web::Data::new(earchive_service),
                edefter_service: web::Data::new(edefter_service),
                blockchain_ledger_service: web::Data::new(blockchain_ledger_service),
                customer_portal_service: web::Data::from(customer_portal_service),
                vendor_portal_service: web::Data::from(vendor_portal_service),
                webhook_service: web::Data::new(webhook_service),
                workflow_service: web::Data::new(workflow_service),
            },
            analytics: crate::app::AnalyticsState {
                audit_service: web::Data::new(audit_service),
                archive_service: web::Data::new(archive_service),
                subscription_service: web::Data::new(subscription_service),
                forecasting_service: web::Data::new(forecasting_service),
            },
            chart_of_accounts_service: web::Data::new(chart_of_accounts_service),
            custom_field_service: web::Data::new(custom_field_service),
            assets_service: web::Data::new(assets_service),
            feature_service: web::Data::new(feature_service),
            observability_service: web::Data::new(observability_service),
            ldap_service: web::Data::new(ldap_service),
            i18n: web::Data::new(i18n),
            schema,
        })
    }
}
