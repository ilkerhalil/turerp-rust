//! Vendor Portal API endpoints (v1)
//!
//! Self-service portal for vendors to view purchase orders, invoices, payments,
//! and manage delivery notes.

use std::collections::HashMap;

use actix_web::{dev::Payload, web, HttpRequest, HttpResponse};

use crate::domain::vendor_portal::model::{
    CreateDeliveryNote, CreateVendorUser, VendorLoginRequest, VendorPaginationParams,
};
use crate::domain::vendor_portal::service::VendorPortal;
use crate::error::{ApiError, ApiResult};
use crate::i18n::{resolve, I18n, Locale};
use crate::utils::jwt::{JwtService, VendorAuthClaims};

/// Vendor auth user extractor - independently validates vendor JWT tokens
pub struct VendorAuthUser(pub VendorAuthClaims);

impl actix_web::FromRequest for VendorAuthUser {
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
                jwt.decode_vendor_token(token)
                    .map_err(|e| actix_web::error::ErrorUnauthorized(e.to_string()))
            }
            _ => Err(actix_web::error::ErrorUnauthorized(
                "Authentication required",
            )),
        };

        std::future::ready(result.map(VendorAuthUser))
    }
}

/// Register a new vendor user (public)
#[utoipa::path(
    post, path = "/api/v1/vendor-portal/register", tag = "Vendor Portal",
    request_body = CreateVendorUser,
    responses((status = 201, description = "Vendor user registered", body = crate::domain::vendor_portal::model::VendorUser), (status = 400, description = "Validation error", body = crate::error::ErrorResponse)),
)]
pub async fn register_vendor_user(
    vendor_service: web::Data<dyn VendorPortal>,
    payload: web::Json<CreateVendorUser>,
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
    match vendor_service.register(tenant_id, request).await {
        Ok(response) => Ok(HttpResponse::Created().json(response)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Login to vendor portal (public)
#[utoipa::path(
    post, path = "/api/v1/vendor-portal/login", tag = "Vendor Portal",
    request_body = VendorLoginRequest,
    responses((status = 200, description = "Login successful", body = crate::domain::vendor_portal::model::VendorAuthResponse), (status = 401, description = "Invalid credentials", body = crate::error::ErrorResponse)),
)]
pub async fn login_vendor_user(
    vendor_service: web::Data<dyn VendorPortal>,
    payload: web::Json<VendorLoginRequest>,
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
    match vendor_service.login(tenant_id, request).await {
        Ok(response) => Ok(HttpResponse::Ok().json(response)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get purchase order history for the logged-in vendor
#[utoipa::path(
    get, path = "/api/v1/vendor-portal/orders", tag = "Vendor Portal",
    params(VendorPaginationParams),
    responses((status = 200, description = "Vendor orders", body = crate::domain::vendor_portal::model::VendorPaginatedResponse<crate::domain::vendor_portal::model::VendorOrderView>), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn get_vendor_orders(
    vendor_user: VendorAuthUser,
    vendor_service: web::Data<dyn VendorPortal>,
    pagination: web::Query<VendorPaginationParams>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let cari_id = vendor_user.0.cari_id;
    let tenant_id = vendor_user.0.tenant_id;
    match vendor_service
        .get_orders(cari_id, tenant_id, pagination.into_inner())
        .await
    {
        Ok(orders) => Ok(HttpResponse::Ok().json(orders)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get invoices for the logged-in vendor
#[utoipa::path(
    get, path = "/api/v1/vendor-portal/invoices", tag = "Vendor Portal",
    params(VendorPaginationParams),
    responses((status = 200, description = "Vendor invoices", body = crate::domain::vendor_portal::model::VendorPaginatedResponse<crate::domain::vendor_portal::model::VendorInvoiceView>), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn get_vendor_invoices(
    vendor_user: VendorAuthUser,
    vendor_service: web::Data<dyn VendorPortal>,
    pagination: web::Query<VendorPaginationParams>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let cari_id = vendor_user.0.cari_id;
    let tenant_id = vendor_user.0.tenant_id;
    match vendor_service
        .get_invoices(cari_id, tenant_id, pagination.into_inner())
        .await
    {
        Ok(invoices) => Ok(HttpResponse::Ok().json(invoices)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get payment history for the logged-in vendor
#[utoipa::path(
    get, path = "/api/v1/vendor-portal/payments", tag = "Vendor Portal",
    params(VendorPaginationParams),
    responses((status = 200, description = "Vendor payments", body = crate::domain::vendor_portal::model::VendorPaginatedResponse<crate::domain::vendor_portal::model::VendorPaymentView>), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn get_vendor_payments(
    vendor_user: VendorAuthUser,
    vendor_service: web::Data<dyn VendorPortal>,
    pagination: web::Query<VendorPaginationParams>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let cari_id = vendor_user.0.cari_id;
    let tenant_id = vendor_user.0.tenant_id;
    match vendor_service
        .get_payments(cari_id, tenant_id, pagination.into_inner())
        .await
    {
        Ok(payments) => Ok(HttpResponse::Ok().json(payments)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// PDF download for an invoice
#[utoipa::path(
    get, path = "/api/v1/vendor-portal/invoices/{id}/pdf", tag = "Vendor Portal",
    params(("id" = i64, Path, description = "Invoice ID")),
    responses((status = 200, description = "PDF bytes"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn get_vendor_invoice_pdf(
    vendor_user: VendorAuthUser,
    path: web::Path<i64>,
    vendor_service: web::Data<dyn VendorPortal>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let invoice_id = path.into_inner();
    let cari_id = vendor_user.0.cari_id;
    let tenant_id = vendor_user.0.tenant_id;

    match vendor_service
        .get_invoice_pdf(invoice_id, cari_id, tenant_id)
        .await
    {
        Ok(pdf_bytes) => Ok(HttpResponse::Ok()
            .content_type("application/pdf")
            .body(pdf_bytes)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Create a delivery note
#[utoipa::path(
    post, path = "/api/v1/vendor-portal/delivery-notes", tag = "Vendor Portal",
    request_body = CreateDeliveryNote,
    responses((status = 201, description = "Delivery note created", body = crate::domain::vendor_portal::model::DeliveryNote), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_delivery_note(
    vendor_user: VendorAuthUser,
    vendor_service: web::Data<dyn VendorPortal>,
    payload: web::Json<CreateDeliveryNote>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let cari_id = vendor_user.0.cari_id;
    let tenant_id = vendor_user.0.tenant_id;
    let vendor_user_id = vendor_user.0.vendor_user_id()?;

    match vendor_service
        .create_delivery_note(vendor_user_id, cari_id, tenant_id, payload.into_inner())
        .await
    {
        Ok(note) => Ok(HttpResponse::Created().json(note)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get delivery notes for the logged-in vendor
#[utoipa::path(
    get, path = "/api/v1/vendor-portal/delivery-notes", tag = "Vendor Portal",
    params(VendorPaginationParams),
    responses((status = 200, description = "Vendor delivery notes", body = crate::domain::vendor_portal::model::VendorPaginatedResponse<crate::domain::vendor_portal::model::DeliveryNote>), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn get_delivery_notes(
    vendor_user: VendorAuthUser,
    vendor_service: web::Data<dyn VendorPortal>,
    pagination: web::Query<VendorPaginationParams>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let tenant_id = vendor_user.0.tenant_id;
    let vendor_user_id = vendor_user.0.vendor_user_id()?;

    match vendor_service
        .get_delivery_notes(vendor_user_id, tenant_id, pagination.into_inner())
        .await
    {
        Ok(notes) => Ok(HttpResponse::Ok().json(notes)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Configure vendor portal routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/v1/vendor-portal/register").route(web::post().to(register_vendor_user)),
    )
    .service(web::resource("/v1/vendor-portal/login").route(web::post().to(login_vendor_user)))
    .service(web::resource("/v1/vendor-portal/orders").route(web::get().to(get_vendor_orders)))
    .service(web::resource("/v1/vendor-portal/invoices").route(web::get().to(get_vendor_invoices)))
    .service(web::resource("/v1/vendor-portal/payments").route(web::get().to(get_vendor_payments)))
    .service(
        web::resource("/v1/vendor-portal/invoices/{id}/pdf")
            .route(web::get().to(get_vendor_invoice_pdf)),
    )
    .service(
        web::resource("/v1/vendor-portal/delivery-notes")
            .route(web::post().to(create_delivery_note))
            .route(web::get().to(get_delivery_notes)),
    );
}
