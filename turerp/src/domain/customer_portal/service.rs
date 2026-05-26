//! Customer Portal service

use async_trait::async_trait;
use std::sync::Arc;

use crate::domain::cari::service::CariService;
use crate::domain::customer_portal::model::{
    CreatePortalUser, CreateSupportTicket, CustomerInvoiceView, CustomerOrderView,
    CustomerPaymentView, PortalAuthResponse, PortalLoginRequest, PortalPaginatedResponse,
    PortalPaginationParams, PortalUser, PortalUserProfile, PortalUserStatus, SupportTicket,
};
use crate::domain::customer_portal::repository::{
    BoxPortalUserRepository, BoxSupportTicketRepository,
};
use crate::domain::invoice::service::InvoiceService;
use crate::domain::sales::service::SalesService;
use crate::error::ApiError;
use crate::utils::jwt::{JwtService, PortalAuthClaims};
use crate::utils::password;

/// Trait for customer portal operations
#[async_trait]
pub trait CustomerPortal: Send + Sync {
    async fn register(&self, tenant_id: i64, req: CreatePortalUser)
        -> Result<PortalUser, ApiError>;
    async fn login(
        &self,
        tenant_id: i64,
        req: PortalLoginRequest,
    ) -> Result<PortalAuthResponse, ApiError>;
    async fn get_profile(
        &self,
        portal_user_id: i64,
        tenant_id: i64,
    ) -> Result<PortalUserProfile, ApiError>;
    async fn get_orders(
        &self,
        cari_id: i64,
        tenant_id: i64,
        pagination: PortalPaginationParams,
    ) -> Result<PortalPaginatedResponse<CustomerOrderView>, ApiError>;
    async fn get_invoices(
        &self,
        cari_id: i64,
        tenant_id: i64,
        pagination: PortalPaginationParams,
    ) -> Result<PortalPaginatedResponse<CustomerInvoiceView>, ApiError>;
    async fn get_payments(
        &self,
        cari_id: i64,
        tenant_id: i64,
        pagination: PortalPaginationParams,
    ) -> Result<PortalPaginatedResponse<CustomerPaymentView>, ApiError>;
    async fn get_invoice_pdf(
        &self,
        invoice_id: i64,
        cari_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<u8>, ApiError>;
    async fn create_support_ticket(
        &self,
        portal_user_id: i64,
        cari_id: i64,
        tenant_id: i64,
        req: CreateSupportTicket,
    ) -> Result<SupportTicket, ApiError>;
    async fn get_support_tickets(
        &self,
        portal_user_id: i64,
        tenant_id: i64,
        pagination: PortalPaginationParams,
    ) -> Result<PortalPaginatedResponse<SupportTicket>, ApiError>;
}

pub type BoxCustomerPortal = Arc<dyn CustomerPortal>;

/// Customer portal service
#[derive(Clone)]
pub struct CustomerPortalService {
    portal_user_repo: BoxPortalUserRepository,
    ticket_repo: BoxSupportTicketRepository,
    cari_service: Arc<CariService>,
    sales_service: Arc<SalesService>,
    invoice_service: Arc<InvoiceService>,
    jwt_service: Arc<JwtService>,
    jwt_expiry_hours: i64,
}

impl CustomerPortalService {
    pub fn new(
        portal_user_repo: BoxPortalUserRepository,
        ticket_repo: BoxSupportTicketRepository,
        cari_service: Arc<CariService>,
        sales_service: Arc<SalesService>,
        invoice_service: Arc<InvoiceService>,
        jwt_service: Arc<JwtService>,
        jwt_expiry_hours: i64,
    ) -> Self {
        Self {
            portal_user_repo,
            ticket_repo,
            cari_service,
            sales_service,
            invoice_service,
            jwt_service,
            jwt_expiry_hours,
        }
    }

    pub fn decode_portal_token(&self, token: &str) -> Result<PortalAuthClaims, ApiError> {
        self.jwt_service.decode_portal_token(token)
    }
}

#[async_trait]
impl CustomerPortal for CustomerPortalService {
    async fn register(
        &self,
        tenant_id: i64,
        req: CreatePortalUser,
    ) -> Result<PortalUser, ApiError> {
        password::validate_password(&req.password).map_err(|e| ApiError::Validation(e.message))?;

        let cari = self
            .cari_service
            .get_cari(req.cari_id, tenant_id)
            .await
            .map_err(|_| ApiError::BadRequest("Invalid cari account".to_string()))?;

        let cari_type = cari.cari_type;
        if cari_type != crate::domain::cari::model::CariType::Customer
            && cari_type != crate::domain::cari::model::CariType::Both
        {
            return Err(ApiError::BadRequest(
                "Cari must be a customer or both type".to_string(),
            ));
        }

        if cari.status != crate::domain::cari::model::CariStatus::Active {
            return Err(ApiError::BadRequest(
                "Cari account must be active".to_string(),
            ));
        }

        if self
            .portal_user_repo
            .find_by_email(&req.email, tenant_id)
            .await?
            .is_some()
        {
            return Err(ApiError::Conflict(
                "Portal user with this email already exists".to_string(),
            ));
        }

        let password_hash = password::hash_password(&req.password)
            .map_err(|e| ApiError::Internal(format!("Password hashing failed: {}", e)))?;

        let user = self
            .portal_user_repo
            .create(req, password_hash, tenant_id)
            .await?;

        Ok(user)
    }

