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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_document_creation() {
        let now = Utc::now();
        let doc = Document {
            id: 1,
            tenant_id: 10,
            name: "Contract".to_string(),
            filename: "contract.pdf".to_string(),
            size_bytes: 1024,
            mime_type: "application/pdf".to_string(),
            hash: "abc123".to_string(),
            storage_path: "/docs/contract.pdf".to_string(),
            uploaded_by: Some(5),
            category_id: Some(2),
            tags: vec!["legal".to_string(), "2024".to_string()],
            description: Some("Main contract".to_string()),
            current_version: 1,
            created_at: now,
            updated_at: now,
            deleted_at: None,
            deleted_by: None,
        };

        assert_eq!(doc.id, 1);
        assert_eq!(doc.tenant_id, 10);
        assert_eq!(doc.name, "Contract");
        assert_eq!(doc.filename, "contract.pdf");
        assert_eq!(doc.size_bytes, 1024);
        assert_eq!(doc.mime_type, "application/pdf");
        assert_eq!(doc.hash, "abc123");
        assert_eq!(doc.storage_path, "/docs/contract.pdf");
        assert_eq!(doc.uploaded_by, Some(5));
        assert_eq!(doc.category_id, Some(2));
        assert_eq!(doc.tags.len(), 2);
        assert_eq!(doc.current_version, 1);
        assert!(doc.deleted_at.is_none());
        assert!(doc.deleted_by.is_none());
    }

    #[test]
    fn test_create_document_creation() {
        let create = CreateDocument {
            tenant_id: 10,
            name: "Report".to_string(),
            filename: "report.xlsx".to_string(),
            size_bytes: 2048,
            mime_type: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
                .to_string(),
            hash: "def456".to_string(),
            storage_path: "/docs/report.xlsx".to_string(),
            uploaded_by: Some(3),
            category_id: None,
            tags: Some(vec!["finance".to_string()]),
            description: Some("Q1 report".to_string()),
        };

        assert_eq!(create.tenant_id, 10);
        assert_eq!(create.name, "Report");
        assert_eq!(create.filename, "report.xlsx");
        assert_eq!(create.size_bytes, 2048);
        assert_eq!(
            create.mime_type,
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
        );
        assert_eq!(create.hash, "def456");
        assert_eq!(create.storage_path, "/docs/report.xlsx");
        assert_eq!(create.uploaded_by, Some(3));
        assert!(create.category_id.is_none());
        assert_eq!(create.tags.as_ref().unwrap().len(), 1);
        assert_eq!(create.description, Some("Q1 report".to_string()));
    }

    #[test]
    fn test_document_serialization() {
        let now = Utc::now();
        let doc = Document {
            id: 1,
            tenant_id: 10,
            name: "Invoice".to_string(),
            filename: "inv.pdf".to_string(),
            size_bytes: 512,
            mime_type: "application/pdf".to_string(),
            hash: "hash99".to_string(),
            storage_path: "/inv.pdf".to_string(),
            uploaded_by: None,
            category_id: None,
            tags: vec![],
            description: None,
            current_version: 1,
            created_at: now,
            updated_at: now,
            deleted_at: None,
            deleted_by: None,
        };

        let json = serde_json::to_string(&doc).unwrap();
        assert!(json.contains("Invoice"));
        assert!(json.contains("inv.pdf"));

        let deserialized: Document = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, doc.id);
        assert_eq!(deserialized.name, doc.name);
        assert_eq!(deserialized.filename, doc.filename);
    }

    #[test]
    fn test_create_document_deserialization() {
        let json = r#"{"tenant_id":10,"name":"Memo","filename":"memo.txt","size_bytes":128,"mime_type":"text/plain","hash":"hash00","storage_path":"/memo.txt","uploaded_by":null,"category_id":null,"tags":null,"description":null}"#;

        let deserialized: CreateDocument = serde_json::from_str(json).unwrap();
        assert_eq!(deserialized.tenant_id, 10);
        assert_eq!(deserialized.name, "Memo");
        assert_eq!(deserialized.filename, "memo.txt");
        assert_eq!(deserialized.size_bytes, 128);
        assert_eq!(deserialized.mime_type, "text/plain");
        assert_eq!(deserialized.hash, "hash00");
        assert_eq!(deserialized.storage_path, "/memo.txt");
        assert!(deserialized.uploaded_by.is_none());
        assert!(deserialized.category_id.is_none());
        assert!(deserialized.tags.is_none());
        assert!(deserialized.description.is_none());
    }

    #[test]
    fn test_document_to_response() {
        let now = Utc::now();
        let doc = Document {
            id: 7,
            tenant_id: 10,
            name: "Plan".to_string(),
            filename: "plan.pdf".to_string(),
            size_bytes: 4096,
            mime_type: "application/pdf".to_string(),
            hash: "h1".to_string(),
            storage_path: "/plan.pdf".to_string(),
            uploaded_by: Some(2),
            category_id: Some(3),
            tags: vec!["strategy".to_string()],
            description: Some("2025 plan".to_string()),
            current_version: 2,
            created_at: now,
            updated_at: now,
            deleted_at: None,
            deleted_by: None,
        };

        let resp: DocumentResponse = doc.into();
        assert_eq!(resp.id, 7);
        assert_eq!(resp.name, "Plan");
        assert_eq!(resp.filename, "plan.pdf");
        assert_eq!(resp.size_bytes, 4096);
        assert_eq!(resp.mime_type, "application/pdf");
        assert_eq!(resp.hash, "h1");
        assert_eq!(resp.uploaded_by, Some(2));
        assert_eq!(resp.category_id, Some(3));
        assert_eq!(resp.tags.len(), 1);
        assert_eq!(resp.current_version, 2);
    }

    #[test]
    fn test_document_category_to_response() {
        let now = Utc::now();
        let cat = DocumentCategory {
            id: 4,
            tenant_id: 10,
            name: "Legal".to_string(),
            description: Some("Legal docs".to_string()),
            color: Some("#ff0000".to_string()),
            parent_id: None,
            created_at: now,
            updated_at: now,
        };

        let resp: DocumentCategoryResponse = cat.into();
        assert_eq!(resp.id, 4);
        assert_eq!(resp.name, "Legal");
        assert_eq!(resp.description, Some("Legal docs".to_string()));
        assert_eq!(resp.color, Some("#ff0000".to_string()));
        assert!(resp.parent_id.is_none());
    }

    #[test]
    fn test_linked_entity_type_display() {
        assert_eq!(LinkedEntityType::Invoice.to_string(), "invoice");
        assert_eq!(LinkedEntityType::Order.to_string(), "order");
        assert_eq!(LinkedEntityType::Cari.to_string(), "cari");
        assert_eq!(LinkedEntityType::Product.to_string(), "product");
        assert_eq!(LinkedEntityType::Project.to_string(), "project");
        assert_eq!(LinkedEntityType::Employee.to_string(), "employee");
        assert_eq!(
            LinkedEntityType::PurchaseOrder.to_string(),
            "purchase_order"
        );
        assert_eq!(LinkedEntityType::SalesOrder.to_string(), "sales_order");
        assert_eq!(LinkedEntityType::WorkOrder.to_string(), "work_order");
        assert_eq!(LinkedEntityType::Other.to_string(), "other");
    }

    #[test]
    fn test_linked_entity_type_from_str() {
        assert!(matches!(
            "invoice".parse::<LinkedEntityType>().unwrap(),
            LinkedEntityType::Invoice
        ));
        assert!(matches!(
            "ORDER".parse::<LinkedEntityType>().unwrap(),
            LinkedEntityType::Order
        ));
        assert!(matches!(
            "Cari".parse::<LinkedEntityType>().unwrap(),
            LinkedEntityType::Cari
        ));
        assert!(matches!(
            "product".parse::<LinkedEntityType>().unwrap(),
            LinkedEntityType::Product
        ));
        assert!(matches!(
            "project".parse::<LinkedEntityType>().unwrap(),
            LinkedEntityType::Project
        ));
        assert!(matches!(
            "employee".parse::<LinkedEntityType>().unwrap(),
            LinkedEntityType::Employee
        ));
        assert!(matches!(
            "purchase_order".parse::<LinkedEntityType>().unwrap(),
            LinkedEntityType::PurchaseOrder
        ));
        assert!(matches!(
            "sales_order".parse::<LinkedEntityType>().unwrap(),
            LinkedEntityType::SalesOrder
        ));
        assert!(matches!(
            "work_order".parse::<LinkedEntityType>().unwrap(),
            LinkedEntityType::WorkOrder
        ));
        assert!(matches!(
            "unknown".parse::<LinkedEntityType>().unwrap(),
            LinkedEntityType::Other
        ));
    }

    #[test]
    fn test_document_search_params_default() {
        let params = DocumentSearchParams::default();
        assert!(params.query.is_none());
        assert!(params.category_id.is_none());
        assert!(params.tags.is_none());
        assert!(params.entity_type.is_none());
        assert!(params.entity_id.is_none());
        assert!(params.mime_type.is_none());
        assert!(params.uploaded_by.is_none());
        assert_eq!(params.page, 1);
        assert_eq!(params.per_page, 20);
    }

    #[test]
    fn test_document_version_creation() {
        let now = Utc::now();
        let version = DocumentVersion {
            id: 1,
            document_id: 5,
            tenant_id: 10,
            version_number: 2,
            filename: "v2.pdf".to_string(),
            size_bytes: 1024,
            hash: "v2hash".to_string(),
            storage_path: "/v2.pdf".to_string(),
            created_by: Some(3),
            comment: Some("Updated".to_string()),
            created_at: now,
        };

        assert_eq!(version.document_id, 5);
        assert_eq!(version.version_number, 2);
        assert_eq!(version.filename, "v2.pdf");
        assert_eq!(version.comment, Some("Updated".to_string()));
    }

    #[test]
    fn test_document_link_creation() {
        let now = Utc::now();
        let link = DocumentLink {
            id: 1,
            document_id: 5,
            tenant_id: 10,
            entity_type: "invoice".to_string(),
            entity_id: 42,
            created_at: now,
        };

        assert_eq!(link.document_id, 5);
        assert_eq!(link.entity_type, "invoice");
        assert_eq!(link.entity_id, 42);
    }

    #[test]
    fn test_bulk_restore_request_creation() {
        let req = BulkRestoreRequest { ids: vec![1, 2, 3] };
        assert_eq!(req.ids.len(), 3);
        assert_eq!(req.ids[0], 1);
    }

    #[test]
    fn test_bulk_restore_response_creation() {
        let resp = BulkRestoreResponse {
            restored: 2,
            items: vec!["a".to_string(), "b".to_string()],
            failed: vec![BulkRestoreFailed {
                id: 3,
                reason: "Not found".to_string(),
            }],
        };
        assert_eq!(resp.restored, 2);
        assert_eq!(resp.items.len(), 2);
        assert_eq!(resp.failed.len(), 1);
        assert_eq!(resp.failed[0].id, 3);
    }
}
