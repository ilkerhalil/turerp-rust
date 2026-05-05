//! Common utilities and types shared across modules

pub mod cache;
pub mod events;
pub mod file_storage;
pub mod gov;
pub mod jobs;
pub mod notifications;
pub mod pagination;
pub mod read_replicas;
pub mod reports;
pub mod search;
pub mod soft_delete;
pub mod tracing_mod;

pub use cache::{BoxCacheService, CacheService, InMemoryCacheService};
pub use events::{
    AccountingEntrySubscriber, BoxEventBus, DeadLetterEntry, DomainEvent,
    EDefterAccountingSubscriber, EFaturaIntegrationSubscriber, EventBus, EventStatus,
    InMemoryEventBus, OutboxEvent, StockDecrementSubscriber, TaxPeriodSubscriber,
};
pub use file_storage::{
    BoxFileStorage, FileMetadata, FileUpload, LocalFileStorage, PresignedUrl, StorageBackend,
};
pub use gov::{BoxGibGateway, GibGateway, GibSendResult, GibStatusResult, InMemoryGibGateway};
pub use jobs::{
    BoxJobScheduler, CreateJob, InMemoryJobScheduler, Job, JobPriority, JobScheduler, JobStatus,
    JobType,
};
pub use notifications::{
    BoxNotificationService, EmailTemplate, InAppNotification, InMemoryNotificationService,
    Notification, NotificationChannel, NotificationPriority, NotificationRequest,
    NotificationService, NotificationStatus,
};
pub use pagination::{PaginatedResult, PaginationParams};
pub use read_replicas::{
    BoxDbRouter, DbRole, DbRouter, InMemoryDbRouter, QueryType, ReadAfterWriteMode, ReplicaHealth,
    ReplicaNode, RouterStats,
};
pub use reports::{
    BoxReportEngine, GeneratedReport, InMemoryReportEngine, ReportEngine, ReportFormat, ReportMeta,
    ReportRequest, ReportType,
};
pub use search::{
    BoxSearchService, InMemorySearchService, SearchDocument, SearchQuery, SearchResult,
    SearchService,
};
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