    async fn login(
        &self,
        tenant_id: i64,
        req: PortalLoginRequest,
    ) -> Result<PortalAuthResponse, ApiError> {
        let user = self
            .portal_user_repo
            .find_by_email(&req.email, tenant_id)
            .await?
            .ok_or(ApiError::InvalidCredentials)?;

        if user.status == PortalUserStatus::Blocked {
            return Err(ApiError::Unauthorized("Account is blocked".to_string()));
        }

        let valid = password::verify_password(&req.password, &user.password_hash)
            .map_err(|e| ApiError::Internal(format!("Password verification failed: {}", e)))?;
        if !valid {
            return Err(ApiError::InvalidCredentials);
        }

        let _ = self
            .portal_user_repo
            .update_last_login(user.id, tenant_id)
            .await
            .ok();

        let claims = PortalAuthClaims::new(
            user.id,
            tenant_id,
            user.cari_id,
            user.email.clone(),
            self.jwt_expiry_hours * 3600,
        );
        let access_token = self
            .jwt_service
            .encode_portal_token(&claims)
            .map_err(|e| ApiError::Internal(format!("Token encoding failed: {}", e)))?;

        let cari_name = match self.cari_service.get_cari(user.cari_id, tenant_id).await {
            Ok(cari) => cari.name,
            Err(_) => String::new(),
        };

        let profile = PortalUserProfile {
            id: user.id,
            email: user.email,
            full_name: user.full_name,
            cari_id: user.cari_id,
            cari_name,
            language: user.language,
            timezone: user.timezone,
        };

        Ok(PortalAuthResponse {
            access_token,
            token_type: "Bearer".to_string(),
            expires_in: self.jwt_expiry_hours * 3600,
            portal_user: profile,
        })
    }

    async fn get_profile(
        &self,
        portal_user_id: i64,
        tenant_id: i64,
    ) -> Result<PortalUserProfile, ApiError> {
        let user = self
            .portal_user_repo
            .find_by_id(portal_user_id, tenant_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!("Portal user {} not found", portal_user_id))
            })?;

        let cari_name = match self.cari_service.get_cari(user.cari_id, tenant_id).await {
            Ok(cari) => cari.name,
            Err(_) => String::new(),
        };

        Ok(PortalUserProfile {
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
        pagination: PortalPaginationParams,
    ) -> Result<PortalPaginatedResponse<CustomerOrderView>, ApiError> {
        let orders = self.sales_service.get_orders_by_cari(cari_id).await?;
        let total = orders.len() as i64;
        let page = pagination.page();
        let per_page = pagination.per_page();

        let views: Vec<CustomerOrderView> = orders
            .into_iter()
            .skip(((page - 1) * per_page) as usize)
            .take(per_page as usize)
            .map(|order| CustomerOrderView {
                id: order.id,
                order_number: order.order_number,
                status: order.status.to_string(),
                order_date: order.order_date.date_naive(),
                delivery_date: order.delivery_date.map(|d| d.date_naive()),
                total_amount: order.total_amount,
                currency: order.currency,
                item_count: 0, // FIXME: populate from SalesOrderLineRepository::find_by_order(order.id) once SalesService exposes a line-count method
            })
            .collect();

        Ok(PortalPaginatedResponse::new(views, total, page, per_page))
    }

    async fn get_invoices(
        &self,
        cari_id: i64,
        _tenant_id: i64,
        pagination: PortalPaginationParams,
    ) -> Result<PortalPaginatedResponse<CustomerInvoiceView>, ApiError> {
        let invoices = self
            .invoice_service
            .get_invoices_by_cari(_tenant_id, cari_id)
            .await?;
        let total = invoices.len() as i64;
        let page = pagination.page();
        let per_page = pagination.per_page();

        let views: Vec<CustomerInvoiceView> = invoices
            .into_iter()
            .skip(((page - 1) * per_page) as usize)
            .take(per_page as usize)
            .map(|inv| CustomerInvoiceView {
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

        Ok(PortalPaginatedResponse::new(views, total, page, per_page))
    }

    async fn get_payments(
        &self,
        cari_id: i64,
        _tenant_id: i64,
        pagination: PortalPaginationParams,
    ) -> Result<PortalPaginatedResponse<CustomerPaymentView>, ApiError> {
        let invoices = self
            .invoice_service
            .get_invoices_by_cari(_tenant_id, cari_id)
            .await?;
        let mut views = Vec::new();

        for inv in invoices {
            let payments = self.invoice_service.get_payments_by_invoice(inv.id).await?;
            for payment in payments {
                views.push(CustomerPaymentView {
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

        Ok(PortalPaginatedResponse::new(
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
                "Invoice not found for this customer".to_string(),
            ));
        }

        Err(ApiError::Internal(
            "PDF generation not yet implemented".to_string(),
        ))
    }

    async fn create_support_ticket(
        &self,
        portal_user_id: i64,
        cari_id: i64,
        tenant_id: i64,
        req: CreateSupportTicket,
    ) -> Result<SupportTicket, ApiError> {
        self.ticket_repo
            .create(req, portal_user_id, cari_id, tenant_id)
            .await
    }

    async fn get_support_tickets(
        &self,
        portal_user_id: i64,
        tenant_id: i64,
        pagination: PortalPaginationParams,
    ) -> Result<PortalPaginatedResponse<SupportTicket>, ApiError> {
        let tickets = self
            .ticket_repo
            .find_by_portal_user(portal_user_id, tenant_id)
            .await?;
        let total = tickets.len() as i64;
        let page = pagination.page();
        let per_page = pagination.per_page();

        let paginated = tickets
            .into_iter()
            .skip(((page - 1) * per_page) as usize)
            .take(per_page as usize)
            .collect();

        Ok(PortalPaginatedResponse::new(
            paginated, total, page, per_page,
        ))
    }
}
