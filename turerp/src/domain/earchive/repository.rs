//! E-Archive repository trait and in-memory implementation

use async_trait::async_trait;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use crate::common::pagination::{PaginatedResult, PaginationParams};
use crate::domain::earchive::model::{EarchiveDocument, EarchiveStatus};
use crate::error::ApiError;

// ---------------------------------------------------------------------------
// EarchiveRepository
// ---------------------------------------------------------------------------

/// Repository trait for E-Archive operations
#[async_trait]
pub trait EarchiveRepository: Send + Sync {
    /// Create a new E-Archive document
    async fn create(&self, doc: EarchiveDocument) -> Result<EarchiveDocument, ApiError>;

    /// Find an E-Archive document by ID
    async fn find_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<EarchiveDocument>, ApiError>;

    /// Find an E-Archive document by UUID
    async fn find_by_uuid(
        &self,
        uuid: &str,
        tenant_id: i64,
    ) -> Result<Option<EarchiveDocument>, ApiError>;

    /// Find all E-Archive documents with optional status filter and pagination
    async fn find_by_tenant(
        &self,
        tenant_id: i64,
        status: Option<EarchiveStatus>,
        params: PaginationParams,
    ) -> Result<PaginatedResult<EarchiveDocument>, ApiError>;

    /// Update the status of an E-Archive document
    async fn update_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: EarchiveStatus,
        gib_response: Option<String>,
        error_message: Option<String>,
        sent_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<EarchiveDocument, ApiError>;

    /// List documents by status
    async fn list_by_status(
        &self,
        tenant_id: i64,
        status: EarchiveStatus,
        params: PaginationParams,
    ) -> Result<PaginatedResult<EarchiveDocument>, ApiError>;
}

/// Type alias for boxed EarchiveRepository
pub type BoxEarchiveRepository = Arc<dyn EarchiveRepository>;

// ---------------------------------------------------------------------------
// InMemoryEarchiveRepository
// ---------------------------------------------------------------------------

struct Inner {
    docs: HashMap<i64, EarchiveDocument>,
    next_id: AtomicI64,
}

/// In-memory E-Archive repository for testing and development
pub struct InMemoryEarchiveRepository {
    inner: Mutex<Inner>,
}

impl InMemoryEarchiveRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                docs: HashMap::new(),
                next_id: AtomicI64::new(1),
            }),
        }
    }
}

impl Default for InMemoryEarchiveRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EarchiveRepository for InMemoryEarchiveRepository {
    async fn create(&self, doc: EarchiveDocument) -> Result<EarchiveDocument, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_id.fetch_add(1, Ordering::SeqCst);
        let now = chrono::Utc::now();

        let stored = EarchiveDocument {
            id,
            tenant_id: doc.tenant_id,
            document_type: doc.document_type,
            related_invoice_id: doc.related_invoice_id,
            uuid: doc.uuid,
            xml_content: doc.xml_content,
            signature: doc.signature,
            status: doc.status,
            gib_response: doc.gib_response,
            error_message: doc.error_message,
            created_at: now,
            updated_at: now,
            sent_at: doc.sent_at,
        };

