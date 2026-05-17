//! Vendor Portal service

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

/// Vendor portal service
#[derive(Clone)]
pub struct VendorPortalService {
    pub vendor_user_repo: BoxVendorUserRepository,
    pub delivery_note_repo: BoxDeliveryNoteRepository,
    pub cari_service: Arc<CariService>,
    pub purchase_service: Arc<PurchaseService>,
    pub invoice_service: Arc<InvoiceService>,
    pub jwt_service: Arc<JwtService>,
    pub jwt_expiry_hours: i64,
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

    // ── Auth ────────────────────────────────────────────────────────────────

    pub async fn register(
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

    pub async fn login(
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
            .await;

        let claims = VendorAuthClaims::new(
            user.id,
            tenant_id,
            user.cari_id,
            user.email.clone(),
            self.jwt_expiry_hours * 3600,
        );
        let access_token = self.encode_vendor_token(&claims)?;

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

    pub async fn get_profile(
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

    // ── Data access (delegates to existing services) ───────────────────────

    pub async fn get_orders(
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
                item_count: 0, // TODO: populate from order lines once PurchaseService exposes them
            })
            .collect();

        Ok(VendorPaginatedResponse::new(views, total, page, per_page))
    }

    pub async fn get_invoices(
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

    pub async fn get_payments(
        &self,
        cari_id: i64,
        _tenant_id: i64,
        pagination: VendorPaginationParams,
    ) -> Result<VendorPaginatedResponse<VendorPaymentView>, ApiError> {
        let invoices = self
            .invoice_service
            .get_invoices_by_cari(_tenant_id, cari_id)
            .await?;
        let mut views = Vec::new();

        for inv in invoices {
            let payments = self.invoice_service.get_payments_by_invoice(inv.id).await?;
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

    pub async fn get_invoice_pdf(
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

    // ── Delivery notes ──────────────────────────────────────────────────────

    pub async fn create_delivery_note(
        &self,
        vendor_user_id: i64,
        cari_id: i64,
        tenant_id: i64,
        req: CreateDeliveryNote,
    ) -> Result<DeliveryNote, ApiError> {
        self.delivery_note_repo
            .create(req, vendor_user_id, cari_id, tenant_id)
            .await
    }

    pub async fn get_delivery_notes(
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

    // ── JWT helpers ─────────────────────────────────────────────────────────

    fn encode_vendor_token(&self, claims: &VendorAuthClaims) -> Result<String, ApiError> {
        self.jwt_service.encode_vendor_token(claims)
    }

    pub fn decode_vendor_token(&self, token: &str) -> Result<VendorAuthClaims, ApiError> {
        self.jwt_service.decode_vendor_token(token)
    }
}
