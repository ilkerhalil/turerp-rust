//! Vendor Portal service

use async_trait::async_trait;
use std::sync::Arc;

use crate::domain::cari::service::CariService;
use crate::domain::invoice::service::InvoiceService;
use crate::domain::purchase::service::PurchaseService;
use crate::domain::vendor_portal::model::{
    CreateDeliveryNote, CreateVendorUser, DeliveryNote, VendorAuthResponse, VendorInvoiceView,
    VendorLoginRequest, VendorOrderView, VendorPaginatedResponse, VendorPaginationParams,
    VendorPaymentView, VendorUser, VendorUserProfile, VendorUserStatus,
};
use crate::domain::vendor_portal::repository::{
    BoxDeliveryNoteRepository, BoxVendorUserRepository,
};
use crate::error::ApiError;
use crate::utils::jwt::{JwtService, VendorAuthClaims};
use crate::utils::password;

/// Trait for vendor portal operations
#[async_trait]
pub trait VendorPortal: Send + Sync {
    async fn register(&self, tenant_id: i64, req: CreateVendorUser)
        -> Result<VendorUser, ApiError>;
    async fn login(
        &self,
        tenant_id: i64,
        req: VendorLoginRequest,
    ) -> Result<VendorAuthResponse, ApiError>;
    async fn get_profile(
        &self,
        vendor_user_id: i64,
        tenant_id: i64,
    ) -> Result<VendorUserProfile, ApiError>;
    async fn get_orders(
        &self,
        cari_id: i64,
        tenant_id: i64,
        pagination: VendorPaginationParams,
    ) -> Result<VendorPaginatedResponse<VendorOrderView>, ApiError>;
    async fn get_invoices(
        &self,
        cari_id: i64,
        tenant_id: i64,
        pagination: VendorPaginationParams,
    ) -> Result<VendorPaginatedResponse<VendorInvoiceView>, ApiError>;
    async fn get_payments(
        &self,
        cari_id: i64,
        tenant_id: i64,
        pagination: VendorPaginationParams,
    ) -> Result<VendorPaginatedResponse<VendorPaymentView>, ApiError>;
    async fn get_invoice_pdf(
        &self,
        invoice_id: i64,
        cari_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<u8>, ApiError>;
    async fn create_delivery_note(
        &self,
        vendor_user_id: i64,
        cari_id: i64,
        tenant_id: i64,
        req: CreateDeliveryNote,
    ) -> Result<DeliveryNote, ApiError>;
    async fn get_delivery_notes(
        &self,
        vendor_user_id: i64,
        tenant_id: i64,
        pagination: VendorPaginationParams,
    ) -> Result<VendorPaginatedResponse<DeliveryNote>, ApiError>;
}

pub type BoxVendorPortal = Arc<dyn VendorPortal>;

/// Vendor portal service
#[derive(Clone)]
pub struct VendorPortalService {
    vendor_user_repo: BoxVendorUserRepository,
    delivery_note_repo: BoxDeliveryNoteRepository,
    cari_service: Arc<CariService>,
    purchase_service: Arc<PurchaseService>,
    invoice_service: Arc<InvoiceService>,
    jwt_service: Arc<JwtService>,
    jwt_expiry_hours: i64,
}

impl VendorPortalService {
    pub fn new(
        vendor_user_repo: BoxVendorUserRepository,
        delivery_note_repo: BoxDeliveryNoteRepository,
        cari_service: Arc<CariService>,
        purchase_service: Arc<PurchaseService>,
        invoice_service: Arc<InvoiceService>,
        jwt_service: Arc<JwtService>,
        jwt_expiry_hours: i64,
    ) -> Self {
        Self {
            vendor_user_repo,
            delivery_note_repo,
            cari_service,
            purchase_service,
            invoice_service,
            jwt_service,
            jwt_expiry_hours,
        }
    }

