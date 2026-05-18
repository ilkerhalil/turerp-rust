//! Customer Portal API endpoints (v1)
//!
//! Self-service portal for customers to view orders, invoices, payments,
//! and manage support tickets.

use std::collections::HashMap;

use actix_web::{dev::Payload, web, HttpRequest, HttpResponse};

use crate::domain::customer_portal::model::{
    CreatePortalUser, CreateSupportTicket, PortalLoginRequest, PortalPaginationParams,
};
use crate::domain::customer_portal::service::CustomerPortal;
use crate::error::{ApiError, ApiResult};
use crate::i18n::{resolve, I18n, Locale};
use crate::utils::jwt::{JwtService, PortalAuthClaims};

/// Portal auth user extractor - independently validates portal JWT tokens
pub struct PortalAuthUser(pub PortalAuthClaims);

impl actix_web::FromRequest for PortalAuthUser {
    type Error = actix_web::Error;
    type Future = std::future::Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let jwt_service = req.app_data::<web::Data<JwtService>>().cloned();
        let auth_header = req
            .headers()
            .get("Authorization")
            .and_then(|h| h.to_str().ok());

        let result = match (jwt_service, auth_header) {
            (Some(jwt), Some(header)) if header.starts_with("Bearer ") => {
                let token = &header[7..];
                jwt.decode_portal_token(token)
                    .map_err(|e| actix_web::error::ErrorUnauthorized(e.to_string()))
            }
            _ => Err(actix_web::error::ErrorUnauthorized(
                "Authentication required",
            )),
        };

        std::future::ready(result.map(PortalAuthUser))
    }
}

