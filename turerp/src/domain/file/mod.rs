//! File (Document Management) domain module

pub mod model;
pub mod postgres_repository;
pub mod repository;

// Re-exports
pub use model::{CreateFileRecord, FileRecord, FileResponse};
pub use postgres_repository::PostgresFileRepository;
pub use repository::{BoxFileRepository, FileRepository, InMemoryFileRepository};
