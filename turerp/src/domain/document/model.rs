//! Document Management System domain models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Document entity — central metadata for uploaded files
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Document {
    pub id: i64,
    pub tenant_id: i64,
    pub name: String,
    pub filename: String,
    pub size_bytes: i64,
    pub mime_type: String,
    pub hash: String,
    pub storage_path: String,
    pub uploaded_by: Option<i64>,
    pub category_id: Option<i64>,
    pub tags: Vec<String>,
    pub description: Option<String>,
    pub current_version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

/// Response DTO for a document
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DocumentResponse {
    pub id: i64,
    pub name: String,
    pub filename: String,
    pub size_bytes: i64,
    pub mime_type: String,
    pub hash: String,
    pub uploaded_by: Option<i64>,
    pub category_id: Option<i64>,
    pub tags: Vec<String>,
    pub description: Option<String>,
    pub current_version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Document> for DocumentResponse {
    fn from(doc: Document) -> Self {
        Self {
            id: doc.id,
            name: doc.name,
            filename: doc.filename,
            size_bytes: doc.size_bytes,
            mime_type: doc.mime_type,
            hash: doc.hash,
            uploaded_by: doc.uploaded_by,
            category_id: doc.category_id,
            tags: doc.tags,
            description: doc.description,
            current_version: doc.current_version,
            created_at: doc.created_at,
            updated_at: doc.updated_at,
        }
    }
}

/// Request to create a document metadata record
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateDocument {
    pub tenant_id: i64,
    pub name: String,
    pub filename: String,
    pub size_bytes: i64,
    pub mime_type: String,
    pub hash: String,
    pub storage_path: String,
    pub uploaded_by: Option<i64>,
    pub category_id: Option<i64>,
    pub tags: Option<Vec<String>>,
    pub description: Option<String>,
}

/// Request to update document metadata
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateDocument {
    pub name: Option<String>,
    pub category_id: Option<i64>,
    pub tags: Option<Vec<String>>,
    pub description: Option<String>,
}

/// Document version tracking
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DocumentVersion {
    pub id: i64,
    pub document_id: i64,
    pub tenant_id: i64,
    pub version_number: i32,
    pub filename: String,
    pub size_bytes: i64,
    pub hash: String,
    pub storage_path: String,
    pub created_by: Option<i64>,
    pub comment: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Request to create a new document version
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateDocumentVersion {
    pub document_id: i64,
    pub tenant_id: i64,
    pub filename: String,
    pub size_bytes: i64,
    pub hash: String,
    pub storage_path: String,
    pub created_by: Option<i64>,
    pub comment: Option<String>,
}

/// Document category for organization
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DocumentCategory {
    pub id: i64,
    pub tenant_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub parent_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Response DTO for a document category
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DocumentCategoryResponse {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub parent_id: Option<i64>,
    pub created_at: DateTime<Utc>,
}

impl From<DocumentCategory> for DocumentCategoryResponse {
    fn from(cat: DocumentCategory) -> Self {
        Self {
            id: cat.id,
            name: cat.name,
            description: cat.description,
            color: cat.color,
            parent_id: cat.parent_id,
            created_at: cat.created_at,
        }
    }
}

/// Request to create a document category
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateDocumentCategory {
    pub tenant_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub parent_id: Option<i64>,
}

/// Request to update a document category
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateDocumentCategory {
    pub name: Option<String>,
    pub description: Option<String>,
    pub color: Option<String>,
    pub parent_id: Option<i64>,
}

/// Link between a document and a business entity (invoice, order, cari, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DocumentLink {
    pub id: i64,
    pub document_id: i64,
    pub tenant_id: i64,
    pub entity_type: String,
    pub entity_id: i64,
    pub created_at: DateTime<Utc>,
}

/// Request to create a document-entity link
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateDocumentLink {
    pub document_id: i64,
    pub tenant_id: i64,
    pub entity_type: String,
    pub entity_id: i64,
}

/// Supported entity types for document linking
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum LinkedEntityType {
    Invoice,
    Order,
    Cari,
    Product,
    Project,
    Employee,
    PurchaseOrder,
    SalesOrder,
    WorkOrder,
    Other,
}

impl std::fmt::Display for LinkedEntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LinkedEntityType::Invoice => write!(f, "invoice"),
            LinkedEntityType::Order => write!(f, "order"),
            LinkedEntityType::Cari => write!(f, "cari"),
            LinkedEntityType::Product => write!(f, "product"),
            LinkedEntityType::Project => write!(f, "project"),
            LinkedEntityType::Employee => write!(f, "employee"),
            LinkedEntityType::PurchaseOrder => write!(f, "purchase_order"),
            LinkedEntityType::SalesOrder => write!(f, "sales_order"),
            LinkedEntityType::WorkOrder => write!(f, "work_order"),
            LinkedEntityType::Other => write!(f, "other"),
        }
    }
}

impl std::str::FromStr for LinkedEntityType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "invoice" => Ok(LinkedEntityType::Invoice),
            "order" => Ok(LinkedEntityType::Order),
            "cari" => Ok(LinkedEntityType::Cari),
            "product" => Ok(LinkedEntityType::Product),
            "project" => Ok(LinkedEntityType::Project),
            "employee" => Ok(LinkedEntityType::Employee),
            "purchase_order" => Ok(LinkedEntityType::PurchaseOrder),
            "sales_order" => Ok(LinkedEntityType::SalesOrder),
            "work_order" => Ok(LinkedEntityType::WorkOrder),
            _ => Ok(LinkedEntityType::Other),
        }
    }
}

/// Search parameters for document queries
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct DocumentSearchParams {
    pub query: Option<String>,
    pub category_id: Option<i64>,
    pub tags: Option<Vec<String>>,
    pub entity_type: Option<String>,
    pub entity_id: Option<i64>,
    pub mime_type: Option<String>,
    pub uploaded_by: Option<i64>,
    pub page: u32,
    pub per_page: u32,
}

impl Default for DocumentSearchParams {
    fn default() -> Self {
        Self {
            query: None,
            category_id: None,
            tags: None,
            entity_type: None,
            entity_id: None,
            mime_type: None,
            uploaded_by: None,
            page: 1,
            per_page: 20,
        }
    }
}

/// Paginated search result for documents
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DocumentSearchResult {
    pub items: Vec<DocumentResponse>,
    pub total: u32,
    pub page: u32,
    pub per_page: u32,
}

/// Bulk restore request
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct BulkRestoreRequest {
    pub ids: Vec<i64>,
}

/// Bulk restore response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BulkRestoreResponse<T> {
    pub restored: usize,
    pub items: Vec<T>,
    pub failed: Vec<BulkRestoreFailed>,
}

/// Failed restore item
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BulkRestoreFailed {
    pub id: i64,
    pub reason: String,
}
