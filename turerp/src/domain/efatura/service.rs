//! e-Fatura service — business logic for Turkish electronic invoicing
//!
//! Orchestrates e-Fatura document lifecycle: creation, GIB submission,
//! status checking, cancellation, and XML retrieval.

use chrono::Utc;
use rust_decimal::Decimal;

use crate::common::gov::BoxGibGateway;
use crate::common::pagination::{PaginatedResult, PaginationParams};
use crate::domain::efatura::model::{
    AddressInfo, EFatura, EFaturaProfile, EFaturaStatus, MonetaryTotal, PartyInfo,
};
use crate::domain::efatura::repository::BoxEFaturaRepository;
use crate::domain::invoice::repository::BoxInvoiceRepository;
use crate::error::ApiError;

/// Service for managing e-Fatura documents and GIB integration
#[derive(Clone)]
pub struct EFaturaService {
    repo: BoxEFaturaRepository,
    gib_gateway: BoxGibGateway,
    invoice_repo: BoxInvoiceRepository,
}

impl EFaturaService {
    pub fn new(
        repo: BoxEFaturaRepository,
        gib_gateway: BoxGibGateway,
        invoice_repo: BoxInvoiceRepository,
    ) -> Self {
        Self {
            repo,
            gib_gateway,
            invoice_repo,
        }
    }

    /// Create an e-Fatura from an invoice.
    ///
    /// Since cross-domain invoice lookup is not yet available, this creates
    /// a placeholder e-Fatura with a generated UUID and document number.
    #[tracing::instrument(skip(self))]
    pub async fn create_from_invoice(
        &self,
        invoice_id: i64,
        profile: EFaturaProfile,
        tenant_id: i64,
    ) -> Result<EFatura, ApiError> {
        // Parent-ownership precheck: the source invoice must belong to the
        // caller's tenant (cross-tenant orphan-FK IDOR, issue #300). The
        // invoice is not otherwise loaded — this gate prevents a tenant from
        // stamping an e-Fatura onto a foreign-tenant invoice id.
        self.invoice_repo
            .find_by_id(invoice_id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Invoice {} not found", invoice_id)))?;
        let uuid = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();
        let document_number = format!("EF{}", now.timestamp_millis());

        let fatura = EFatura {
            id: 0,
            tenant_id,
            invoice_id: Some(invoice_id),
            uuid,
            document_number,
            issue_date: now.date_naive(),
            profile_id: profile,
            sender: PartyInfo {
                vkn_tckn: String::new(),
                name: String::new(),
                tax_office: String::new(),
                address: AddressInfo {
                    street: String::new(),
                    district: None,
                    city: String::new(),
                    country: None,
                    postal_code: None,
                },
                email: None,
                phone: None,
                register_number: None,
                mersis_number: None,
            },
            receiver: PartyInfo {
                vkn_tckn: String::new(),
                name: String::new(),
                tax_office: String::new(),
                address: AddressInfo {
                    street: String::new(),
                    district: None,
                    city: String::new(),
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
                line_extension_amount: Decimal::ZERO,
                tax_exclusive_amount: Decimal::ZERO,
                tax_inclusive_amount: Decimal::ZERO,
                allowance_total_amount: None,
                payable_amount: Decimal::ZERO,
            },
            status: EFaturaStatus::Draft,
            response_code: None,
            response_desc: None,
            xml_content: None,
            created_at: now,
            updated_at: now,
        };

        self.repo.create(fatura).await
    }

    /// Get an e-Fatura by ID
    #[tracing::instrument(skip(self))]
    pub async fn get_efatura(&self, id: i64, tenant_id: i64) -> Result<EFatura, ApiError> {
        self.repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("e-Fatura {} not found", id)))
    }

