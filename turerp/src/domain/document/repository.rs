//! Document Management System repository trait and in-memory implementation

use async_trait::async_trait;
use chrono::Utc;
use parking_lot::Mutex;
use std::sync::Arc;

use crate::domain::document::model::{
    CreateDocument, CreateDocumentCategory, CreateDocumentLink, CreateDocumentVersion, Document,
    DocumentCategory, DocumentLink, DocumentSearchParams, DocumentSearchResult, DocumentVersion,
    UpdateDocument, UpdateDocumentCategory,
};
use crate::error::ApiError;

/// Repository trait for document operations
#[async_trait]
pub trait DocumentRepository: Send + Sync {
    // --- Document CRUD ---

    /// Create a new document metadata record
    async fn create(&self, doc: CreateDocument) -> Result<Document, ApiError>;

    /// Find document by ID (tenant-scoped)
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Document>, ApiError>;

    /// Search documents with filters and pagination
    async fn search(
        &self,
        tenant_id: i64,
        params: DocumentSearchParams,
    ) -> Result<DocumentSearchResult, ApiError>;

    /// Update document metadata
    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateDocument,
    ) -> Result<Document, ApiError>;

    /// Soft delete a document
    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError>;

    /// Restore a soft-deleted document
    async fn restore(&self, id: i64, tenant_id: i64) -> Result<Document, ApiError>;

    /// Find deleted documents for a tenant
    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Document>, ApiError>;

    /// Hard delete a document (permanent)
    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;

    // --- Versioning ---

    /// Create a new document version
    async fn create_version(
        &self,
        version: CreateDocumentVersion,
    ) -> Result<DocumentVersion, ApiError>;

    /// List versions for a document
    async fn list_versions(
        &self,
        document_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<DocumentVersion>, ApiError>;

    /// Get a specific version
    async fn get_version(
        &self,
        version_id: i64,
        tenant_id: i64,
    ) -> Result<Option<DocumentVersion>, ApiError>;

    // --- Categories ---

    /// Create a document category
    async fn create_category(
        &self,
        category: CreateDocumentCategory,
    ) -> Result<DocumentCategory, ApiError>;

    /// Find category by ID
    async fn find_category_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<DocumentCategory>, ApiError>;

    /// List all categories for a tenant
    async fn list_categories(&self, tenant_id: i64) -> Result<Vec<DocumentCategory>, ApiError>;

    /// Update a category
    async fn update_category(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateDocumentCategory,
    ) -> Result<DocumentCategory, ApiError>;

    /// Delete a category
    async fn delete_category(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;

    // --- Entity Links ---

    /// Link a document to an entity
    async fn create_link(&self, link: CreateDocumentLink) -> Result<DocumentLink, ApiError>;

    /// Remove a link
    async fn delete_link(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;

    /// List links for a document
    async fn list_links_by_document(
        &self,
        document_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<DocumentLink>, ApiError>;

    /// List links for an entity
    async fn list_links_by_entity(
        &self,
        tenant_id: i64,
        entity_type: &str,
        entity_id: i64,
    ) -> Result<Vec<DocumentLink>, ApiError>;

    /// Find documents linked to an entity
    async fn find_documents_by_entity(
        &self,
        tenant_id: i64,
        entity_type: &str,
        entity_id: i64,
    ) -> Result<Vec<Document>, ApiError>;
}

/// Type alias for boxed document repository
pub type BoxDocumentRepository = Arc<dyn DocumentRepository>;

/// In-memory document repository for development/testing
pub struct InMemoryDocumentRepository {
    documents: Mutex<Vec<Document>>,
    versions: Mutex<Vec<DocumentVersion>>,
    categories: Mutex<Vec<DocumentCategory>>,
    links: Mutex<Vec<DocumentLink>>,
    next_doc_id: Mutex<i64>,
    next_version_id: Mutex<i64>,
    next_category_id: Mutex<i64>,
    next_link_id: Mutex<i64>,
}

impl InMemoryDocumentRepository {
    pub fn new() -> Self {
        Self {
            documents: Mutex::new(Vec::new()),
            versions: Mutex::new(Vec::new()),
            categories: Mutex::new(Vec::new()),
            links: Mutex::new(Vec::new()),
            next_doc_id: Mutex::new(1),
            next_version_id: Mutex::new(1),
            next_category_id: Mutex::new(1),
            next_link_id: Mutex::new(1),
        }
    }

    fn allocate_doc_id(&self) -> i64 {
        let mut id = self.next_doc_id.lock();
        let val = *id;
        *id += 1;
        val
    }

    fn allocate_version_id(&self) -> i64 {
        let mut id = self.next_version_id.lock();
        let val = *id;
        *id += 1;
        val
    }

    fn allocate_category_id(&self) -> i64 {
        let mut id = self.next_category_id.lock();
        let val = *id;
        *id += 1;
        val
    }

    fn allocate_link_id(&self) -> i64 {
        let mut id = self.next_link_id.lock();
        let val = *id;
        *id += 1;
        val
    }
}

impl Default for InMemoryDocumentRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DocumentRepository for InMemoryDocumentRepository {
    // --- Document CRUD ---

    async fn create(&self, doc: CreateDocument) -> Result<Document, ApiError> {
        let now = Utc::now();
        let record = Document {
            id: self.allocate_doc_id(),
            tenant_id: doc.tenant_id,
            name: doc.name,
            filename: doc.filename,
            size_bytes: doc.size_bytes,
            mime_type: doc.mime_type,
            hash: doc.hash,
            storage_path: doc.storage_path,
            uploaded_by: doc.uploaded_by,
            category_id: doc.category_id,
            tags: doc.tags.unwrap_or_default(),
            description: doc.description,
            current_version: 1,
            created_at: now,
            updated_at: now,
            deleted_at: None,
            deleted_by: None,
        };
        self.documents.lock().push(record.clone());

        // Create initial version
        let version = DocumentVersion {
            id: self.allocate_version_id(),
            document_id: record.id,
            tenant_id: doc.tenant_id,
            version_number: 1,
            filename: record.filename.clone(),
            size_bytes: record.size_bytes,
            hash: record.hash.clone(),
            storage_path: record.storage_path.clone(),
            created_by: doc.uploaded_by,
            comment: Some("Initial version".to_string()),
            created_at: now,
        };
        self.versions.lock().push(version);

        Ok(record)
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Document>, ApiError> {
        Ok(self
            .documents
            .lock()
            .iter()
            .find(|d| d.id == id && d.tenant_id == tenant_id && d.deleted_at.is_none())
            .cloned())
    }

    async fn search(
        &self,
        tenant_id: i64,
        params: DocumentSearchParams,
    ) -> Result<DocumentSearchResult, ApiError> {
        let docs = self.documents.lock();
        let links = self.links.lock();

        let mut filtered: Vec<Document> = docs
            .iter()
            .filter(|d| d.tenant_id == tenant_id && d.deleted_at.is_none())
            .filter(|d| {
                if let Some(ref query) = params.query {
                    let q = query.to_lowercase();
                    d.name.to_lowercase().contains(&q)
                        || d.filename.to_lowercase().contains(&q)
                        || d.mime_type.to_lowercase().contains(&q)
                        || d.tags.iter().any(|t| t.to_lowercase().contains(&q))
                } else {
                    true
                }
            })
            .filter(|d| {
                if let Some(cat_id) = params.category_id {
                    d.category_id == Some(cat_id)
                } else {
                    true
                }
            })
            .filter(|d| {
                if let Some(ref tags) = params.tags {
                    tags.iter().all(|t| d.tags.contains(t))
                } else {
                    true
                }
            })
            .filter(|d| {
                if let Some(ref mime) = params.mime_type {
                    d.mime_type.to_lowercase() == mime.to_lowercase()
                } else {
                    true
                }
            })
            .filter(|d| {
                if let Some(uid) = params.uploaded_by {
                    d.uploaded_by == Some(uid)
                } else {
                    true
                }
            })
            .filter(|d| {
                if let (Some(ref et), Some(eid)) = (&params.entity_type, params.entity_id) {
                    links.iter().any(|l| {
                        l.document_id == d.id
                            && l.tenant_id == tenant_id
                            && l.entity_type == *et
                            && l.entity_id == eid
                    })
                } else {
                    true
                }
            })
            .cloned()
            .collect();

        let total = filtered.len() as u32;
        let offset = ((params.page.saturating_sub(1)) * params.per_page) as usize;
        let limit = params.per_page as usize;

        let items: Vec<Document> = filtered
            .drain(offset..offset + limit.min(filtered.len().saturating_sub(offset)))
            .collect();

        Ok(DocumentSearchResult {
            items: items.into_iter().map(Into::into).collect(),
            total,
            page: params.page,
            per_page: params.per_page,
        })
    }

    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateDocument,
    ) -> Result<Document, ApiError> {
        let mut docs = self.documents.lock();
        let doc = docs
            .iter_mut()
            .find(|d| d.id == id && d.tenant_id == tenant_id && d.deleted_at.is_none())
            .ok_or_else(|| ApiError::NotFound(format!("Document {} not found", id)))?;

        if let Some(name) = update.name {
            doc.name = name;
        }
        if let Some(category_id) = update.category_id {
            doc.category_id = Some(category_id);
        }
        if let Some(tags) = update.tags {
            doc.tags = tags;
        }
        if let Some(description) = update.description {
            doc.description = Some(description);
        }
        doc.updated_at = Utc::now();

        Ok(doc.clone())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let mut docs = self.documents.lock();
        let doc = docs
            .iter_mut()
            .find(|d| d.id == id && d.tenant_id == tenant_id && d.deleted_at.is_none())
            .ok_or_else(|| ApiError::NotFound(format!("Document {} not found", id)))?;
        doc.deleted_at = Some(Utc::now());
        doc.deleted_by = Some(deleted_by);
        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<Document, ApiError> {
        let mut docs = self.documents.lock();
        let doc = docs
            .iter_mut()
            .find(|d| d.id == id && d.tenant_id == tenant_id && d.deleted_at.is_some())
            .ok_or_else(|| ApiError::NotFound(format!("Document {} not found", id)))?;
        doc.deleted_at = None;
        doc.deleted_by = None;
        Ok(doc.clone())
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Document>, ApiError> {
        Ok(self
            .documents
            .lock()
            .iter()
            .filter(|d| d.tenant_id == tenant_id && d.deleted_at.is_some())
            .cloned()
            .collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut docs = self.documents.lock();
        let pos = docs
            .iter()
            .position(|d| d.id == id && d.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Document {} not found", id)))?;
        docs.remove(pos);

        // Clean up versions and links
        self.versions
            .lock()
            .retain(|v| v.document_id != id || v.tenant_id != tenant_id);
        self.links
            .lock()
            .retain(|l| l.document_id != id || l.tenant_id != tenant_id);

        Ok(())
    }

    // --- Versioning ---

    async fn create_version(
        &self,
        version: CreateDocumentVersion,
    ) -> Result<DocumentVersion, ApiError> {
        let mut docs = self.documents.lock();
        let doc = docs
            .iter_mut()
            .find(|d| {
                d.id == version.document_id
                    && d.tenant_id == version.tenant_id
                    && d.deleted_at.is_none()
            })
            .ok_or_else(|| {
                ApiError::NotFound(format!("Document {} not found", version.document_id))
            })?;

        doc.current_version += 1;
        doc.filename = version.filename.clone();
        doc.size_bytes = version.size_bytes;
        doc.hash = version.hash.clone();
        doc.storage_path = version.storage_path.clone();
        doc.updated_at = Utc::now();

        let record = DocumentVersion {
            id: self.allocate_version_id(),
            document_id: version.document_id,
            tenant_id: version.tenant_id,
            version_number: doc.current_version,
            filename: version.filename,
            size_bytes: version.size_bytes,
            hash: version.hash,
            storage_path: version.storage_path,
            created_by: version.created_by,
            comment: version.comment,
            created_at: Utc::now(),
        };
        self.versions.lock().push(record.clone());
        Ok(record)
    }

    async fn list_versions(
        &self,
        document_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<DocumentVersion>, ApiError> {
        Ok(self
            .versions
            .lock()
            .iter()
            .filter(|v| v.document_id == document_id && v.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn get_version(
        &self,
        version_id: i64,
        tenant_id: i64,
    ) -> Result<Option<DocumentVersion>, ApiError> {
        Ok(self
            .versions
            .lock()
            .iter()
            .find(|v| v.id == version_id && v.tenant_id == tenant_id)
            .cloned())
    }

    // --- Categories ---

    async fn create_category(
        &self,
        category: CreateDocumentCategory,
    ) -> Result<DocumentCategory, ApiError> {
        let now = Utc::now();
        let record = DocumentCategory {
            id: self.allocate_category_id(),
            tenant_id: category.tenant_id,
            name: category.name,
            description: category.description,
            color: category.color,
            parent_id: category.parent_id,
            created_at: now,
            updated_at: now,
        };
        self.categories.lock().push(record.clone());
        Ok(record)
    }

    async fn find_category_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<DocumentCategory>, ApiError> {
        Ok(self
            .categories
            .lock()
            .iter()
            .find(|c| c.id == id && c.tenant_id == tenant_id)
            .cloned())
    }

    async fn list_categories(&self, tenant_id: i64) -> Result<Vec<DocumentCategory>, ApiError> {
        Ok(self
            .categories
            .lock()
            .iter()
            .filter(|c| c.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn update_category(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateDocumentCategory,
    ) -> Result<DocumentCategory, ApiError> {
        let mut cats = self.categories.lock();
        let cat = cats
            .iter_mut()
            .find(|c| c.id == id && c.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Category {} not found", id)))?;

        if let Some(name) = update.name {
            cat.name = name;
        }
        if let Some(description) = update.description {
            cat.description = Some(description);
        }
        if let Some(color) = update.color {
            cat.color = Some(color);
        }
        if let Some(parent_id) = update.parent_id {
            cat.parent_id = Some(parent_id);
        }
        cat.updated_at = Utc::now();

        Ok(cat.clone())
    }

    async fn delete_category(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut cats = self.categories.lock();
        let pos = cats
            .iter()
            .position(|c| c.id == id && c.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Category {} not found", id)))?;
        cats.remove(pos);

        // Unlink documents from this category
        let mut docs = self.documents.lock();
        for doc in docs.iter_mut().filter(|d| d.tenant_id == tenant_id) {
            if doc.category_id == Some(id) {
                doc.category_id = None;
            }
        }

        Ok(())
    }

    // --- Entity Links ---

    async fn create_link(&self, link: CreateDocumentLink) -> Result<DocumentLink, ApiError> {
        let record = DocumentLink {
            id: self.allocate_link_id(),
            document_id: link.document_id,
            tenant_id: link.tenant_id,
            entity_type: link.entity_type,
            entity_id: link.entity_id,
            created_at: Utc::now(),
        };
        self.links.lock().push(record.clone());
        Ok(record)
    }

    async fn delete_link(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut links = self.links.lock();
        let pos = links
            .iter()
            .position(|l| l.id == id && l.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Link {} not found", id)))?;
        links.remove(pos);
        Ok(())
    }

    async fn list_links_by_document(
        &self,
        document_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<DocumentLink>, ApiError> {
        Ok(self
            .links
            .lock()
            .iter()
            .filter(|l| l.document_id == document_id && l.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn list_links_by_entity(
        &self,
        tenant_id: i64,
        entity_type: &str,
        entity_id: i64,
    ) -> Result<Vec<DocumentLink>, ApiError> {
        Ok(self
            .links
            .lock()
            .iter()
            .filter(|l| {
                l.tenant_id == tenant_id && l.entity_type == entity_type && l.entity_id == entity_id
            })
            .cloned()
            .collect())
    }

    async fn find_documents_by_entity(
        &self,
        tenant_id: i64,
        entity_type: &str,
        entity_id: i64,
    ) -> Result<Vec<Document>, ApiError> {
        let links = self.links.lock();
        let docs = self.documents.lock();

        let linked_doc_ids: Vec<i64> = links
            .iter()
            .filter(|l| {
                l.tenant_id == tenant_id && l.entity_type == entity_type && l.entity_id == entity_id
            })
            .map(|l| l.document_id)
            .collect();

        Ok(docs
            .iter()
            .filter(|d| {
                d.tenant_id == tenant_id && d.deleted_at.is_none() && linked_doc_ids.contains(&d.id)
            })
            .cloned()
            .collect())
    }
}
