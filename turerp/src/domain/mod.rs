//! Domain layer

pub mod accounting;
pub mod api_key;
pub mod archive;
pub mod assets;
pub mod audit;
pub mod auth;
pub mod bank;
pub mod cari;
pub mod chart_of_accounts;
pub mod company;
pub mod cost_center;
pub mod crm;
pub mod currency;
pub mod custom_field;
pub mod dashboard;
pub mod document;
pub mod edefter;
pub mod efatura;
pub mod feature;
pub mod file;
pub mod forecasting;
pub mod hr;
pub mod invoice;
pub mod ip_whitelist;
pub mod job;
pub mod ldap;
pub mod manufacturing;
pub mod mfa;
pub mod notification;
pub mod observability;
pub mod product;
pub mod project;
pub mod purchase;
pub mod sales;
pub mod settings;
pub mod shift;
pub mod stock;
pub mod subscription;
pub mod tax;
pub mod tenant;
pub mod user;
pub mod webhook;
pub mod workflow;

// Re-exports with explicit naming to avoid ambiguity
pub use auth::AuthService;

// Accounting module re-exports
pub use accounting::model::{
    Account, AccountBalance, AccountSubType, AccountType, CreateAccount, CreateJournalEntry,
    CreateJournalLine, JournalEntry, JournalEntryStatus, JournalLine, TrialBalance,
};
pub use accounting::repository::{
    AccountRepository, BoxAccountRepository, BoxJournalEntryRepository, BoxJournalLineRepository,
    InMemoryAccountRepository, InMemoryJournalEntryRepository, InMemoryJournalLineRepository,
    JournalEntryRepository, JournalLineRepository,
};
pub use accounting::service::AccountingService;

// Project module re-exports
pub use project::model::{
    CostType, CreateProject, CreateProjectCost, CreateWbsItem, Project, ProjectCost,
    ProjectProfitability, ProjectStatus, WbsItem,
};
pub use project::repository::{
    BoxProjectCostRepository, BoxProjectRepository, BoxWbsItemRepository,
    InMemoryProjectRepository, ProjectRepository, WbsItemRepository,
};
pub use project::service::ProjectService;

// Cari module re-exports
pub use cari::model::{Cari, CariResponse, CariStatus, CariType, CreateCari, UpdateCari};
pub use cari::repository::{BoxCariRepository, CariRepository, InMemoryCariRepository};
pub use cari::service::CariService;

// Chart of Accounts module re-exports
pub use chart_of_accounts::model::{
    AccountGroup, AccountTreeNode, ChartAccount, ChartAccountResponse, CreateChartAccount,
    TrialBalanceEntry, UpdateChartAccount,
};
pub use chart_of_accounts::repository::{
    BoxChartAccountRepository, ChartAccountRepository, InMemoryChartAccountRepository,
};
pub use chart_of_accounts::service::ChartOfAccountsService;

// HR module re-exports
pub use hr::model::{
    Attendance, AttendanceStatus, CreateAttendance, CreateEmployee, CreateLeaveRequest, Employee,
    EmployeeResponse, EmployeeStatus, LeaveRequest, LeaveRequestStatus, LeaveType, Payroll,
    PayrollStatus,
};
pub use hr::repository::{
    AttendanceRepository, BoxAttendanceRepository, BoxEmployeeRepository,
    BoxLeaveRequestRepository, BoxLeaveTypeRepository, BoxPayrollRepository, EmployeeRepository,
    InMemoryEmployeeRepository, LeaveRequestRepository, LeaveTypeRepository, PayrollRepository,
};
pub use hr::service::HrService;
pub use hr::sgk::model::{
    CreateEmployeeBonus, CreateSgkConfig, CreateSgkEmployeeRegistration, EmployeeBonus,
    IncomeTaxBracket, MaritalStatus, SgkConfig, SgkEmployeeRegistration, SgkPayrollLineItem,
    SgkPayrollSummary, UpdateSgkConfig,
};
pub use hr::sgk::repository::{
    BoxEmployeeBonusRepository, BoxSgkConfigRepository, BoxSgkEmployeeRegistrationRepository,
    EmployeeBonusRepository, InMemoryEmployeeBonusRepository, InMemorySgkConfigRepository,
    InMemorySgkEmployeeRegistrationRepository, SgkConfigRepository,
    SgkEmployeeRegistrationRepository,
};

