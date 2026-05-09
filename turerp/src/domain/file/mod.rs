//! File (Document Management) domain module

pub mod model;
#[cfg(feature = "postgres")]
pub mod postgres_repository;
pub mod repository;

// Re-exports
pub use model::{CreateFileRecord, FileRecord, FileResponse};
#[cfg(feature = "postgres")]
pub use postgres_repository::PostgresFileRepository;
pub use repository::{BoxFileRepository, FileRepository, InMemoryFileRepository};
