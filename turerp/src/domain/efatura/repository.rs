//! e-Fatura repository trait and in-memory implementation

use async_trait::async_trait;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use crate::common::pagination::{PaginatedResult, PaginationParams};
use crate::domain::efatura::model::{EFatura, EFaturaStatus};
use crate::error::ApiError;

// ---------------------------------------------------------------------------
// EFaturaRepository
// ---------------------------------------------------------------------------

/// Repository trait for e-Fatura operations
#[async_trait]
pub trait EFaturaRepository: Send + Sync {
    /// Create a new e-Fatura document
    async fn create(&self, fatura: EFatura) -> Result<EFatura, ApiError>;

    /// Find an e-Fatura document by ID
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<EFatura>, ApiError>;

    /// Find an e-Fatura document by UUID
    async fn find_by_uuid(&self, uuid: &str, tenant_id: i64) -> Result<Option<EFatura>, ApiError>;

    /// Find an e-Fatura document by invoice ID
    async fn find_by_invoice_id(
        &self,
        invoice_id: i64,
        tenant_id: i64,
    ) -> Result<Option<EFatura>, ApiError>;

    /// Find all e-Fatura documents with optional status filter and pagination
    async fn find_all(
        &self,
        tenant_id: i64,
        status: Option<EFaturaStatus>,
        params: PaginationParams,
    ) -> Result<PaginatedResult<EFatura>, ApiError>;

    /// Update the status of an e-Fatura document
    async fn update_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: EFaturaStatus,
        response_code: Option<String>,
        response_desc: Option<String>,
    ) -> Result<EFatura, ApiError>;

    /// Update the XML content of an e-Fatura document
    async fn update_xml(
        &self,
        id: i64,
        tenant_id: i64,
        xml_content: String,
    ) -> Result<EFatura, ApiError>;

    /// Soft-delete an e-Fatura document
    async fn soft_delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
}

/// Type alias for boxed EFaturaRepository
pub type BoxEFaturaRepository = Arc<dyn EFaturaRepository>;

// ---------------------------------------------------------------------------
// InMemoryEFaturaRepository
// ---------------------------------------------------------------------------

struct Inner {
    faturas: HashMap<i64, EFatura>,
    next_id: AtomicI64,
}

/// In-memory e-Fatura repository for testing and development
pub struct InMemoryEFaturaRepository {
    inner: Mutex<Inner>,
}

impl InMemoryEFaturaRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                faturas: HashMap::new(),
                next_id: AtomicI64::new(1),
            }),
        }
    }
}

impl Default for InMemoryEFaturaRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EFaturaRepository for InMemoryEFaturaRepository {
    async fn create(&self, fatura: EFatura) -> Result<EFatura, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_id.fetch_add(1, Ordering::SeqCst);
        let now = chrono::Utc::now();

        let stored = EFatura {
            id,
            tenant_id: fatura.tenant_id,
            invoice_id: fatura.invoice_id,
            uuid: fatura.uuid,
            document_number: fatura.document_number,
            issue_date: fatura.issue_date,
            profile_id: fatura.profile_id,
            sender: fatura.sender,
            receiver: fatura.receiver,
            lines: fatura.lines,
            tax_totals: fatura.tax_totals,
            legal_monetary_total: fatura.legal_monetary_total,
            status: fatura.status,
            response_code: fatura.response_code,
            response_desc: fatura.response_desc,
            xml_content: fatura.xml_content,
            created_at: now,
            updated_at: now,
        };