// Invoice module re-exports
pub use invoice::model::{
    CreateInvoice, CreateInvoiceLine, CreatePayment, Invoice, InvoiceLine, InvoiceResponse,
    InvoiceStatus, InvoiceType, Payment,
};
pub use invoice::repository::{
    BoxInvoiceLineRepository, BoxInvoiceRepository, BoxPaymentRepository,
    InMemoryInvoiceLineRepository, InMemoryInvoiceRepository, InMemoryPaymentRepository,
    InvoiceLineRepository, InvoiceRepository, PaymentRepository,
};
pub use invoice::service::InvoiceService;

// Manufacturing module re-exports
pub use manufacturing::model::{
    BillOfMaterials, BillOfMaterialsLine, CreateBillOfMaterials, CreateBillOfMaterialsLine,
    CreateRouting, CreateRoutingOperation, CreateWorkOrder, CreateWorkOrderMaterial,
    CreateWorkOrderOperation, Inspection, InspectionStatus, NcrStatus, NcrType,
    NonConformanceReport, Routing, RoutingOperation, WorkOrder, WorkOrderMaterial,
    WorkOrderOperation, WorkOrderPriority, WorkOrderStatus,
};
pub use manufacturing::repository::{
    BillOfMaterialsRepository, BoxBillOfMaterialsRepository, BoxRoutingRepository,
    BoxWorkOrderRepository, InMemoryBillOfMaterialsRepository, InMemoryRoutingRepository,
    InMemoryWorkOrderRepository, RoutingRepository, WorkOrderRepository,
};
pub use manufacturing::service::ManufacturingService;

// Product module re-exports
pub use product::model::{
    Category, CreateCategory, CreateProduct, CreateUnit, Product, ProductResponse, Unit,
    UpdateProduct,
};
pub use product::repository::{
    BoxCategoryRepository, BoxProductRepository, BoxUnitRepository, CategoryRepository,
    InMemoryProductRepository, ProductRepository, UnitRepository,
};
pub use product::service::ProductService;

// Purchase module re-exports
pub use purchase::model::{
    CreateGoodsReceipt, CreateGoodsReceiptLine, CreatePurchaseOrder, CreatePurchaseOrderLine,
    GoodsReceipt, GoodsReceiptLine, GoodsReceiptResponse, GoodsReceiptStatus, PurchaseOrder,
    PurchaseOrderLine, PurchaseOrderResponse, PurchaseOrderStatus,
};
pub use purchase::repository::{
    BoxGoodsReceiptLineRepository, BoxGoodsReceiptRepository, BoxPurchaseOrderLineRepository,
    BoxPurchaseOrderRepository, GoodsReceiptLineRepository, GoodsReceiptRepository,
    InMemoryPurchaseOrderRepository, PurchaseOrderLineRepository, PurchaseOrderRepository,
};
pub use purchase::service::PurchaseService;

// Sales module re-exports
pub use sales::model::{
    CreateQuotation, CreateQuotationLine, CreateSalesOrder, CreateSalesOrderLine, Quotation,
    QuotationLine, QuotationResponse, QuotationStatus, SalesOrder, SalesOrderLine,
    SalesOrderResponse, SalesOrderStatus,
};
pub use sales::repository::{
    BoxQuotationLineRepository, BoxQuotationRepository, BoxSalesOrderLineRepository,
    BoxSalesOrderRepository, InMemorySalesOrderRepository, QuotationLineRepository,
    QuotationRepository, SalesOrderLineRepository, SalesOrderRepository,
};
pub use sales::service::SalesService;

// CRM module re-exports
pub use crm::model::{
    Campaign, CampaignStatus, CreateCampaign, CreateLead, CreateOpportunity, CreateTicket, Lead,
    LeadStatus, Opportunity, OpportunityStatus, Ticket, TicketPriority, TicketStatus,
};
pub use crm::repository::{
    BoxCampaignRepository, BoxLeadRepository, BoxOpportunityRepository, BoxTicketRepository,
    CampaignRepository, InMemoryCampaignRepository, InMemoryLeadRepository,
    InMemoryOpportunityRepository, InMemoryTicketRepository, LeadRepository, OpportunityRepository,
    TicketRepository,
};
pub use crm::service::CrmService;

// Currency module re-exports
pub use currency::model::{
    ConversionResult, CreateCurrency, CreateExchangeRate, Currency, CurrencyResponse, ExchangeRate,
    ExchangeRateResponse, UpdateCurrency, UpdateExchangeRate,
};
pub use currency::repository::{
    BoxCurrencyRepository, BoxExchangeRateRepository, CurrencyRepository, ExchangeRateRepository,
    InMemoryCurrencyRepository, InMemoryExchangeRateRepository,
};
pub use currency::service::CurrencyService;