    pub fn decode_vendor_token(&self, token: &str) -> Result<VendorAuthClaims, ApiError> {
        self.jwt_service.decode_vendor_token(token)
    }
}

#[async_trait]
impl VendorPortal for VendorPortalService {
    async fn register(
        &self,
        tenant_id: i64,
        req: CreateVendorUser,
    ) -> Result<VendorUser, ApiError> {
        password::validate_password(&req.password).map_err(|e| ApiError::Validation(e.message))?;

        let cari = self
            .cari_service
            .get_cari(req.cari_id, tenant_id)
            .await
            .map_err(|_| ApiError::BadRequest("Invalid cari account".to_string()))?;

        let cari_type = cari.cari_type;
        if cari_type != crate::domain::cari::model::CariType::Vendor
            && cari_type != crate::domain::cari::model::CariType::Both
        {
            return Err(ApiError::BadRequest(
                "Cari must be a vendor or both type".to_string(),
            ));
        }

        if cari.status != crate::domain::cari::model::CariStatus::Active {
            return Err(ApiError::BadRequest(
                "Cari account must be active".to_string(),
            ));
        }

        if self
            .vendor_user_repo
            .find_by_email(&req.email, tenant_id)
            .await?
            .is_some()
        {
            return Err(ApiError::Conflict(
                "Vendor user with this email already exists".to_string(),
            ));
        }

        let password_hash = password::hash_password(&req.password)
            .map_err(|e| ApiError::Internal(format!("Password hashing failed: {}", e)))?;

        let user = self
            .vendor_user_repo
            .create(req, password_hash, tenant_id)
            .await?;

        Ok(user)
    }

    async fn login(
        &self,
        tenant_id: i64,
        req: VendorLoginRequest,
    ) -> Result<VendorAuthResponse, ApiError> {
        let user = self
            .vendor_user_repo
            .find_by_email(&req.email, tenant_id)
            .await?
            .ok_or(ApiError::InvalidCredentials)?;

        if user.status == VendorUserStatus::Blocked {
            return Err(ApiError::Unauthorized("Account is blocked".to_string()));
        }

        let valid = password::verify_password(&req.password, &user.password_hash)
            .map_err(|e| ApiError::Internal(format!("Password verification failed: {}", e)))?;
        if !valid {
            return Err(ApiError::InvalidCredentials);
        }

        let _ = self
            .vendor_user_repo
            .update_last_login(user.id, tenant_id)
            .await
            .ok();

        let claims = VendorAuthClaims::new(
            user.id,
            tenant_id,
            user.cari_id,
            user.email.clone(),
            self.jwt_expiry_hours * 3600,
        );
        let access_token = self
            .jwt_service
            .encode_vendor_token(&claims)
            .map_err(|e| ApiError::Internal(format!("Token encoding failed: {}", e)))?;

        let cari_name = match self.cari_service.get_cari(user.cari_id, tenant_id).await {
            Ok(cari) => cari.name,
            Err(_) => String::new(),
        };

        let profile = VendorUserProfile {
            id: user.id,
            email: user.email,
            full_name: user.full_name,
            cari_id: user.cari_id,
            cari_name,
            language: user.language,
            timezone: user.timezone,
        };

        Ok(VendorAuthResponse {
            access_token,
            token_type: "Bearer".to_string(),
            expires_in: self.jwt_expiry_hours * 3600,
            vendor_user: profile,
        })
    }

    async fn get_profile(
        &self,
        vendor_user_id: i64,
        tenant_id: i64,
    ) -> Result<VendorUserProfile, ApiError> {
        let user = self
            .vendor_user_repo
            .find_by_id(vendor_user_id, tenant_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!("Vendor user {} not found", vendor_user_id))
            })?;

        let cari_name = match self.cari_service.get_cari(user.cari_id, tenant_id).await {
            Ok(cari) => cari.name,
            Err(_) => String::new(),
        };