        inner.faturas.insert(id, stored.clone());
        Ok(stored)
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<EFatura>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .faturas
            .get(&id)
            .filter(|f| f.tenant_id == tenant_id)
            .cloned())
    }

    async fn find_by_uuid(&self, uuid: &str, tenant_id: i64) -> Result<Option<EFatura>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .faturas
            .values()
            .find(|f| f.tenant_id == tenant_id && f.uuid == uuid)
            .cloned())
    }

    async fn find_by_invoice_id(
        &self,
        invoice_id: i64,
        tenant_id: i64,
    ) -> Result<Option<EFatura>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .faturas
            .values()
            .find(|f| f.tenant_id == tenant_id && f.invoice_id == Some(invoice_id))
            .cloned())
    }

    async fn find_all(
        &self,
        tenant_id: i64,
        status: Option<EFaturaStatus>,
        params: PaginationParams,
    ) -> Result<PaginatedResult<EFatura>, ApiError> {
        let inner = self.inner.lock();
        let mut items: Vec<EFatura> = inner
            .faturas
            .values()
            .filter(|f| f.tenant_id == tenant_id)
            .filter(|f| match &status {
                Some(s) => f.status == *s,
                None => true,
            })
            .cloned()
            .collect();

        items.sort_by(|a, b| a.id.cmp(&b.id));
        let total = items.len() as u64;
        let start = (params.page.saturating_sub(1)) * params.per_page;
        let paginated: Vec<EFatura> = items
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
        status: EFaturaStatus,
        response_code: Option<String>,
        response_desc: Option<String>,
    ) -> Result<EFatura, ApiError> {
        let mut inner = self.inner.lock();

        let fatura = inner
            .faturas
            .get_mut(&id)
            .filter(|f| f.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("e-Fatura {} not found", id)))?;

        fatura.status = status;
        fatura.response_code = response_code;
        fatura.response_desc = response_desc;
        fatura.updated_at = chrono::Utc::now();

        Ok(fatura.clone())
    }

    async fn update_xml(
        &self,
        id: i64,
        tenant_id: i64,
        xml_content: String,
    ) -> Result<EFatura, ApiError> {
        let mut inner = self.inner.lock();

        let fatura = inner
            .faturas
            .get_mut(&id)
            .filter(|f| f.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("e-Fatura {} not found", id)))?;

        fatura.xml_content = Some(xml_content);
        fatura.updated_at = chrono::Utc::now();

        Ok(fatura.clone())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();

        let fatura = inner
            .faturas
            .get(&id)
            .filter(|f| f.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("e-Fatura {} not found", id)))?;

        let key = fatura.id;
        inner.faturas.remove(&key);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::efatura::model::{
        AddressInfo, EFaturaProfile, EFaturaStatus, MonetaryTotal, PartyInfo,
    };
    use chrono::NaiveDate;
    use rust_decimal::Decimal;

    fn sample_fatura(tenant_id: i64) -> EFatura {
        EFatura {
            id: 0,
            tenant_id,
            invoice_id: Some(1),
            uuid: "test-uuid-1".to_string(),
            document_number: "FTR-2024-001".to_string(),
            issue_date: NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            profile_id: EFaturaProfile::TemelFatura,
            sender: PartyInfo {
                vkn_tckn: "1234567890".to_string(),
                name: "Sender Corp".to_string(),
                tax_office: "Kadikoy".to_string(),
                address: AddressInfo {
                    street: "Main St 1".to_string(),
                    district: None,
                    city: "Istanbul".to_string(),
                    country: None,
                    postal_code: None,
                },
                email: None,
                phone: None,
                register_number: None,
                mersis_number: None,
            },
            receiver: PartyInfo {
                vkn_tckn: "9876543210".to_string(),
                name: "Receiver Ltd".to_string(),
                tax_office: "Uskudar".to_string(),
                address: AddressInfo {
                    street: "Side St 2".to_string(),
                    district: None,
                    city: "Istanbul".to_string(),
                    country: None,
                    postal_code: None,
                },
                email: None,
                phone: None,
                register_number: None,
                mersis_number: None,
            },
            lines: vec![],
            tax_totals: vec![],
            legal_monetary_total: MonetaryTotal {
                line_extension_amount: Decimal::new(10000, 2),
                tax_exclusive_amount: Decimal::new(10000, 2),
                tax_inclusive_amount: Decimal::new(11800, 2),
                allowance_total_amount: None,
                payable_amount: Decimal::new(11800, 2),
            },
            status: EFaturaStatus::Draft,
            response_code: None,
            response_desc: None,
            xml_content: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_efatura_crud() {
        let repo = InMemoryEFaturaRepository::new();

        // Create
        let fatura = repo.create(sample_fatura(1)).await.unwrap();
        assert_eq!(fatura.id, 1);
        assert_eq!(fatura.tenant_id, 1);
        assert_eq!(fatura.status, EFaturaStatus::Draft);

        // Find by ID
        let found = repo.find_by_id(1, 1).await.unwrap().unwrap();
        assert_eq!(found.id, fatura.id);

        // Not found for different tenant
        let not_found = repo.find_by_id(1, 999).await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_find_by_uuid() {
        let repo = InMemoryEFaturaRepository::new();

        let fatura = repo.create(sample_fatura(1)).await.unwrap();

        let found = repo.find_by_uuid("test-uuid-1", 1).await.unwrap().unwrap();
        assert_eq!(found.id, fatura.id);

        let not_found = repo.find_by_uuid("nonexistent", 1).await.unwrap();
        assert!(not_found.is_none());

        let wrong_tenant = repo.find_by_uuid("test-uuid-1", 999).await.unwrap();
        assert!(wrong_tenant.is_none());
    }

    #[tokio::test]
    async fn test_find_by_invoice_id() {
        let repo = InMemoryEFaturaRepository::new();

        repo.create(sample_fatura(1)).await.unwrap();

        let found = repo.find_by_invoice_id(1, 1).await.unwrap().unwrap();
        assert_eq!(found.uuid, "test-uuid-1");

        let not_found = repo.find_by_invoice_id(999, 1).await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_find_all_with_status_filter() {
        let repo = InMemoryEFaturaRepository::new();

        repo.create(sample_fatura(1)).await.unwrap();

        let mut fatura2 = sample_fatura(1);
        fatura2.uuid = "test-uuid-2".to_string();
        fatura2.document_number = "FTR-2024-002".to_string();
        fatura2.status = EFaturaStatus::Sent;
        repo.create(fatura2).await.unwrap();

        // All
        let all = repo
            .find_all(1, None, PaginationParams::default())
            .await
            .unwrap();
        assert_eq!(all.items.len(), 2);

        // Filter by status
        let sent = repo
            .find_all(1, Some(EFaturaStatus::Sent), PaginationParams::default())
            .await
            .unwrap();
        assert_eq!(sent.items.len(), 1);
        assert_eq!(sent.items[0].status, EFaturaStatus::Sent);

        // Different tenant
        let other = repo
            .find_all(999, None, PaginationParams::default())
            .await
            .unwrap();
        assert_eq!(other.items.len(), 0);
    }

    #[tokio::test]
    async fn test_update_status() {
        let repo = InMemoryEFaturaRepository::new();

        repo.create(sample_fatura(1)).await.unwrap();

        let updated = repo
            .update_status(
                1,
                1,
                EFaturaStatus::Accepted,
                Some("200".to_string()),
                Some("OK".to_string()),
            )
            .await
            .unwrap();
        assert_eq!(updated.status, EFaturaStatus::Accepted);
        assert_eq!(updated.response_code, Some("200".to_string()));
        assert_eq!(updated.response_desc, Some("OK".to_string()));

        // Not found for different tenant
        let result = repo
            .update_status(1, 999, EFaturaStatus::Rejected, None, None)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_xml() {
        let repo = InMemoryEFaturaRepository::new();

        repo.create(sample_fatura(1)).await.unwrap();

        let xml = "<Invoice>...</Invoice>".to_string();
        let updated = repo.update_xml(1, 1, xml.clone()).await.unwrap();
        assert_eq!(updated.xml_content, Some(xml));
    }

    #[tokio::test]
    async fn test_soft_delete() {
        let repo = InMemoryEFaturaRepository::new();

        repo.create(sample_fatura(1)).await.unwrap();

        repo.soft_delete(1, 1).await.unwrap();

        let gone = repo.find_by_id(1, 1).await.unwrap();
        assert!(gone.is_none());

        // Not found for different tenant
        let result = repo.soft_delete(999, 1).await;
        assert!(result.is_err());
    }
}
