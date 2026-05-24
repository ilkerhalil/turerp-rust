//! File (Document Management) domain module

pub mod model;
pub mod postgres_repository;
pub mod repository;
pub mod service;

// Re-exports
pub use model::{CreateFileRecord, FileRecord, FileResponse, UpdateFileRecord};
pub use postgres_repository::PostgresFileRepository;
pub use repository::{BoxFileRepository, FileRepository, InMemoryFileRepository};
pub use service::FileService;
