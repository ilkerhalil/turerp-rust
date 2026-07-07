//! E-Archive service — business logic for Turkish e-Arşiv Fatura and E-Serbest Meslek Makbuzu
//!
//! Orchestrates E-Archive document lifecycle: generation, signing,
//! GİB submission, status checking, and cancellation.

use chrono::Utc;

use crate::common::pagination::{PaginatedResult, PaginationParams};
use crate::domain::earchive::model::{
    CreateEarchiveDocument, EarchiveDocument, EarchiveStatus, EarchiveType,
};
use crate::domain::earchive::repository::BoxEarchiveRepository;
use crate::domain::invoice::repository::BoxInvoiceRepository;
use crate::error::ApiError;

/// Service for managing E-Archive documents and GİB integration
#[derive(Clone)]
pub struct EarchiveService {
    repo: BoxEarchiveRepository,
    invoice_repo: BoxInvoiceRepository,
}

impl EarchiveService {
    pub fn new(repo: BoxEarchiveRepository, invoice_repo: BoxInvoiceRepository) -> Self {
        Self { repo, invoice_repo }
    }

    /// Generate an E-Archive document from an invoice.
    #[tracing::instrument(skip(self))]
    pub async fn generate_earchive(
        &self,
        tenant_id: i64,
        invoice_id: i64,
        document_type: EarchiveType,
    ) -> Result<EarchiveDocument, ApiError> {
        // Parent-ownership precheck: the source invoice must belong to the
        // caller's tenant (cross-tenant orphan-FK IDOR, issue #301). The
        // invoice is not otherwise loaded — this gate prevents a tenant from
        // stamping an E-Archive (and embedding an invoice id in the UBL-TR
        // XML) onto a foreign-tenant invoice id.
        self.invoice_repo
            .find_by_id(invoice_id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Invoice {} not found", invoice_id)))?;
        let uuid = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();

        let xml_content = match document_type {
            EarchiveType::EArchiveInvoice => self.generate_ubl_tr_xml(invoice_id, &uuid, now),
            EarchiveType::ESerbestMeslekMakbuzu => self.generate_smm_xml(invoice_id, &uuid, now),
        };

        let doc = EarchiveDocument {
            id: 0,
            tenant_id,
            document_type,
            related_invoice_id: Some(invoice_id),
            uuid,
            xml_content,
            signature: None,
            status: EarchiveStatus::Generated,
            gib_response: None,
            error_message: None,
            created_at: now,
            updated_at: now,
            sent_at: None,
        };

        self.repo.create(doc).await
    }

    /// Generate a simple UBL-TR XML for e-Arşiv Fatura
    fn generate_ubl_tr_xml(
        &self,
        invoice_id: i64,
        uuid: &str,
        issue_date: chrono::DateTime<Utc>,
    ) -> String {
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <Invoice xmlns=\"urn:oasis:names:specification:ubl:schema:xsd:Invoice-2\"\n\
             xmlns:cac=\"urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2\"\n\
             xmlns:cbc=\"urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2\"\n\
             xmlns:ext=\"urn:oasis:names:specification:ubl:schema:xsd:CommonExtensionComponents-2\"\n\
             xmlns:ds=\"http://www.w3.org/2000/09/xmldsig#\"\n\
             xmlns:xades=\"http://uri.etsi.org/01903/v1.3.2#\"\n\
             xmlns:schemaLocation=\"urn:oasis:names:specification:ubl:schema:xsd:Invoice-2 ../xsd/maindoc/UBL-Invoice-2.1.xsd\">\n\
             <cbc:UBLVersionID>2.1</cbc:UBLVersionID>\n\
             <cbc:CustomizationID>TR1.2</cbc:CustomizationID>\n\
             <cbc:ProfileID>EARSIVFATURA</cbc:ProfileID>\n\
             <cbc:ID>{invoice_id}</cbc:ID>\n\
             <cbc:CopyIndicator>false</cbc:CopyIndicator>\n\
             <cbc:UUID>{uuid}</cbc:UUID>\n\
             <cbc:IssueDate>{date}</cbc:IssueDate>\n\
             <cbc:IssueTime>{time}</cbc:IssueTime>\n\
             <cbc:InvoiceTypeCode>SATIS</cbc:InvoiceTypeCode>\n\
             <cbc:DocumentCurrencyCode>TRY</cbc:DocumentCurrencyCode>\n\
             <cbc:LineCountNumeric>0</cbc:LineCountNumeric>\n\
             <cac:AccountingSupplierParty>\n\
             <cac:Party>\n\
             <cac:PartyName>\n\
             <cbc:Name>Sender Company</cbc:Name>\n\
             </cac:PartyName>\n\
             </cac:Party>\n\
             </cac:AccountingSupplierParty>\n\
             <cac:AccountingCustomerParty>\n\
             <cac:Party>\n\
             <cac:PartyName>\n\
             <cbc:Name>Customer Company</cbc:Name>\n\
             </cac:PartyName>\n\
             </cac:Party>\n\
             </cac:AccountingCustomerParty>\n\
             <cac:LegalMonetaryTotal>\n\
             <cbc:TaxExclusiveAmount currencyID=\"TRY\">0.00</cbc:TaxExclusiveAmount>\n\
             <cbc:TaxInclusiveAmount currencyID=\"TRY\">0.00</cbc:TaxInclusiveAmount>\n\
             <cbc:PayableAmount currencyID=\"TRY\">0.00</cbc:PayableAmount>\n\
             </cac:LegalMonetaryTotal>\n\
             </Invoice>",
            invoice_id = invoice_id,
            uuid = uuid,
            date = issue_date.format("%Y-%m-%d"),
            time = issue_date.format("%H:%M:%S"),
        )
    }