/// Register a new portal user (public)
#[utoipa::path(
    post, path = "/api/v1/customer-portal/register", tag = "Customer Portal",
    request_body = CreatePortalUser,
    responses((status = 201, description = "Portal user registered", body = crate::domain::customer_portal::model::PortalUser), (status = 400, description = "Validation error", body = crate::error::ErrorResponse)),
)]
pub async fn register_portal_user(
    portal_service: web::Data<dyn CustomerPortal>,
    payload: web::Json<CreatePortalUser>,
    query: web::Query<HashMap<String, String>>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let tenant_id = query
        .get("tenant_id")
        .and_then(|s| s.parse::<i64>().ok())
        .ok_or_else(|| ApiError::BadRequest("tenant_id query parameter is required".to_string()))?;
    let request = payload.into_inner();
    match portal_service.register(tenant_id, request).await {
        Ok(response) => Ok(HttpResponse::Created().json(response)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Login to customer portal (public)
#[utoipa::path(
    post, path = "/api/v1/customer-portal/login", tag = "Customer Portal",
    request_body = PortalLoginRequest,
    responses((status = 200, description = "Login successful", body = crate::domain::customer_portal::model::PortalAuthResponse), (status = 401, description = "Invalid credentials", body = crate::error::ErrorResponse)),
)]
pub async fn login_portal_user(
    portal_service: web::Data<dyn CustomerPortal>,
    payload: web::Json<PortalLoginRequest>,
    query: web::Query<HashMap<String, String>>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let tenant_id = query
        .get("tenant_id")
        .and_then(|s| s.parse::<i64>().ok())
        .ok_or_else(|| ApiError::BadRequest("tenant_id query parameter is required".to_string()))?;
    let request = payload.into_inner();
    match portal_service.login(tenant_id, request).await {
        Ok(response) => Ok(HttpResponse::Ok().json(response)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get order history for the logged-in customer
#[utoipa::path(
    get, path = "/api/v1/customer-portal/orders", tag = "Customer Portal",
    params(PortalPaginationParams),
    responses((status = 200, description = "Customer orders", body = crate::domain::customer_portal::model::PortalPaginatedResponse<crate::domain::customer_portal::model::CustomerOrderView>), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn get_customer_orders(
    portal_user: PortalAuthUser,
    portal_service: web::Data<dyn CustomerPortal>,
    pagination: web::Query<PortalPaginationParams>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let cari_id = portal_user.0.cari_id;
    let tenant_id = portal_user.0.tenant_id;
    match portal_service
        .get_orders(cari_id, tenant_id, pagination.into_inner())
        .await
    {
        Ok(orders) => Ok(HttpResponse::Ok().json(orders)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get invoices for the logged-in customer
#[utoipa::path(
    get, path = "/api/v1/customer-portal/invoices", tag = "Customer Portal",
    params(PortalPaginationParams),
    responses((status = 200, description = "Customer invoices", body = crate::domain::customer_portal::model::PortalPaginatedResponse<crate::domain::customer_portal::model::CustomerInvoiceView>), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn get_customer_invoices(
    portal_user: PortalAuthUser,
    portal_service: web::Data<dyn CustomerPortal>,
    pagination: web::Query<PortalPaginationParams>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let cari_id = portal_user.0.cari_id;
    let tenant_id = portal_user.0.tenant_id;
    match portal_service
        .get_invoices(cari_id, tenant_id, pagination.into_inner())
        .await
    {
        Ok(invoices) => Ok(HttpResponse::Ok().json(invoices)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get payment history for the logged-in customer
#[utoipa::path(
    get, path = "/api/v1/customer-portal/payments", tag = "Customer Portal",
    params(PortalPaginationParams),
    responses((status = 200, description = "Customer payments", body = crate::domain::customer_portal::model::PortalPaginatedResponse<crate::domain::customer_portal::model::CustomerPaymentView>), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn get_customer_payments(
    portal_user: PortalAuthUser,
    portal_service: web::Data<dyn CustomerPortal>,
    pagination: web::Query<PortalPaginationParams>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let cari_id = portal_user.0.cari_id;
    let tenant_id = portal_user.0.tenant_id;
    match portal_service
        .get_payments(cari_id, tenant_id, pagination.into_inner())
        .await
    {
        Ok(payments) => Ok(HttpResponse::Ok().json(payments)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// PDF download for an invoice
#[utoipa::path(
    get, path = "/api/v1/customer-portal/invoices/{id}/pdf", tag = "Customer Portal",
    params(("id" = i64, Path, description = "Invoice ID")),
    responses((status = 200, description = "PDF bytes"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn get_invoice_pdf(
    portal_user: PortalAuthUser,
    path: web::Path<i64>,
    portal_service: web::Data<dyn CustomerPortal>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let invoice_id = path.into_inner();
    let cari_id = portal_user.0.cari_id;
    let tenant_id = portal_user.0.tenant_id;

    match portal_service
        .get_invoice_pdf(invoice_id, cari_id, tenant_id)
        .await
    {
        Ok(pdf_bytes) => Ok(HttpResponse::Ok()
            .content_type("application/pdf")
            .body(pdf_bytes)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Create a support ticket
#[utoipa::path(
    post, path = "/api/v1/customer-portal/support-tickets", tag = "Customer Portal",
    request_body = CreateSupportTicket,
    responses((status = 201, description = "Ticket created", body = crate::domain::customer_portal::model::SupportTicket), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_support_ticket(
    portal_user: PortalAuthUser,
    portal_service: web::Data<dyn CustomerPortal>,
    payload: web::Json<CreateSupportTicket>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let cari_id = portal_user.0.cari_id;
    let tenant_id = portal_user.0.tenant_id;
    let portal_user_id = portal_user.0.portal_user_id()?;

    match portal_service
        .create_support_ticket(portal_user_id, cari_id, tenant_id, payload.into_inner())
        .await
    {
        Ok(ticket) => Ok(HttpResponse::Created().json(ticket)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get support tickets for the logged-in customer
#[utoipa::path(
    get, path = "/api/v1/customer-portal/support-tickets", tag = "Customer Portal",
    params(PortalPaginationParams),
    responses((status = 200, description = "Customer support tickets", body = crate::domain::customer_portal::model::PortalPaginatedResponse<crate::domain::customer_portal::model::SupportTicket>), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn get_support_tickets(
    portal_user: PortalAuthUser,
    portal_service: web::Data<dyn CustomerPortal>,
    pagination: web::Query<PortalPaginationParams>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let tenant_id = portal_user.0.tenant_id;
    let portal_user_id = portal_user.0.portal_user_id()?;

    match portal_service
        .get_support_tickets(portal_user_id, tenant_id, pagination.into_inner())
        .await
    {
        Ok(tickets) => Ok(HttpResponse::Ok().json(tickets)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Configure customer portal routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/v1/customer-portal/register").route(web::post().to(register_portal_user)),
    )
    .service(web::resource("/v1/customer-portal/login").route(web::post().to(login_portal_user)))
    .service(web::resource("/v1/customer-portal/orders").route(web::get().to(get_customer_orders)))
    .service(
        web::resource("/v1/customer-portal/invoices").route(web::get().to(get_customer_invoices)),
    )
    .service(
        web::resource("/v1/customer-portal/payments").route(web::get().to(get_customer_payments)),
    )
    .service(
        web::resource("/v1/customer-portal/invoices/{id}/pdf")
            .route(web::get().to(get_invoice_pdf)),
    )
    .service(
        web::resource("/v1/customer-portal/support-tickets")
            .route(web::post().to(create_support_ticket))
            .route(web::get().to(get_support_tickets)),
    );
}