        inner.docs.insert(id, stored.clone());
        Ok(stored)
    }

    async fn find_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<EarchiveDocument>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .docs
            .get(&id)
            .filter(|d| d.tenant_id == tenant_id)
            .cloned())
    }

    async fn find_by_uuid(
        &self,
        uuid: &str,
        tenant_id: i64,
    ) -> Result<Option<EarchiveDocument>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .docs
            .values()
            .find(|d| d.tenant_id == tenant_id && d.uuid == uuid)
            .cloned())
    }

    async fn find_by_tenant(
        &self,
        tenant_id: i64,
        status: Option<EarchiveStatus>,
        params: PaginationParams,
    ) -> Result<PaginatedResult<EarchiveDocument>, ApiError> {
        let inner = self.inner.lock();
        let mut items: Vec<EarchiveDocument> = inner
            .docs
            .values()
            .filter(|d| d.tenant_id == tenant_id)
            .filter(|d| match &status {
                Some(s) => d.status == *s,
                None => true,
            })
            .cloned()
            .collect();

        items.sort_by_key(|a| a.id);
        let total = items.len() as u64;
        let start = (params.page.saturating_sub(1)) * params.per_page;
        let paginated: Vec<EarchiveDocument> = items
            .into_iter()
            .skip(start as usize)
            .take(params.per_page as usize)
            .collect();
        Ok(PaginatedResult::new(
            paginated,
            params.page,
            params.per_page,
            total,
        ))
    }

    async fn update_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: EarchiveStatus,
        gib_response: Option<String>,
        error_message: Option<String>,
        sent_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<EarchiveDocument, ApiError> {
        let mut inner = self.inner.lock();

        let doc = inner
            .docs
            .get_mut(&id)
            .filter(|d| d.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("E-Archive document {} not found", id)))?;

        doc.status = status;
        if gib_response.is_some() {
            doc.gib_response = gib_response;
        }
        if error_message.is_some() {
            doc.error_message = error_message;
        }
        if sent_at.is_some() {
            doc.sent_at = sent_at;
        }
        doc.updated_at = chrono::Utc::now();

        Ok(doc.clone())
    }

    async fn list_by_status(
        &self,
        tenant_id: i64,
        status: EarchiveStatus,
        params: PaginationParams,
    ) -> Result<PaginatedResult<EarchiveDocument>, ApiError> {
        self.find_by_tenant(tenant_id, Some(status), params).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::earchive::model::EarchiveType;

    fn sample_doc(tenant_id: i64) -> EarchiveDocument {
        EarchiveDocument {
            id: 0,
            tenant_id,
            document_type: EarchiveType::EArchiveInvoice,
            related_invoice_id: Some(1),
            uuid: "test-uuid-1".to_string(),
            xml_content: "<xml/>".to_string(),
            signature: None,
            status: EarchiveStatus::Draft,
            gib_response: None,
            error_message: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            sent_at: None,
        }
    }

    #[tokio::test]
    async fn test_earchive_crud() {
        let repo = InMemoryEarchiveRepository::new();

        let doc = repo.create(sample_doc(1)).await.unwrap();
        assert_eq!(doc.id, 1);
        assert_eq!(doc.tenant_id, 1);
        assert_eq!(doc.status, EarchiveStatus::Draft);

        let found = repo.find_by_id(1, 1).await.unwrap().unwrap();
        assert_eq!(found.id, doc.id);

        let not_found = repo.find_by_id(1, 999).await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_find_by_uuid() {
        let repo = InMemoryEarchiveRepository::new();

        let doc = repo.create(sample_doc(1)).await.unwrap();

        let found = repo.find_by_uuid("test-uuid-1", 1).await.unwrap().unwrap();
        assert_eq!(found.id, doc.id);

        let not_found = repo.find_by_uuid("nonexistent", 1).await.unwrap();
        assert!(not_found.is_none());

        let wrong_tenant = repo.find_by_uuid("test-uuid-1", 999).await.unwrap();
        assert!(wrong_tenant.is_none());
    }

    #[tokio::test]
    async fn test_find_by_tenant_with_status_filter() {
        let repo = InMemoryEarchiveRepository::new();

        repo.create(sample_doc(1)).await.unwrap();

        let mut doc2 = sample_doc(1);
        doc2.uuid = "test-uuid-2".to_string();
        doc2.status = EarchiveStatus::Sent;
        repo.create(doc2).await.unwrap();

        let all = repo
            .find_by_tenant(1, None, PaginationParams::default())
            .await
            .unwrap();
        assert_eq!(all.items.len(), 2);

        let sent = repo
            .find_by_tenant(1, Some(EarchiveStatus::Sent), PaginationParams::default())
            .await
            .unwrap();
        assert_eq!(sent.items.len(), 1);
        assert_eq!(sent.items[0].status, EarchiveStatus::Sent);

        let other = repo
            .find_by_tenant(999, None, PaginationParams::default())
            .await
            .unwrap();
        assert_eq!(other.items.len(), 0);
    }

    #[tokio::test]
    async fn test_update_status() {
        let repo = InMemoryEarchiveRepository::new();

        repo.create(sample_doc(1)).await.unwrap();

        let updated = repo
            .update_status(
                1,
                1,
                EarchiveStatus::Accepted,
                Some("200".to_string()),
                None,
                Some(chrono::Utc::now()),
            )
            .await
            .unwrap();
        assert_eq!(updated.status, EarchiveStatus::Accepted);
        assert_eq!(updated.gib_response, Some("200".to_string()));
        assert!(updated.sent_at.is_some());

        let result = repo
            .update_status(1, 999, EarchiveStatus::Rejected, None, None, None)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_by_status() {
        let repo = InMemoryEarchiveRepository::new();

        repo.create(sample_doc(1)).await.unwrap();

        let mut doc2 = sample_doc(1);
        doc2.uuid = "test-uuid-2".to_string();
        doc2.status = EarchiveStatus::Signed;
        repo.create(doc2).await.unwrap();

        let drafts = repo
            .list_by_status(1, EarchiveStatus::Draft, PaginationParams::default())
            .await
            .unwrap();
        assert_eq!(drafts.items.len(), 1);

        let signed = repo
            .list_by_status(1, EarchiveStatus::Signed, PaginationParams::default())
            .await
            .unwrap();
        assert_eq!(signed.items.len(), 1);
    }

    #[tokio::test]
    async fn test_tenant_isolation() {
        let repo = InMemoryEarchiveRepository::new();

        repo.create(sample_doc(1)).await.unwrap();

        let found = repo.find_by_id(1, 1).await.unwrap();
        assert!(found.is_some());

        let not_found = repo.find_by_id(1, 2).await.unwrap();
        assert!(not_found.is_none());
    }
}