    /// Generate a simple XML for E-Serbest Meslek Makbuzu
    fn generate_smm_xml(
        &self,
        invoice_id: i64,
        uuid: &str,
        issue_date: chrono::DateTime<Utc>,
    ) -> String {
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <SerbestMeslekMakbuzu xmlns=\"http://www.smm.efatura.gov.tr\"\n\
             xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"\n\
             xsi:schemaLocation=\"http://www.smm.efatura.gov.tr smm.xsd\">\n\
             <MakbuzuNo>{invoice_id}</MakbuzuNo>\n\
             <UUID>{uuid}</UUID>\n\
             <Tarih>{date}</Tarih>\n\
             <Saat>{time}</Saat>\n\
             <VergiKimlikNo>00000000000</VergiKimlikNo>\n\
             <AdSoyad>Test Professional</AdSoyad>\n\
             <BrutUcret>0.00</BrutUcret>\n\
             <GelirVergisiKesintisi>0.00</GelirVergisiKesintisi>\n\
             <NetUcret>0.00</NetUcret>\n\
             </SerbestMeslekMakbuzu>",
            invoice_id = invoice_id,
            uuid = uuid,
            date = issue_date.format("%Y-%m-%d"),
            time = issue_date.format("%H:%M:%S"),
        )
    }

    /// Sign an E-Archive document (mock signing)
    #[tracing::instrument(skip(self))]
    pub async fn sign_document(
        &self,
        tenant_id: i64,
        document_id: i64,
    ) -> Result<EarchiveDocument, ApiError> {
        let doc = self.get_document(tenant_id, document_id).await?;

        if doc.status != EarchiveStatus::Generated && doc.status != EarchiveStatus::Draft {
            return Err(ApiError::BadRequest(format!(
                "Cannot sign document in {} status; must be Generated or Draft",
                doc.status
            )));
        }

        let signature = format!(
            "SIGNED:{uuid}:{timestamp}",
            uuid = doc.uuid,
            timestamp = Utc::now().timestamp()
        );

        self.repo
            .update_status(
                document_id,
                tenant_id,
                EarchiveStatus::Signed,
                None,
                None,
                None,
            )
            .await
            .map(|mut d| {
                d.signature = Some(signature);
                d
            })
    }

    /// Send an E-Archive document to GİB (mock)
    #[tracing::instrument(skip(self))]
    pub async fn send_to_gib(
        &self,
        tenant_id: i64,
        document_id: i64,
    ) -> Result<EarchiveDocument, ApiError> {
        let doc = self.get_document(tenant_id, document_id).await?;

        if doc.status != EarchiveStatus::Signed {
            return Err(ApiError::BadRequest(format!(
                "Cannot send document in {} status; must be Signed",
                doc.status
            )));
        }

        // Mock GİB integration - simulate successful send
        let gib_response = format!("GIB_OK:{}", doc.uuid);
        let sent_at = Some(Utc::now());

        self.repo
            .update_status(
                document_id,
                tenant_id,
                EarchiveStatus::Sent,
                Some(gib_response),
                None,
                sent_at,
            )
            .await
    }

    /// Get an E-Archive document by ID
    #[tracing::instrument(skip(self))]
    pub async fn get_document(
        &self,
        tenant_id: i64,
        document_id: i64,
    ) -> Result<EarchiveDocument, ApiError> {
        self.repo
            .find_by_id(document_id, tenant_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!("E-Archive document {} not found", document_id))
            })
    }

    /// List E-Archive documents with optional status filter and pagination
    #[tracing::instrument(skip(self))]
    pub async fn list_documents(
        &self,
        tenant_id: i64,
        status: Option<EarchiveStatus>,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<EarchiveDocument>, ApiError> {
        let params = PaginationParams { page, per_page };
        self.repo.find_by_tenant(tenant_id, status, params).await
    }

    /// Cancel an E-Archive document
    #[tracing::instrument(skip(self))]
    pub async fn cancel_document(
        &self,
        tenant_id: i64,
        document_id: i64,
    ) -> Result<EarchiveDocument, ApiError> {
        let doc = self.get_document(tenant_id, document_id).await?;

        if doc.status == EarchiveStatus::Cancelled {
            return Err(ApiError::BadRequest(format!(
                "E-Archive document {} is already cancelled",
                document_id
            )));
        }

        self.repo
            .update_status(
                document_id,
                tenant_id,
                EarchiveStatus::Cancelled,
                Some("CANCELLED".to_string()),
                None,
                None,
            )
            .await
    }

    /// Create a document directly from a request (for testing/advanced use)
    #[tracing::instrument(skip(self))]
    pub async fn create_document(
        &self,
        tenant_id: i64,
        create: CreateEarchiveDocument,
    ) -> Result<EarchiveDocument, ApiError> {
        let uuid = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();

        let doc = EarchiveDocument {
            id: 0,
            tenant_id,
            document_type: create.document_type,
            related_invoice_id: create.related_invoice_id,
            uuid,
            xml_content: create.xml_content,
            signature: None,
            status: EarchiveStatus::Draft,
            gib_response: None,
            error_message: None,
            created_at: now,
            updated_at: now,
            sent_at: None,
        };

        self.repo.create(doc).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::earchive::repository::InMemoryEarchiveRepository;
    use crate::domain::invoice::model::{CreateInvoice, CreateInvoiceLine, InvoiceType};
    use crate::domain::invoice::repository::InMemoryInvoiceRepository;
    use chrono::Utc;
    use rust_decimal::Decimal;
    use std::sync::Arc;

    /// Minimal owned-tenant invoice for seeding the parent-FK precheck.
    fn seed_invoice(tenant_id: i64) -> CreateInvoice {
        CreateInvoice {
            tenant_id,
            company_id: 1,
            invoice_type: InvoiceType::SalesInvoice,
            cari_id: 1,
            issue_date: Utc::now(),
            due_date: Utc::now() + chrono::Duration::days(7),
            currency: "TRY".to_string(),
            exchange_rate: Decimal::ONE,
            notes: None,
            cost_center_id: None,
            lines: vec![CreateInvoiceLine {
                product_id: None,
                description: "Seed line".to_string(),
                quantity: Decimal::new(1, 0),
                unit_price: Decimal::new(10, 0),
                tax_rate: Decimal::ZERO,
                discount_rate: Decimal::ZERO,
            }],
        }
    }

    /// Build a service with a seeded invoice repo: tenant-1 invoices auto-id
    /// to 1 and 2 (the own-tenant referents the existing tests pass to
    /// `generate_earchive`), plus a tenant-2 invoice auto-id to 3 (the foreign
    /// referent for the cross-tenant IDOR rejection test). The
    /// `generate_earchive` invoice_id precheck (#301) resolves for own-tenant
    /// ids 1/2 and 404s for the foreign id 3.
    async fn make_service() -> EarchiveService {
        let repo = Arc::new(InMemoryEarchiveRepository::new()) as BoxEarchiveRepository;
        let invoice_repo = Arc::new(InMemoryInvoiceRepository::new()) as BoxInvoiceRepository;
        invoice_repo.create(seed_invoice(1)).await.unwrap(); // id 1 (tenant 1)
        invoice_repo.create(seed_invoice(1)).await.unwrap(); // id 2 (tenant 1)
        invoice_repo.create(seed_invoice(2)).await.unwrap(); // id 3 (tenant 2)
        EarchiveService::new(repo, invoice_repo)
    }

    #[tokio::test]
    async fn test_generate_earchive_invoice() {
        let svc = make_service().await;

        let doc = svc
            .generate_earchive(1, 1, EarchiveType::EArchiveInvoice)
            .await
            .unwrap();

        assert_eq!(doc.tenant_id, 1);
        assert_eq!(doc.related_invoice_id, Some(1));
        assert_eq!(doc.document_type, EarchiveType::EArchiveInvoice);
        assert_eq!(doc.status, EarchiveStatus::Generated);
        assert!(!doc.uuid.is_empty());
        assert!(doc.xml_content.contains("EARSIVFATURA"));
        assert!(doc.xml_content.contains(&doc.uuid));
    }

    #[tokio::test]
    async fn test_generate_smm() {
        let svc = make_service().await;

        let doc = svc
            .generate_earchive(1, 1, EarchiveType::ESerbestMeslekMakbuzu)
            .await
            .unwrap();

        assert_eq!(doc.document_type, EarchiveType::ESerbestMeslekMakbuzu);
        assert_eq!(doc.status, EarchiveStatus::Generated);
        assert!(doc.xml_content.contains("SerbestMeslekMakbuzu"));
    }

    #[tokio::test]
    async fn test_get_document() {
        let svc = make_service().await;
        let doc = svc
            .generate_earchive(1, 1, EarchiveType::EArchiveInvoice)
            .await
            .unwrap();

        let found = svc.get_document(1, doc.id).await.unwrap();
        assert_eq!(found.id, doc.id);

        let result = svc.get_document(9999, 1).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_documents() {
        let svc = make_service().await;

        svc.generate_earchive(1, 1, EarchiveType::EArchiveInvoice)
            .await
            .unwrap();
        svc.generate_earchive(1, 2, EarchiveType::ESerbestMeslekMakbuzu)
            .await
            .unwrap();

        let all = svc.list_documents(1, None, 1, 20).await.unwrap();
        assert_eq!(all.items.len(), 2);

        let generated = svc
            .list_documents(1, Some(EarchiveStatus::Generated), 1, 20)
            .await
            .unwrap();
        assert_eq!(generated.items.len(), 2);

        let sent = svc
            .list_documents(1, Some(EarchiveStatus::Sent), 1, 20)
            .await
            .unwrap();
        assert_eq!(sent.items.len(), 0);
    }

    #[tokio::test]
    async fn test_sign_document() {
        let svc = make_service().await;
        let doc = svc
            .generate_earchive(1, 1, EarchiveType::EArchiveInvoice)
            .await
            .unwrap();

        let signed = svc.sign_document(1, doc.id).await.unwrap();
        assert_eq!(signed.status, EarchiveStatus::Signed);
        assert!(signed.signature.is_some());
        assert!(signed.signature.as_ref().unwrap().starts_with("SIGNED:"));
    }

    #[tokio::test]
    async fn test_sign_non_generated_fails() {
        let svc = make_service().await;
        let doc = svc
            .generate_earchive(1, 1, EarchiveType::EArchiveInvoice)
            .await
            .unwrap();

        // Sign once
        svc.sign_document(1, doc.id).await.unwrap();

        // Sign again should fail
        let result = svc.sign_document(1, doc.id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_send_to_gib() {
        let svc = make_service().await;
        let doc = svc
            .generate_earchive(1, 1, EarchiveType::EArchiveInvoice)
            .await
            .unwrap();

        // Must sign first
        svc.sign_document(1, doc.id).await.unwrap();

        let sent = svc.send_to_gib(1, doc.id).await.unwrap();
        assert_eq!(sent.status, EarchiveStatus::Sent);
        assert!(sent.gib_response.is_some());
        assert!(sent.sent_at.is_some());
    }

    #[tokio::test]
    async fn test_send_without_sign_fails() {
        let svc = make_service().await;
        let doc = svc
            .generate_earchive(1, 1, EarchiveType::EArchiveInvoice)
            .await
            .unwrap();

        let result = svc.send_to_gib(1, doc.id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cancel_document() {
        let svc = make_service().await;
        let doc = svc
            .generate_earchive(1, 1, EarchiveType::EArchiveInvoice)
            .await
            .unwrap();

        let cancelled = svc.cancel_document(1, doc.id).await.unwrap();
        assert_eq!(cancelled.status, EarchiveStatus::Cancelled);
    }

    #[tokio::test]
    async fn test_cancel_already_cancelled_fails() {
        let svc = make_service().await;
        let doc = svc
            .generate_earchive(1, 1, EarchiveType::EArchiveInvoice)
            .await
            .unwrap();

        svc.cancel_document(1, doc.id).await.unwrap();

        let result = svc.cancel_document(1, doc.id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_tenant_isolation() {
        let svc = make_service().await;
        let doc = svc
            .generate_earchive(1, 1, EarchiveType::EArchiveInvoice)
            .await
            .unwrap();

        let result = svc.get_document(999, doc.id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_xml_generation_ubl_tr() {
        let svc = make_service().await;
        let now = Utc::now();
        let xml = svc.generate_ubl_tr_xml(42, "test-uuid", now);

        assert!(xml.contains("EARSIVFATURA"));
        assert!(xml.contains("test-uuid"));
        assert!(xml.contains("42"));
        assert!(xml.contains("AccountingSupplierParty"));
        assert!(xml.contains("LegalMonetaryTotal"));
    }

    #[tokio::test]
    async fn test_xml_generation_smm() {
        let svc = make_service().await;
        let now = Utc::now();
        let xml = svc.generate_smm_xml(42, "test-uuid", now);

        assert!(xml.contains("SerbestMeslekMakbuzu"));
        assert!(xml.contains("test-uuid"));
        assert!(xml.contains("42"));
    }

    /// Rejects an E-Archive stamped onto a foreign-tenant invoice (orphan-FK
    /// IDOR, issue #301). Own-tenant invoice id 1 succeeds; the tenant-2
    /// invoice id 3 and a nonexistent id 999 both 404.
    #[tokio::test]
    async fn test_generate_earchive_rejects_foreign_invoice() {
        let svc = make_service().await;

        // Own-tenant invoice (id 1, tenant 1) → ok.
        assert!(
            svc.generate_earchive(1, 1, EarchiveType::EArchiveInvoice)
                .await
                .is_ok(),
            "own-tenant invoice must succeed"
        );

        // Foreign-tenant invoice (id 3, belongs to tenant 2) → NotFound.
        let result = svc
            .generate_earchive(1, 3, EarchiveType::EArchiveInvoice)
            .await;
        assert!(
            matches!(result, Err(ApiError::NotFound(_))),
            "tenant-1 must NOT generate an E-Archive for a tenant-2 invoice, got {:?}",
            result
        );

        // Nonexistent invoice → NotFound.
        let result = svc
            .generate_earchive(1, 999, EarchiveType::EArchiveInvoice)
            .await;
        assert!(
            matches!(result, Err(ApiError::NotFound(_))),
            "nonexistent invoice must be NotFound, got {:?}",
            result
        );
    }
}