        Ok(VendorUserProfile {
            id: user.id,
            email: user.email,
            full_name: user.full_name,
            cari_id: user.cari_id,
            cari_name,
            language: user.language,
            timezone: user.timezone,
        })
    }

    async fn get_orders(
        &self,
        cari_id: i64,
        _tenant_id: i64,
        pagination: VendorPaginationParams,
    ) -> Result<VendorPaginatedResponse<VendorOrderView>, ApiError> {
        let orders = self.purchase_service.get_orders_by_cari(cari_id).await?;
        let total = orders.len() as i64;
        let page = pagination.page();
        let per_page = pagination.per_page();

        let views: Vec<VendorOrderView> = orders
            .into_iter()
            .skip(((page - 1) * per_page) as usize)
            .take(per_page as usize)
            .map(|order| VendorOrderView {
                id: order.id,
                order_number: order.order_number,
                status: order.status.to_string(),
                order_date: order.order_date.date_naive(),
                expected_delivery_date: order.expected_delivery_date.map(|d| d.date_naive()),
                total_amount: order.total_amount,
                currency: order.currency,
                item_count: 0, // FIXME: populate from PurchaseOrderLineRepository::find_by_order(order.id) once PurchaseService exposes a line-count method
            })
            .collect();

        Ok(VendorPaginatedResponse::new(views, total, page, per_page))
    }

    async fn get_invoices(
        &self,
        cari_id: i64,
        _tenant_id: i64,
        pagination: VendorPaginationParams,
    ) -> Result<VendorPaginatedResponse<VendorInvoiceView>, ApiError> {
        let invoices = self
            .invoice_service
            .get_invoices_by_cari(_tenant_id, cari_id)
            .await?;
        let total = invoices.len() as i64;
        let page = pagination.page();
        let per_page = pagination.per_page();

        let views: Vec<VendorInvoiceView> = invoices
            .into_iter()
            .skip(((page - 1) * per_page) as usize)
            .take(per_page as usize)
            .map(|inv| VendorInvoiceView {
                id: inv.id,
                invoice_number: inv.invoice_number,
                invoice_type: inv.invoice_type.to_string(),
                status: inv.status.to_string(),
                issue_date: inv.issue_date.date_naive(),
                due_date: inv.due_date.date_naive(),
                total_amount: inv.total_amount,
                paid_amount: inv.paid_amount,
                outstanding_amount: inv.total_amount - inv.paid_amount,
                currency: inv.currency,
            })
            .collect();

        Ok(VendorPaginatedResponse::new(views, total, page, per_page))
    }

    async fn get_payments(
        &self,
        cari_id: i64,
        tenant_id: i64,
        pagination: VendorPaginationParams,
    ) -> Result<VendorPaginatedResponse<VendorPaymentView>, ApiError> {
        let invoices = self
            .invoice_service
            .get_invoices_by_cari(tenant_id, cari_id)
            .await?;
        let mut views = Vec::new();

        for inv in invoices {
            let payments = self
                .invoice_service
                .get_payments_by_invoice(inv.id, tenant_id)
                .await?;
            for payment in payments {
                views.push(VendorPaymentView {
                    id: payment.id,
                    invoice_id: inv.id,
                    invoice_number: inv.invoice_number.clone(),
                    amount: payment.amount,
                    payment_date: payment.payment_date.date_naive(),
                    payment_method: payment.payment_method.clone(),
                    currency: payment.currency.clone(),
                });
            }
        }

        views.sort_by_key(|a| std::cmp::Reverse(a.payment_date));

        let total = views.len() as i64;
        let page = pagination.page();
        let per_page = pagination.per_page();

        let paginated = views
            .into_iter()
            .skip(((page - 1) * per_page) as usize)
            .take(per_page as usize)
            .collect();

        Ok(VendorPaginatedResponse::new(
            paginated, total, page, per_page,
        ))
    }

    async fn get_invoice_pdf(
        &self,
        invoice_id: i64,
        cari_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<u8>, ApiError> {
        let invoices = self
            .invoice_service
            .get_invoices_by_cari(tenant_id, cari_id)
            .await?;
        if !invoices
            .iter()
            .any(|i| i.id == invoice_id && i.tenant_id == tenant_id)
        {
            return Err(ApiError::NotFound(
                "Invoice not found for this vendor".to_string(),
            ));
        }

        Err(ApiError::NotFound(
            "Invoice PDF not yet available".to_string(),
        ))
    }

    async fn create_delivery_note(
        &self,
        vendor_user_id: i64,
        cari_id: i64,
        tenant_id: i64,
        req: CreateDeliveryNote,
    ) -> Result<DeliveryNote, ApiError> {
        // Parent-ownership precheck: the body-controlled `purchase_order_id`
        // must reference a purchase order in the caller's tenant AND issued to
        // THIS vendor's cari. The tenant-scoped `get_purchase_order` returns
        // NotFound for a foreign-tenant PO (cross-tenant gate); the `cari_id`
        // match returns NotFound for a PO issued to a different vendor
        // (within-tenant vendor isolation). Both gates run BEFORE the delivery
        // note INSERT, and NotFound reveals only "PO not found" (no existence
        // leak beyond the intended signal).
        let po = self
            .purchase_service
            .get_purchase_order(req.purchase_order_id, tenant_id)
            .await?;
        if po.cari_id != cari_id {
            return Err(ApiError::NotFound(format!(
                "Purchase order {} not found",
                req.purchase_order_id
            )));
        }
        self.delivery_note_repo
            .create(req, vendor_user_id, cari_id, tenant_id)
            .await
    }

    async fn get_delivery_notes(
        &self,
        vendor_user_id: i64,
        tenant_id: i64,
        pagination: VendorPaginationParams,
    ) -> Result<VendorPaginatedResponse<DeliveryNote>, ApiError> {
        let notes = self
            .delivery_note_repo
            .find_by_vendor_user(vendor_user_id, tenant_id)
            .await?;
        let total = notes.len() as i64;
        let page = pagination.page();
        let per_page = pagination.per_page();

        let paginated = notes
            .into_iter()
            .skip(((page - 1) * per_page) as usize)
            .take(per_page as usize)
            .collect();

        Ok(VendorPaginatedResponse::new(
            paginated, total, page, per_page,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::cari::repository::{BoxCariRepository, InMemoryCariRepository};
    use crate::domain::company::repository::{BoxCompanyRepository, InMemoryCompanyRepository};
    use crate::domain::cost_center::repository::{
        BoxCostCenterRepository, InMemoryCostCenterRepository,
    };
    use crate::domain::invoice::repository::{
        BoxInvoiceLineRepository, BoxInvoiceRepository, BoxPaymentRepository,
        InMemoryInvoiceLineRepository, InMemoryInvoiceRepository, InMemoryPaymentRepository,
    };
    use crate::domain::product::repository::{BoxProductRepository, InMemoryProductRepository};
    use crate::domain::purchase::model::CreatePurchaseOrder;
    use crate::domain::purchase::repository::{
        BoxGoodsReceiptLineRepository, BoxGoodsReceiptRepository, BoxPurchaseOrderLineRepository,
        BoxPurchaseOrderRepository, InMemoryGoodsReceiptLineRepository,
        InMemoryGoodsReceiptRepository, InMemoryPurchaseOrderLineRepository,
        InMemoryPurchaseOrderRepository,
    };
    use crate::domain::vendor_portal::repository::{
        InMemoryDeliveryNoteRepository, InMemoryVendorUserRepository,
    };

    // Build a VendorPortalService backed by InMemory repos. The
    // purchase_service holds a shared InMemoryPurchaseOrderRepository that we
    // seed directly (bypassing the service-level cari/product/company
    // prechecks) with three POs:
    //   id=1: tenant 1, cari 10  (this vendor's own PO)
    //   id=2: tenant 1, cari 20  (same tenant, DIFFERENT vendor)
    //   id=3: tenant 2, cari 10  (foreign tenant)
    // cari_id=10 / tenant_id=1 identify the calling vendor in the reject cases.
    async fn make_service() -> (VendorPortalService, BoxPurchaseOrderRepository) {
        let order_repo =
            Arc::new(InMemoryPurchaseOrderRepository::new()) as BoxPurchaseOrderRepository;
        let order_line_repo =
            Arc::new(InMemoryPurchaseOrderLineRepository::new()) as BoxPurchaseOrderLineRepository;
        let receipt_repo =
            Arc::new(InMemoryGoodsReceiptRepository::new()) as BoxGoodsReceiptRepository;
        let receipt_line_repo =
            Arc::new(InMemoryGoodsReceiptLineRepository::new()) as BoxGoodsReceiptLineRepository;
        let cari_repo = Arc::new(InMemoryCariRepository::new()) as BoxCariRepository;
        let product_repo = Arc::new(InMemoryProductRepository::new()) as BoxProductRepository;
        let company_repo = Arc::new(InMemoryCompanyRepository::new()) as BoxCompanyRepository;

        let po = |tenant: i64, cari: i64| CreatePurchaseOrder {
            tenant_id: tenant,
            company_id: 1,
            cari_id: cari,
            order_date: chrono::Utc::now(),
            expected_delivery_date: None,
            notes: None,
            currency: "TRY".to_string(),
            exchange_rate: rust_decimal::Decimal::ONE,
            lines: vec![],
        };
        // Seed: own PO (1), same-tenant other-vendor PO (2), foreign-tenant PO (3).
        order_repo.create(po(1, 10)).await.expect("seed po 1");
        order_repo.create(po(1, 20)).await.expect("seed po 2");
        order_repo.create(po(2, 10)).await.expect("seed po 3");

        let purchase_service = Arc::new(PurchaseService::new(
            order_repo.clone(),
            order_line_repo,
            receipt_repo,
            receipt_line_repo,
            cari_repo.clone(),
            product_repo.clone(),
            company_repo.clone(),
        ));
        let cari_service = Arc::new(CariService::new(cari_repo.clone(), company_repo.clone()));
        let invoice_service = Arc::new(InvoiceService::new(
            Arc::new(InMemoryInvoiceRepository::new()) as BoxInvoiceRepository,
            Arc::new(InMemoryInvoiceLineRepository::new()) as BoxInvoiceLineRepository,
            Arc::new(InMemoryPaymentRepository::new()) as BoxPaymentRepository,
            cari_repo.clone(),
            Arc::new(InMemoryCostCenterRepository::new()) as BoxCostCenterRepository,
            product_repo,
            company_repo,
        ));
        let jwt_service = Arc::new(JwtService::new(
            "test-secret-at-least-32-chars-long".to_string(),
            1,
            1,
        ));

        let vendor_user_repo =
            Arc::new(InMemoryVendorUserRepository::new()) as BoxVendorUserRepository;
        let delivery_note_repo =
            Arc::new(InMemoryDeliveryNoteRepository::new()) as BoxDeliveryNoteRepository;

        let service = VendorPortalService::new(
            vendor_user_repo,
            delivery_note_repo,
            cari_service,
            purchase_service,
            invoice_service,
            jwt_service,
            1,
        );
        (service, order_repo)
    }

    fn note(po_id: i64) -> CreateDeliveryNote {
        CreateDeliveryNote {
            purchase_order_id: po_id,
            description: "shipped".to_string(),
        }
    }

    #[tokio::test]
    async fn test_create_delivery_note_own_po_ok() {
        let (service, _) = make_service().await;
        // Vendor (cari 10, tenant 1) references their own PO (id=1) -> ok.
        let result = service
            .create_delivery_note(1, 10, 1, note(1))
            .await
            .expect("own PO must succeed");
        assert_eq!(result.purchase_order_id, 1);
    }

    #[tokio::test]
    async fn test_create_delivery_note_rejects_foreign_tenant_po() {
        let (service, _) = make_service().await;
        // PO id=3 belongs to tenant 2 -> tenant-scoped get_purchase_order
        // returns NotFound (cross-tenant gate).
        let result = service.create_delivery_note(1, 10, 1, note(3)).await;
        assert!(
            matches!(result, Err(ApiError::NotFound(_))),
            "tenant-1 vendor must NOT create a delivery note against a tenant-2 PO, got {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_create_delivery_note_rejects_other_vendor_po() {
        let (service, _) = make_service().await;
        // PO id=2 is in tenant 1 but issued to cari 20 (a different vendor);
        // the calling vendor is cari 10 -> cari_id mismatch -> NotFound
        // (within-tenant vendor isolation).
        let result = service.create_delivery_note(1, 10, 1, note(2)).await;
        assert!(
            matches!(result, Err(ApiError::NotFound(_))),
            "vendor cari 10 must NOT create a delivery note against a PO issued to cari 20, got {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_create_delivery_note_rejects_nonexistent_po() {
        let (service, _) = make_service().await;
        let result = service.create_delivery_note(1, 10, 1, note(999_999)).await;
        assert!(
            matches!(result, Err(ApiError::NotFound(_))),
            "nonexistent PO must be NotFound, got {:?}",
            result
        );
    }
}
