//! Common utilities and types shared across modules

pub mod bank_parsers;
pub mod cache;
pub mod cdc;
pub mod circuit_breaker;
pub mod events;
#[cfg(feature = "postgres")]
pub mod events_postgres;
pub mod file_storage;
pub mod gov;
pub mod import;
pub mod inter_company;
pub mod jobs;
pub mod notifications;
pub mod pagination;
pub mod read_replicas;
pub mod reports;
pub mod retry;
pub mod s3_storage;
pub mod search;
pub mod search_postgres;
pub mod soft_delete;
pub mod tracing_mod;

pub use bank_parsers::{parse_bank_xml, parse_camt053, parse_mt940};
pub use cache::{BoxCacheService, CacheService, InMemoryCacheService};
pub use cdc::{convert_to_domain_event, parse_cdc_event, CdcEvent, CdcListener, CdcOperation};
pub use circuit_breaker::{
    CircuitBreaker, CircuitBreakerConfig, CircuitBreakerRegistry, CircuitBreakerStats,
    CircuitState, SERVICE_BANK, SERVICE_EMAIL, SERVICE_FILE_STORAGE, SERVICE_GIB, SERVICE_SMS,
    SERVICE_WEBHOOK,
};
pub use events::{
    AccountingEntrySubscriber, BoxEventBus, DeadLetterEntry, DomainEvent,
    EDefterAccountingSubscriber, EFaturaIntegrationSubscriber, EventBus, EventStatus,
    InMemoryEventBus, OutboxEvent, StockDecrementSubscriber, TaxPeriodSubscriber,
};
#[cfg(feature = "postgres")]
pub use events_postgres::{publish_to_redis_streams, PostgresEventBus};
pub use file_storage::{
    BoxFileStorage, FileMetadata, FileUpload, LocalFileStorage, PresignedUrl, StorageBackend,
};
pub use gov::{
    BoxGibGateway, GibGateway, GibSendResult, GibStatusResult, InMemoryGibGateway,
    ResilientGibGateway,
};
pub use import::{BoxImportService, CsvImportService, ImportService};
pub use inter_company::{
    InterCompanyInvoiceLine, InterCompanyInvoiceResult, InterCompanyService,
    InterCompanyStockTransferResult,
};
pub use jobs::{
    BoxJobScheduler, CreateJob, InMemoryJobScheduler, Job, JobPriority, JobScheduler, JobStatus,
    JobType,
};
pub use notifications::{
    BoxNotificationService, EmailTemplate, InAppNotification, InMemoryNotificationService,
    Notification, NotificationChannel, NotificationPreference, NotificationPriority,
    NotificationRequest, NotificationService, NotificationStatus, ResilientNotificationService,
    UpdatePreference,
};
pub use pagination::{PaginatedResult, PaginationParams};
pub use read_replicas::{
    BoxDbRouter, DbRole, DbRouter, InMemoryDbRouter, QueryType, ReadAfterWriteMode, ReplicaHealth,
    ReplicaNode, RouterStats,
};
pub use reports::{
    BoxReportEngine, GeneratedReport, InMemoryReportEngine, ReportEngine, ReportError,
    ReportFormat, ReportMeta, ReportRequest, ReportTemplate, ReportType,
};
pub use retry::{
    resilient_call, BoxRetryStats, RetryConfig, RetryPolicy, RetryStats, RetryStatsSnapshot,
};
pub use s3_storage::S3FileStorage;
pub use search::{
    BoxSearchService, InMemorySearchService, SearchDocument, SearchQuery, SearchResult,
    SearchService,
};
#[cfg(feature = "postgres")]
pub use search_postgres::PostgresSearchService;
pub use soft_delete::{SoftDeletable, SoftDeleteMeta};
pub use tracing_mod::{
    BoxTracingService, InMemoryTracingService, SpanStatus, TraceContext, TraceQuery, TraceSpan,
    TracingService,
};

use serde::Serialize;
use utoipa::ToSchema;

/// Simple localized success message payload.
#[derive(Serialize, ToSchema)]
pub struct MessageResponse {
    pub message: String,
}
