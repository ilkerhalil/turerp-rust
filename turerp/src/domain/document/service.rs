//! Document Management System service — business logic

use crate::domain::document::model::{
    BulkRestoreFailed, BulkRestoreResponse, CreateDocument, CreateDocumentCategory,
    CreateDocumentLink, CreateDocumentVersion, Document, DocumentCategory, DocumentLink,
    DocumentResponse, DocumentSearchParams, DocumentSearchResult, DocumentVersion,
    LinkedEntityType, UpdateDocument, UpdateDocumentCategory,
};
use crate::domain::document::repository::BoxDocumentRepository;
use crate::error::ApiError;

/// Service for managing documents, versions, categories, and entity links
#[derive(Clone)]
pub struct DocumentService {
    repo: BoxDocumentRepository,
}

impl DocumentService {
    pub fn new(repo: BoxDocumentRepository) -> Self {
        Self { repo }
    }

    // --- Document Operations ---

    /// Create a new document metadata record
    pub async fn create_document(&self, create: CreateDocument) -> Result<Document, ApiError> {
        if create.name.trim().is_empty() {
            return Err(ApiError::Validation(
                "Document name is required".to_string(),
            ));
        }
        if create.filename.trim().is_empty() {
            return Err(ApiError::Validation("Filename is required".to_string()));
        }
        if create.size_bytes <= 0 {
            return Err(ApiError::Validation(
                "File size must be positive".to_string(),
            ));
        }
        if create.hash.trim().is_empty() {
            return Err(ApiError::Validation("File hash is required".to_string()));
        }
        self.repo.create(create).await
    }

    /// Get a document by ID
    pub async fn get_document(&self, id: i64, tenant_id: i64) -> Result<Document, ApiError> {
        self.repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Document {} not found", id)))
    }

    /// Search documents with filters
    pub async fn search_documents(
        &self,
        tenant_id: i64,
        params: DocumentSearchParams,
    ) -> Result<DocumentSearchResult, ApiError> {
        self.repo.search(tenant_id, params).await
    }

    /// Update document metadata
    pub async fn update_document(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateDocument,
    ) -> Result<Document, ApiError> {
        self.repo.update(id, tenant_id, update).await
    }

    /// Soft delete a document
    pub async fn delete_document(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.repo.soft_delete(id, tenant_id, deleted_by).await
    }

    /// Restore a soft-deleted document
    pub async fn restore_document(&self, id: i64, tenant_id: i64) -> Result<Document, ApiError> {
        self.repo.restore(id, tenant_id).await
    }

    /// List soft-deleted documents
    pub async fn list_deleted_documents(&self, tenant_id: i64) -> Result<Vec<Document>, ApiError> {
        self.repo.find_deleted(tenant_id).await
    }

    /// Permanently destroy a document
    pub async fn destroy_document(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.repo.destroy(id, tenant_id).await
    }

    /// Bulk restore soft-deleted documents
    pub async fn bulk_restore_documents(
        &self,
        ids: Vec<i64>,
        tenant_id: i64,
    ) -> Result<BulkRestoreResponse<DocumentResponse>, ApiError> {
        let mut restored = Vec::with_capacity(ids.len());
        let mut failed = Vec::with_capacity(ids.len());
        for id in ids {
            match self.repo.restore(id, tenant_id).await {
                Ok(doc) => restored.push(DocumentResponse::from(doc)),
                Err(e) => {
                    tracing::warn!("Failed to restore document {}: {}", id, e);
                    failed.push(BulkRestoreFailed {
                        id,
                        reason: e.to_string(),
                    });
                }
            }
        }
        Ok(BulkRestoreResponse {
            restored: restored.len(),
            items: restored,
            failed,
        })
    }

    // --- Versioning Operations ---

    /// Create a new version of a document
    pub async fn create_version(
        &self,
        version: CreateDocumentVersion,
    ) -> Result<DocumentVersion, ApiError> {
        if version.filename.trim().is_empty() {
            return Err(ApiError::Validation("Filename is required".to_string()));
        }
        if version.size_bytes <= 0 {
            return Err(ApiError::Validation(
                "File size must be positive".to_string(),
            ));
        }
        if version.hash.trim().is_empty() {
            return Err(ApiError::Validation("File hash is required".to_string()));
        }
        self.repo.create_version(version).await
    }

