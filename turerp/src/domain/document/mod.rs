//! Document Management System (DMS) domain module

pub mod model;
pub mod postgres_repository;
pub mod repository;
pub mod service;

// Re-exports
pub use model::{
    BulkRestoreFailed, BulkRestoreRequest, BulkRestoreResponse, CreateDocument,
    CreateDocumentCategory, CreateDocumentLink, CreateDocumentVersion, Document, DocumentCategory,
    DocumentCategoryResponse, DocumentLink, DocumentResponse, DocumentSearchParams,
    DocumentSearchResult, DocumentVersion, LinkedEntityType, UpdateDocument,
    UpdateDocumentCategory,
};
pub use postgres_repository::PostgresDocumentRepository;
pub use repository::{BoxDocumentRepository, DocumentRepository, InMemoryDocumentRepository};
pub use service::DocumentService;