    /// Get an e-Fatura by UUID
    #[tracing::instrument(skip(self))]
    pub async fn get_by_uuid(&self, uuid: &str, tenant_id: i64) -> Result<EFatura, ApiError> {
        self.repo
            .find_by_uuid(uuid, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("e-Fatura with UUID {} not found", uuid)))
    }

    /// List e-Fatura documents with optional status filter and pagination
    #[tracing::instrument(skip(self))]
    pub async fn list_efaturas(
        &self,
        tenant_id: i64,
        status: Option<EFaturaStatus>,
        params: PaginationParams,
    ) -> Result<PaginatedResult<EFatura>, ApiError> {
        self.repo.find_all(tenant_id, status, params).await
    }

    /// Send an e-Fatura to GIB
    #[tracing::instrument(skip(self))]
    pub async fn send_to_gib(&self, id: i64, tenant_id: i64) -> Result<EFatura, ApiError> {
        let fatura = self.get_efatura(id, tenant_id).await?;

        if fatura.status != EFaturaStatus::Draft && fatura.status != EFaturaStatus::Error {
            return Err(ApiError::BadRequest(format!(
                "Cannot send e-Fatura in {} status; must be Draft or Error",
                fatura.status
            )));
        }

        // Build XML content with the e-Fatura UUID so GIB can track it
        let placeholder_xml = format!(
            "<Invoice xmlns=\"urn:oasis:names:specification:ubl:schema:xsd:Invoice-2\">\
             <UUID>{}</UUID></Invoice>",
            fatura.uuid
        );
        let xml = fatura.xml_content.as_deref().unwrap_or(&placeholder_xml);

        match self
            .gib_gateway
            .send_invoice(xml, fatura.profile_id.clone())
            .await
        {
            Ok(result) => {
                // Store the envelope UUID in response_code for later GIB operations
                let gib_uuid = result.envelope_uuid.as_deref().unwrap_or(&fatura.uuid);
                self.repo
                    .update_status(
                        id,
                        tenant_id,
                        EFaturaStatus::Sent,
                        Some(format!("envelope:{}", gib_uuid)),
                        result.message,
                    )
                    .await
            }
            Err(e) => {
                self.repo
                    .update_status(
                        id,
                        tenant_id,
                        EFaturaStatus::Error,
                        Some("SEND_FAILED".to_string()),
                        Some(format!("Failed to send to GIB: {}", e)),
                    )
                    .await
            }
        }
    }

    /// Extract the GIB envelope UUID from the response_code field.
    /// Falls back to the e-Fatura UUID if no envelope UUID is stored.
    fn extract_gib_uuid(fatura: &EFatura) -> String {
        fatura
            .response_code
            .as_deref()
            .and_then(|code| code.strip_prefix("envelope:"))
            .map(|s| s.to_string())
            .unwrap_or_else(|| fatura.uuid.clone())
    }

    /// Check the status of an e-Fatura at GIB by UUID
    #[tracing::instrument(skip(self))]
    pub async fn check_status(&self, uuid: &str, tenant_id: i64) -> Result<EFatura, ApiError> {
        let fatura = self.get_by_uuid(uuid, tenant_id).await?;
        let gib_uuid = Self::extract_gib_uuid(&fatura);

        match self.gib_gateway.check_status(&gib_uuid).await {
            Ok(status) => {
                let new_status = match status.status.as_str() {
                    "Accepted" => EFaturaStatus::Accepted,
                    "Rejected" => EFaturaStatus::Rejected,
                    _ => EFaturaStatus::Sent,
                };
                self.repo
                    .update_status(
                        fatura.id,
                        tenant_id,
                        new_status,
                        status.response_code,
                        status.response_desc,
                    )
                    .await
            }
            Err(e) => Err(e),
        }
    }

    /// Cancel (retract) an e-Fatura
    #[tracing::instrument(skip(self))]
    pub async fn cancel_efatura(
        &self,
        id: i64,
        tenant_id: i64,
        reason: String,
    ) -> Result<EFatura, ApiError> {
        let fatura = self.get_efatura(id, tenant_id).await?;

        if fatura.status == EFaturaStatus::Cancelled {
            return Err(ApiError::BadRequest(format!(
                "e-Fatura {} is already cancelled",
                id
            )));
        }

        // Only sent/accepted invoices can be cancelled via GIB
        if fatura.status == EFaturaStatus::Sent || fatura.status == EFaturaStatus::Accepted {
            let gib_uuid = Self::extract_gib_uuid(&fatura);
            self.gib_gateway.cancel(&gib_uuid, &reason).await?;
        }

        self.repo
            .update_status(
                id,
                tenant_id,
                EFaturaStatus::Cancelled,
                Some("CANCELLED".to_string()),
                Some(reason),
            )
            .await
    }

    /// Get the XML content of an e-Fatura.
    ///
    /// If no stored XML exists, a placeholder is generated.
    #[tracing::instrument(skip(self))]
    pub async fn get_xml(&self, id: i64, tenant_id: i64) -> Result<String, ApiError> {
        let fatura = self.get_efatura(id, tenant_id).await?;

        match fatura.xml_content {
            Some(xml) => Ok(xml),
            None => {
                let placeholder = format!(
                    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
                     <Invoice xmlns=\"urn:oasis:names:specification:ubl:schema:xsd:Invoice-2\"\n\
                     xmlns:cac=\"urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2\"\n\
                     xmlns:cbc=\"urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2\">\n\
                     <cbc:ID>{}</cbc:ID>\n\
                     <cbc:UUID>{}</cbc:UUID>\n\
                     <cbc:IssueDate>{}</cbc:IssueDate>\n\
                     <cbc:ProfileID>{}</cbc:ProfileID>\n\
                     </Invoice>",
                    fatura.document_number,
                    fatura.uuid,
                    fatura.issue_date,
                    fatura.profile_id,
                );

                // Store the generated placeholder for future retrieval
                self.repo
                    .update_xml(id, tenant_id, placeholder.clone())
                    .await?;
                Ok(placeholder)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::efatura::model::EFaturaProfile;
    use crate::domain::efatura::repository::InMemoryEFaturaRepository;
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
    /// to 1 and 2, a tenant-2 invoice auto-ids to 3 (the foreign referent for
    /// the cross-tenant IDOR rejection test). The `create_from_invoice`
    /// invoice_id precheck (#300) resolves for own-tenant ids 1/2 and 404s
    /// for the foreign id 3.
    async fn make_service() -> EFaturaService {
        let repo = Arc::new(InMemoryEFaturaRepository::new()) as BoxEFaturaRepository;
        let gateway = Arc::new(crate::common::gov::InMemoryGibGateway::new()) as BoxGibGateway;
        let invoice_repo = Arc::new(InMemoryInvoiceRepository::new()) as BoxInvoiceRepository;
        invoice_repo.create(seed_invoice(1)).await.unwrap(); // id 1 (tenant 1)
        invoice_repo.create(seed_invoice(1)).await.unwrap(); // id 2 (tenant 1)
        invoice_repo.create(seed_invoice(2)).await.unwrap(); // id 3 (tenant 2)
        EFaturaService::new(repo, gateway, invoice_repo)
    }

    #[tokio::test]
    async fn test_create_from_invoice() {
        let svc = make_service().await;

        let fatura = svc
            .create_from_invoice(1, EFaturaProfile::TemelFatura, 1)
            .await
            .unwrap();

        assert_eq!(fatura.tenant_id, 1);
        assert_eq!(fatura.invoice_id, Some(1));
        assert_eq!(fatura.profile_id, EFaturaProfile::TemelFatura);
        assert_eq!(fatura.status, EFaturaStatus::Draft);
        assert!(!fatura.uuid.is_empty());
        assert!(fatura.document_number.starts_with("EF"));
    }

    #[tokio::test]
    async fn test_get_efatura() {
        let svc = make_service().await;
        let fatura = svc
            .create_from_invoice(1, EFaturaProfile::TemelFatura, 1)
            .await
            .unwrap();

        let found = svc.get_efatura(fatura.id, 1).await.unwrap();
        assert_eq!(found.id, fatura.id);

        let result = svc.get_efatura(9999, 1).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_by_uuid() {
        let svc = make_service().await;
        let fatura = svc
            .create_from_invoice(1, EFaturaProfile::TemelFatura, 1)
            .await
            .unwrap();

        let found = svc.get_by_uuid(&fatura.uuid, 1).await.unwrap();
        assert_eq!(found.id, fatura.id);

        let result = svc.get_by_uuid("nonexistent", 1).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_efaturas() {
        let svc = make_service().await;

        svc.create_from_invoice(1, EFaturaProfile::TemelFatura, 1)
            .await
            .unwrap();
        svc.create_from_invoice(2, EFaturaProfile::Ihracat, 1)
            .await
            .unwrap();

        let all = svc
            .list_efaturas(1, None, PaginationParams::default())
            .await
            .unwrap();
        assert_eq!(all.items.len(), 2);

        let drafts = svc
            .list_efaturas(1, Some(EFaturaStatus::Draft), PaginationParams::default())
            .await
            .unwrap();
        assert_eq!(drafts.items.len(), 2);

        let sent = svc
            .list_efaturas(1, Some(EFaturaStatus::Sent), PaginationParams::default())
            .await
            .unwrap();
        assert_eq!(sent.items.len(), 0);
    }

    #[tokio::test]
    async fn test_send_to_gib() {
        let svc = make_service().await;
        let fatura = svc
            .create_from_invoice(1, EFaturaProfile::TemelFatura, 1)
            .await
            .unwrap();

        let sent = svc.send_to_gib(fatura.id, 1).await.unwrap();
        assert_eq!(sent.status, EFaturaStatus::Sent);
        assert!(sent.response_code.is_some());
    }

    #[tokio::test]
    async fn test_send_non_draft_fails() {
        let svc = make_service().await;
        let fatura = svc
            .create_from_invoice(1, EFaturaProfile::TemelFatura, 1)
            .await
            .unwrap();

        // Send once
        svc.send_to_gib(fatura.id, 1).await.unwrap();

        // Sending again should fail
        let result = svc.send_to_gib(fatura.id, 1).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cancel_efatura() {
        let svc = make_service().await;
        let fatura = svc
            .create_from_invoice(1, EFaturaProfile::TemelFatura, 1)
            .await
            .unwrap();

        // Send first
        svc.send_to_gib(fatura.id, 1).await.unwrap();

        // Cancel
        let cancelled = svc
            .cancel_efatura(fatura.id, 1, "Mistake".to_string())
            .await
            .unwrap();
        assert_eq!(cancelled.status, EFaturaStatus::Cancelled);
    }

    #[tokio::test]
    async fn test_cancel_already_cancelled_fails() {
        let svc = make_service().await;
        let fatura = svc
            .create_from_invoice(1, EFaturaProfile::TemelFatura, 1)
            .await
            .unwrap();

        // Cancel a draft (no GIB call)
        svc.cancel_efatura(fatura.id, 1, "Mistake".to_string())
            .await
            .unwrap();

        // Cancel again should fail
        let result = svc.cancel_efatura(fatura.id, 1, "Again".to_string()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_xml_placeholder() {
        let svc = make_service().await;
        let fatura = svc
            .create_from_invoice(1, EFaturaProfile::TemelFatura, 1)
            .await
            .unwrap();

        let xml = svc.get_xml(fatura.id, 1).await.unwrap();
        assert!(xml.contains("Invoice"));
        assert!(xml.contains(&fatura.uuid));
    }

    #[tokio::test]
    async fn test_check_status() {
        let svc = make_service().await;
        let fatura = svc
            .create_from_invoice(1, EFaturaProfile::TemelFatura, 1)
            .await
            .unwrap();

        // Send first so GIB gateway knows the UUID
        svc.send_to_gib(fatura.id, 1).await.unwrap();

        let checked = svc.check_status(&fatura.uuid, 1).await.unwrap();
        assert_eq!(checked.status, EFaturaStatus::Accepted);
    }

    #[tokio::test]
    async fn test_tenant_isolation() {
        let svc = make_service().await;
        let fatura = svc
            .create_from_invoice(1, EFaturaProfile::TemelFatura, 1)
            .await
            .unwrap();

        let result = svc.get_efatura(fatura.id, 999).await;
        assert!(result.is_err());
    }

    /// Rejects an e-Fatura stamped onto a foreign-tenant invoice (orphan-FK
    /// IDOR, issue #300). Own-tenant invoice id 1 succeeds; the tenant-2
    /// invoice id 3 and a nonexistent id 999 both 404.
    #[tokio::test]
    async fn test_create_from_invoice_rejects_foreign_invoice() {
        let svc = make_service().await;

        // Own-tenant invoice (id 1, tenant 1) → ok.
        assert!(
            svc.create_from_invoice(1, EFaturaProfile::TemelFatura, 1)
                .await
                .is_ok(),
            "own-tenant invoice must succeed"
        );

        // Foreign-tenant invoice (id 3, belongs to tenant 2) → NotFound.
        let result = svc
            .create_from_invoice(3, EFaturaProfile::TemelFatura, 1)
            .await;
        assert!(
            matches!(result, Err(ApiError::NotFound(_))),
            "tenant-1 must NOT create an e-Fatura for a tenant-2 invoice, got {:?}",
            result
        );

        // Nonexistent invoice → NotFound.
        let result = svc
            .create_from_invoice(999, EFaturaProfile::TemelFatura, 1)
            .await;
        assert!(
            matches!(result, Err(ApiError::NotFound(_))),
            "nonexistent invoice must be NotFound, got {:?}",
            result
        );
    }
}