    /// List all versions of a document
    pub async fn list_versions(
        &self,
        document_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<DocumentVersion>, ApiError> {
        self.repo.list_versions(document_id, tenant_id).await
    }

    /// Get a specific version
    pub async fn get_version(
        &self,
        version_id: i64,
        tenant_id: i64,
    ) -> Result<DocumentVersion, ApiError> {
        self.repo
            .get_version(version_id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Version {} not found", version_id)))
    }

    // --- Category Operations ---

    /// Create a document category
    pub async fn create_category(
        &self,
        category: CreateDocumentCategory,
    ) -> Result<DocumentCategory, ApiError> {
        if category.name.trim().is_empty() {
            return Err(ApiError::Validation(
                "Category name is required".to_string(),
            ));
        }
        self.repo.create_category(category).await
    }

    /// Get a category by ID
    pub async fn get_category(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<DocumentCategory, ApiError> {
        self.repo
            .find_category_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Category {} not found", id)))
    }

    /// List all categories for a tenant
    pub async fn list_categories(&self, tenant_id: i64) -> Result<Vec<DocumentCategory>, ApiError> {
        self.repo.list_categories(tenant_id).await
    }

    /// Update a category
    pub async fn update_category(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateDocumentCategory,
    ) -> Result<DocumentCategory, ApiError> {
        self.repo.update_category(id, tenant_id, update).await
    }

    /// Delete a category
    pub async fn delete_category(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.repo.delete_category(id, tenant_id).await
    }

    // --- Entity Link Operations ---

    /// Link a document to a business entity
    pub async fn link_document(&self, link: CreateDocumentLink) -> Result<DocumentLink, ApiError> {
        if link.entity_type.trim().is_empty() {
            return Err(ApiError::Validation("Entity type is required".to_string()));
        }
        if link.entity_id <= 0 {
            return Err(ApiError::Validation(
                "Entity ID must be positive".to_string(),
            ));
        }
        self.repo.create_link(link).await
    }

    /// Remove a document-entity link
    pub async fn unlink_document(&self, link_id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.repo.delete_link(link_id, tenant_id).await
    }

    /// List links for a document
    pub async fn list_document_links(
        &self,
        document_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<DocumentLink>, ApiError> {
        self.repo
            .list_links_by_document(document_id, tenant_id)
            .await
    }

    /// List links for an entity
    pub async fn list_entity_links(
        &self,
        tenant_id: i64,
        entity_type: LinkedEntityType,
        entity_id: i64,
    ) -> Result<Vec<DocumentLink>, ApiError> {
        self.repo
            .list_links_by_entity(tenant_id, &entity_type.to_string(), entity_id)
            .await
    }

    /// Find documents linked to an entity
    pub async fn find_documents_by_entity(
        &self,
        tenant_id: i64,
        entity_type: LinkedEntityType,
        entity_id: i64,
    ) -> Result<Vec<Document>, ApiError> {
        self.repo
            .find_documents_by_entity(tenant_id, &entity_type.to_string(), entity_id)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::document::repository::InMemoryDocumentRepository;
    use std::sync::Arc;

    fn make_service() -> DocumentService {
        let repo = Arc::new(InMemoryDocumentRepository::new());
        DocumentService::new(repo)
    }

    #[tokio::test]
    async fn test_create_and_get_document() {
        let svc = make_service();

        let create = CreateDocument {
            tenant_id: 1,
            name: "Invoice #123".to_string(),
            filename: "invoice_123.pdf".to_string(),
            size_bytes: 1024,
            mime_type: "application/pdf".to_string(),
            hash: "sha256:abc123".to_string(),
            storage_path: "/docs/invoice_123.pdf".to_string(),
            uploaded_by: Some(1),
            category_id: None,
            tags: Some(vec!["invoice".to_string(), "2024".to_string()]),
            description: Some("January invoice".to_string()),
        };

        let doc = svc.create_document(create).await.unwrap();
        assert_eq!(doc.name, "Invoice #123");
        assert_eq!(doc.current_version, 1);
        assert_eq!(doc.tags.len(), 2);

        let found = svc.get_document(doc.id, 1).await.unwrap();
        assert_eq!(found.id, doc.id);
    }

    #[tokio::test]
    async fn test_create_document_validation() {
        let svc = make_service();

        let bad = CreateDocument {
            tenant_id: 1,
            name: "".to_string(),
            filename: "x.pdf".to_string(),
            size_bytes: 100,
            mime_type: "application/pdf".to_string(),
            hash: "h".to_string(),
            storage_path: "/x".to_string(),
            uploaded_by: None,
            category_id: None,
            tags: None,
            description: None,
        };
        assert!(svc.create_document(bad).await.is_err());
    }

    #[tokio::test]
    async fn test_document_versioning() {
        let svc = make_service();

        let create = CreateDocument {
            tenant_id: 1,
            name: "Contract".to_string(),
            filename: "contract_v1.pdf".to_string(),
            size_bytes: 2048,
            mime_type: "application/pdf".to_string(),
            hash: "sha256:v1".to_string(),
            storage_path: "/docs/contract_v1.pdf".to_string(),
            uploaded_by: Some(1),
            category_id: None,
            tags: None,
            description: None,
        };
        let doc = svc.create_document(create).await.unwrap();

        let version = CreateDocumentVersion {
            document_id: doc.id,
            tenant_id: 1,
            filename: "contract_v2.pdf".to_string(),
            size_bytes: 3072,
            hash: "sha256:v2".to_string(),
            storage_path: "/docs/contract_v2.pdf".to_string(),
            created_by: Some(1),
            comment: Some("Updated terms".to_string()),
        };
        let v = svc.create_version(version).await.unwrap();
        assert_eq!(v.version_number, 2);

        let versions = svc.list_versions(doc.id, 1).await.unwrap();
        assert_eq!(versions.len(), 2);

        let updated = svc.get_document(doc.id, 1).await.unwrap();
        assert_eq!(updated.current_version, 2);
    }

    #[tokio::test]
    async fn test_categories() {
        let svc = make_service();

        let cat = svc
            .create_category(CreateDocumentCategory {
                tenant_id: 1,
                name: "Invoices".to_string(),
                description: Some("All invoices".to_string()),
                color: Some("#FF0000".to_string()),
                parent_id: None,
            })
            .await
            .unwrap();
        assert_eq!(cat.name, "Invoices");

        let found = svc.get_category(cat.id, 1).await.unwrap();
        assert_eq!(found.id, cat.id);

        let list = svc.list_categories(1).await.unwrap();
        assert_eq!(list.len(), 1);
    }

    #[tokio::test]
    async fn test_entity_links() {
        let svc = make_service();

        let create = CreateDocument {
            tenant_id: 1,
            name: "Invoice doc".to_string(),
            filename: "inv.pdf".to_string(),
            size_bytes: 100,
            mime_type: "application/pdf".to_string(),
            hash: "sha256:a".to_string(),
            storage_path: "/inv.pdf".to_string(),
            uploaded_by: None,
            category_id: None,
            tags: None,
            description: None,
        };
        let doc = svc.create_document(create).await.unwrap();

        let link = svc
            .link_document(CreateDocumentLink {
                document_id: doc.id,
                tenant_id: 1,
                entity_type: "invoice".to_string(),
                entity_id: 42,
            })
            .await
            .unwrap();
        assert_eq!(link.entity_type, "invoice");

        let links = svc.list_document_links(doc.id, 1).await.unwrap();
        assert_eq!(links.len(), 1);

        let entity_docs = svc
            .find_documents_by_entity(1, LinkedEntityType::Invoice, 42)
            .await
            .unwrap();
        assert_eq!(entity_docs.len(), 1);
        assert_eq!(entity_docs[0].id, doc.id);
    }

    #[tokio::test]
    async fn test_soft_delete_and_restore() {
        let svc = make_service();

        let create = CreateDocument {
            tenant_id: 1,
            name: "Temp".to_string(),
            filename: "temp.pdf".to_string(),
            size_bytes: 100,
            mime_type: "application/pdf".to_string(),
            hash: "sha256:t".to_string(),
            storage_path: "/temp.pdf".to_string(),
            uploaded_by: None,
            category_id: None,
            tags: None,
            description: None,
        };
        let doc = svc.create_document(create).await.unwrap();

        svc.delete_document(doc.id, 1, 1).await.unwrap();
        assert!(svc.get_document(doc.id, 1).await.is_err());

        let deleted = svc.list_deleted_documents(1).await.unwrap();
        assert_eq!(deleted.len(), 1);

        let restored = svc.restore_document(doc.id, 1).await.unwrap();
        assert_eq!(restored.id, doc.id);
        assert!(restored.deleted_at.is_none());
    }

    #[tokio::test]
    async fn test_search_documents() {
        let svc = make_service();

        for i in 1..=3 {
            let create = CreateDocument {
                tenant_id: 1,
                name: format!("Doc {}", i),
                filename: format!("doc{}.pdf", i),
                size_bytes: 100,
                mime_type: "application/pdf".to_string(),
                hash: format!("sha256:{}", i),
                storage_path: format!("/doc{}.pdf", i),
                uploaded_by: Some(1),
                category_id: None,
                tags: Some(vec!["test".to_string()]),
                description: None,
            };
            svc.create_document(create).await.unwrap();
        }

        let params = DocumentSearchParams {
            query: Some("Doc".to_string()),
            ..Default::default()
        };
        let result = svc.search_documents(1, params).await.unwrap();
        assert_eq!(result.items.len(), 3);

        let params = DocumentSearchParams {
            tags: Some(vec!["test".to_string()]),
            ..Default::default()
        };
        let result = svc.search_documents(1, params).await.unwrap();
        assert_eq!(result.items.len(), 3);
    }
}
