//! Data Archiving domain module
//!
//! Provides archive policies, background jobs, record restoration,
//! and querying for archived data across tenant-scoped tables.

pub mod model;
#[cfg(feature = "postgres")]
pub mod postgres_repository;
pub mod repository;
pub mod service;

pub use model::{
    ArchiveJob, ArchiveJobResponse, ArchiveJobStatus, ArchivePolicy, ArchivePolicyResponse,
    ArchiveRecord, ArchiveRecordResponse, BulkRestoreFailed, BulkRestoreResponse, CreateArchiveJob,
    CreateArchivePolicy, RestoreRequest, UpdateArchivePolicy,
};
#[cfg(feature = "postgres")]
pub use postgres_repository::{
    PostgresArchiveJobRepository, PostgresArchivePolicyRepository, PostgresArchiveRecordRepository,
};
pub use repository::{
    ArchiveJobRepository, ArchivePolicyRepository, ArchiveRecordRepository,
    BoxArchiveJobRepository, BoxArchivePolicyRepository, BoxArchiveRecordRepository,
    InMemoryArchiveJobRepository, InMemoryArchivePolicyRepository, InMemoryArchiveRecordRepository,
};
pub use service::ArchiveService;