// Stock module re-exports
pub use stock::model::{
    CreateStockMovement, CreateWarehouse, MovementType, StockLevel, StockMovement, StockSummary,
    Warehouse, WarehouseStock,
};
pub use stock::repository::{
    BoxStockLevelRepository, BoxStockMovementRepository, BoxWarehouseRepository,
    InMemoryStockLevelRepository, InMemoryStockMovementRepository, InMemoryWarehouseRepository,
    StockLevelRepository, StockMovementRepository, WarehouseRepository,
};
pub use stock::service::StockService;

// Forecasting module re-exports
pub use forecasting::model::{
    DemandDataPoint, DemandForecast, ForecastPeriod, ForecastReport, ForecastRequest,
    ReorderRequest, ReorderSuggestion, ReorderUrgency, StockAlert, StockAlertRequest,
    StockAlertType,
};
pub use forecasting::repository::{
    BoxForecastingRepository, ForecastProduct, ForecastingRepository, HistoricalSale,
    InMemoryForecastingRepository,
};
pub use forecasting::service::ForecastingService;

// IP Whitelist module re-exports
pub use ip_whitelist::model::{
    CreateIpWhitelistEntry, IpWhitelistCheckResult, IpWhitelistEntry, IpWhitelistEntryResponse,
    UpdateIpWhitelistEntry,
};
pub use ip_whitelist::repository::{
    BoxIpWhitelistRepository, InMemoryIpWhitelistRepository, IpWhitelistRepository,
};
pub use ip_whitelist::service::IpWhitelistService;

// Tenant module re-exports
pub use tenant::model::{generate_db_name, CreateTenant, Tenant, UpdateTenant};
pub use tenant::repository::{BoxTenantRepository, InMemoryTenantRepository, TenantRepository};
pub use tenant::service::TenantService;

// User module re-exports
pub use user::model::{CreateUser, Role, UpdateUser, User, UserResponse};
pub use user::repository::{
    BoxUserRepository, InMemoryUserRepository, RepositoryError, UserRepository,
};
pub use user::service::UserService;

// Audit module re-exports
pub use audit::model::{AuditLog, AuditLogQueryParams};
pub use audit::repository::{
    AuditLogRepository, BoxAuditLogRepository, InMemoryAuditLogRepository,
};
pub use audit::service::AuditService;

// Feature module re-exports
pub use feature::model::{
    CreateFeatureFlag, FeatureFlag, FeatureFlagResponse, FeatureFlagStatus, UpdateFeatureFlag,
};
pub use feature::repository::{FeatureFlagRepository, InMemoryFeatureFlagRepository};
pub use feature::service::FeatureFlagService;

// Settings module re-exports
pub use settings::model::{
    BulkUpdateSettingItem, BulkUpdateSettings, CreateSetting, Setting, SettingDataType,
    SettingGroup, SettingResponse, UpdateSetting,
};
pub use settings::repository::{
    BoxSettingsRepository, InMemorySettingsRepository, SettingsRepository,
};
pub use settings::service::SettingsService;

// Shift module re-exports
pub use shift::model::{
    AttendanceRecord, AttendanceRecordResponse, ClockInRequest, ClockOutRequest, CreateShift,
    CreateShiftAssignment, OvertimeCalculation, Shift, ShiftAssignment, ShiftReport,
    ShiftReportQuery, ShiftResponse, ShiftType, UpdateShift,
};
pub use shift::repository::{
    AttendanceRecordRepository, BoxAttendanceRecordRepository, BoxShiftAssignmentRepository,
    BoxShiftRepository, InMemoryAttendanceRecordRepository, InMemoryShiftAssignmentRepository,
    InMemoryShiftRepository, ShiftAssignmentRepository, ShiftRepository,
};
pub use shift::service::ShiftService;

// API Key module re-exports
pub use api_key::model::{
    ApiKey, ApiKeyCreationResult, ApiKeyResponse, ApiKeyScope, CreateApiKey, UpdateApiKey,
};
pub use api_key::repository::{ApiKeyRepository, BoxApiKeyRepository, InMemoryApiKeyRepository};
pub use api_key::service::ApiKeyService;

// Archive module re-exports
pub use archive::model::{
    ArchiveJob, ArchiveJobResponse, ArchiveJobStatus, ArchivePolicy, ArchivePolicyResponse,
    ArchiveRecord, ArchiveRecordResponse, BulkRestoreFailed, BulkRestoreResponse, CreateArchiveJob,
    CreateArchivePolicy, RestoreRequest, UpdateArchivePolicy,
};
pub use archive::repository::{
    ArchiveJobRepository, ArchivePolicyRepository, ArchiveRecordRepository,
    BoxArchiveJobRepository, BoxArchivePolicyRepository, BoxArchiveRecordRepository,
    InMemoryArchiveJobRepository, InMemoryArchivePolicyRepository, InMemoryArchiveRecordRepository,
};
pub use archive::service::ArchiveService;

