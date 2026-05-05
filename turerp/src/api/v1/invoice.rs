//! Invoice API endpoints (v1)

use actix_web::{web, HttpResponse};

use crate::common::pagination::PaginationParams;
use crate::common::MessageResponse;
use crate::domain::invoice::model::{CreateInvoice, InvoiceResponse, InvoiceStatus};
use crate::domain::invoice::service::InvoiceService;
use crate::error::{ApiError, ApiResult};
use crate::i18n::{resolve, I18n, Locale};
use crate::middleware::{AdminUser, AuthUser};

/// Create invoice (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/invoices", tag = "Invoice",
    request_body = CreateInvoice,
    responses((status = 201, description = "Invoice created"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_invoice(
    admin_user: AdminUser,
    invoice_service: web::Data<InvoiceService>,
    payload: web::Json<CreateInvoice>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    match invoice_service.create_invoice(create).await {
        Ok(invoice) => Ok(HttpResponse::Created().json(invoice)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get invoice by ID
#[utoipa::path(
    get, path = "/api/v1/invoices/{id}", tag = "Invoice",
    params(("id" = i64, Path, description = "Invoice ID")),
    responses((status = 200, description = "Invoice found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_invoice(
    auth_user: AuthUser,
    invoice_service: web::Data<InvoiceService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match invoice_service
        .get_invoice(*path, auth_user.0.tenant_id)
        .await
    {
        Ok(invoice) => Ok(HttpResponse::Ok().json(invoice)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get all invoices
#[utoipa::path(
    get, path = "/api/v1/invoices", tag = "Invoice",
    params(PaginationParams),
    responses((status = 200, description = "Paginated list of invoices")),
    security(("bearer_auth" = []))
)]
pub async fn get_invoices(
    auth_user: AuthUser,
    invoice_service: web::Data<InvoiceService>,
    pagination: web::Query<PaginationParams>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    if let Err(e) = pagination.validate() {
        let err = ApiError::Validation(e.to_string());
        return Ok(err.to_http_response(i18n, locale.as_str()));
    }
    match invoice_service
        .get_invoices_by_tenant_paginated(
            auth_user.0.tenant_id,
            pagination.page,
            pagination.per_page,
        )
        .await
    {
        Ok(result) => Ok(HttpResponse::Ok().json(result)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get invoices by status
#[utoipa::path(
    get, path = "/api/v1/invoices/status/{status}", tag = "Invoice",
    params(("status" = InvoiceStatus, Path, description = "Invoice status"), PaginationParams),
    responses((status = 200, description = "Paginated list of invoices by status")),
    security(("bearer_auth" = []))
)]
pub async fn get_invoices_by_status(
    auth_user: AuthUser,
    invoice_service: web::Data<InvoiceService>,
    path: web::Path<InvoiceStatus>,
    pagination: web::Query<PaginationParams>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    if let Err(e) = pagination.validate() {
        let err = ApiError::Validation(e.to_string());
        return Ok(err.to_http_response(i18n, locale.as_str()));
    }
    match invoice_service
        .get_invoices_by_status_paginated(
            auth_user.0.tenant_id,
            path.into_inner(),
            pagination.page,
            pagination.per_page,
        )
        .await
    {
        Ok(result) => Ok(HttpResponse::Ok().json(result)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get outstanding invoices
#[utoipa::path(
    get, path = "/api/v1/invoices/outstanding", tag = "Invoice",
    responses((status = 200, description = "Outstanding invoices")),
    security(("bearer_auth" = []))
)]
pub async fn get_outstanding_invoices(
    auth_user: AuthUser,
    invoice_service: web::Data<InvoiceService>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match invoice_service
        .get_outstanding_invoices(auth_user.0.tenant_id)
        .await
    {
        Ok(invoices) => Ok(HttpResponse::Ok().json(invoices)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get overdue invoices
#[utoipa::path(
    get, path = "/api/v1/invoices/overdue", tag = "Invoice",
    responses((status = 200, description = "Overdue invoices")),
    security(("bearer_auth" = []))
)]
pub async fn get_overdue_invoices(
    auth_user: AuthUser,
    invoice_service: web::Data<InvoiceService>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match invoice_service
        .get_overdue_invoices(auth_user.0.tenant_id)
        .await
    {
        Ok(invoices) => Ok(HttpResponse::Ok().json(invoices)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Update invoice status (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/invoices/{id}/status", tag = "Invoice",
    params(("id" = i64, Path, description = "Invoice ID")),
    request_body = UpdateStatusRequest,
    responses((status = 200, description = "Status updated"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn update_invoice_status(
    admin_user: AdminUser,
    invoice_service: web::Data<InvoiceService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateStatusRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match invoice_service
        .update_invoice_status(*path, admin_user.0.tenant_id, payload.into_inner().status)
        .await
    {
        Ok(invoice) => Ok(HttpResponse::Ok().json(invoice)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Delete invoice (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/invoices/{id}", tag = "Invoice",
    params(("id" = i64, Path, description = "Invoice ID")),
    responses((status = 200, description = "Invoice deleted", body = MessageResponse), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn delete_invoice(
    admin_user: AdminUser,
    invoice_service: web::Data<InvoiceService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match invoice_service
        .soft_delete_invoice(*path, admin_user.0.tenant_id, admin_user.0.user_id()?)
        .await
    {
        Ok(()) => {
            let msg = i18n.t(locale.as_str(), "invoice.deleted");
            Ok(HttpResponse::Ok().json(MessageResponse { message: msg }))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Restore a soft-deleted invoice (admin only)
#[utoipa::path(
    put, path = "/api/v1/invoices/{id}/restore", tag = "Invoice",
    params(("id" = i64, Path, description = "Invoice ID")),
    responses(
        (status = 200, description = "Invoice restored", body = InvoiceResponse),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Invoice not found or not deleted"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn restore_invoice(
    admin_user: AdminUser,
    invoice_service: web::Data<InvoiceService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let invoice = invoice_service
        .restore_invoice(*path, admin_user.0.tenant_id)
        .await?;
    let lines = invoice_service.get_invoice_lines(invoice.id).await?;
    let response = InvoiceResponse::from((invoice, lines));
    Ok(HttpResponse::Ok().json(response))
}

/// List soft-deleted invoices (admin only)
#[utoipa::path(
    get, path = "/api/v1/invoices/deleted", tag = "Invoice",
    responses(
        (status = 200, description = "List of deleted invoices"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_deleted_invoices(
    admin_user: AdminUser,
    invoice_service: web::Data<InvoiceService>,
) -> ApiResult<HttpResponse> {
    let invoices = invoice_service
        .list_deleted_invoices(admin_user.0.tenant_id)
        .await?;
    let responses: Vec<InvoiceResponse> = invoices
        .into_iter()
        .map(|inv| {
            let lines = Vec::new(); // Deleted invoices don't need lines
            InvoiceResponse::from((inv, lines))
        })
        .collect();
    Ok(HttpResponse::Ok().json(responses))
}

/// Permanently delete an invoice (admin only)
#[utoipa::path(
    delete, path = "/api/v1/invoices/{id}/destroy", tag = "Invoice",
    params(("id" = i64, Path, description = "Invoice ID")),
    responses(
        (status = 204, description = "Invoice permanently deleted"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Invoice not found"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn destroy_invoice(
    admin_user: AdminUser,
    invoice_service: web::Data<InvoiceService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    invoice_service
        .destroy_invoice(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

/// Add payment to invoice (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/invoices/payments", tag = "Invoice",
    request_body = crate::domain::invoice::model::CreatePayment,
    responses((status = 201, description = "Payment created"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_payment(
    admin_user: AdminUser,
    invoice_service: web::Data<InvoiceService>,
    payload: web::Json<crate::domain::invoice::model::CreatePayment>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    match invoice_service.create_payment(create).await {
        Ok(payment) => Ok(HttpResponse::Created().json(payment)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get payments by invoice
#[utoipa::path(
    get, path = "/api/v1/invoices/{id}/payments", tag = "Invoice",
    params(("id" = i64, Path, description = "Invoice ID")),
    responses((status = 200, description = "Payments for invoice")),
    security(("bearer_auth" = []))
)]
pub async fn get_payments_by_invoice(
    _auth_user: AuthUser,
    invoice_service: web::Data<InvoiceService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match invoice_service.get_payments_by_invoice(*path).await {
        Ok(payments) => Ok(HttpResponse::Ok().json(payments)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct UpdateStatusRequest {
    pub status: InvoiceStatus,
}

/// Configure invoice routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/v1/invoices")
            .route(web::get().to(get_invoices))
            .route(web::post().to(create_invoice)),
    )
    .service(web::resource("/v1/invoices/deleted").route(web::get().to(list_deleted_invoices)))
    .service(
        web::resource("/v1/invoices/outstanding").route(web::get().to(get_outstanding_invoices)),
    )
    .service(web::resource("/v1/invoices/overdue").route(web::get().to(get_overdue_invoices)))
    .service(
        web::resource("/v1/invoices/status/{status}").route(web::get().to(get_invoices_by_status)),
    )
    // MUST register /payments BEFORE /{id} to avoid route shadowing
    .service(web::resource("/v1/invoices/payments").route(web::post().to(create_payment)))
    .service(
        web::resource("/v1/invoices/{id}")
            .route(web::get().to(get_invoice))
            .route(web::delete().to(delete_invoice)),
    )
    .service(web::resource("/v1/invoices/{id}/status").route(web::put().to(update_invoice_status)))
    .service(
        web::resource("/v1/invoices/{id}/payments").route(web::get().to(get_payments_by_invoice)),
    )
    .service(web::resource("/v1/invoices/{id}/restore").route(web::put().to(restore_invoice)))
    .service(web::resource("/v1/invoices/{id}/destroy").route(web::delete().to(destroy_invoice)));
}