// Custom Field module re-exports
pub use custom_field::model::{
    CreateCustomFieldDefinition, CustomFieldDefinition, CustomFieldDefinitionResponse,
    CustomFieldModule, CustomFieldType, CustomFieldValues, UpdateCustomFieldDefinition,
};
pub use custom_field::repository::{
    BoxCustomFieldRepository, CustomFieldRepository, InMemoryCustomFieldRepository,
};
pub use custom_field::service::CustomFieldService;

// Tax module re-exports
pub use tax::model::{
    CreateTaxPeriod, CreateTaxRate, TaxCalculationResult, TaxPeriod, TaxPeriodDetail,
    TaxPeriodResponse, TaxPeriodStatus, TaxRate, TaxRateResponse, TaxType, UpdateTaxRate,
};
pub use tax::repository::{
    BoxTaxPeriodRepository, BoxTaxRateRepository, InMemoryTaxPeriodRepository,
    InMemoryTaxRateRepository, TaxPeriodRepository, TaxRateRepository,
};
pub use tax::service::TaxService;

// e-Fatura module re-exports
pub use efatura::model::{
    AddressInfo, CreateEFatura, EFatura, EFaturaLine, EFaturaProfile, EFaturaResponse,
    EFaturaStatus, MonetaryTotal, PartyInfo, TaxSubtotal,
};
pub use efatura::repository::{BoxEFaturaRepository, EFaturaRepository, InMemoryEFaturaRepository};
pub use efatura::service::EFaturaService;

// e-Defter module re-exports
pub use edefter::gib::{generate_berat_xml, generate_buyuk_defter_xml, generate_yevmiye_xml};
pub use edefter::model::{
    BalanceCheckResult, BeratInfo, CreateLedgerPeriod, EDefterStatus, LedgerPeriod,
    LedgerPeriodResponse, LedgerType, YevmiyeEntry, YevmiyeLine,
};
pub use edefter::repository::{BoxEDefterRepository, EDefterRepository, InMemoryEDefterRepository};
pub use edefter::service::EDefterService;

// Webhook module re-exports
pub use webhook::model::{
    CreateWebhook, DeliveryStatus, UpdateWebhook, Webhook, WebhookDelivery,
    WebhookDeliveryResponse, WebhookResponse, WebhookStatus,
};
pub use webhook::repository::{
    BoxWebhookDeliveryRepository, BoxWebhookRepository, InMemoryWebhookDeliveryRepository,
    InMemoryWebhookRepository, WebhookDeliveryRepository, WebhookRepository,
};
pub use webhook::service::WebhookService;
pub use webhook::subscriber::WebhookSubscriber;

// MFA module re-exports
pub use mfa::model::{
    BackupCodesResponse, DisableMfaRequest, EnableMfaRequest, MfaChallenge, MfaMethod,
    MfaRequiredResponse, MfaSettings, MfaSetupResponse, MfaStatusResponse, VerifyMfaRequest,
    VerifyTotpRequest,
};
pub use mfa::repository::{BoxMfaRepository, InMemoryMfaRepository, MfaRepository};
pub use mfa::service::MfaService;

// Document module re-exports
pub use document::model::{
    BulkRestoreRequest, CreateDocument, CreateDocumentCategory, CreateDocumentLink,
    CreateDocumentVersion, Document, DocumentCategory, DocumentCategoryResponse, DocumentLink,
    DocumentResponse, DocumentSearchParams, DocumentSearchResult, DocumentVersion,
    LinkedEntityType, UpdateDocument, UpdateDocumentCategory,
};
pub use document::repository::{
    BoxDocumentRepository, DocumentRepository, InMemoryDocumentRepository,
};
pub use document::service::DocumentService;

// Job module re-exports
pub use job::model::{
    CreateJob, CreateJobSchedule, Job, JobCounts, JobPriority, JobSchedule, JobStatus, JobType,
};
pub use job::repository::{BoxJobRepository, InMemoryJobRepository, JobRepository};
pub use job::service::JobService;

// LDAP module re-exports
pub use ldap::model::{
    CreateLdapConfig, LdapConfig, LdapConfigResponse, LdapSyncResult, LdapUser,
    TestLdapConnectionRequest, UpdateLdapConfig,
};
pub use ldap::repository::{
    BoxLdapConfigRepository, InMemoryLdapConfigRepository, LdapConfigRepository,
};
pub use ldap::service::{Ldap3Client, LdapClient, LdapSyncService};

// Test: is file module visible?
pub type __TestFileVisibility = crate::domain::file::model::FileRecord;
