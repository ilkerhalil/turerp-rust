//! Invoice API endpoints (v1)

use actix_web::{web, HttpResponse};

use crate::common::pagination::PaginationParams;
use crate::domain::invoice::model::{CreateInvoice, InvoiceStatus};
use crate::domain::invoice::service::InvoiceService;
use crate::error::ApiResult;
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
) -> ApiResult<HttpResponse> {
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    let invoice = invoice_service.create_invoice(create).await?;
    Ok(HttpResponse::Created().json(invoice))
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
) -> ApiResult<HttpResponse> {
    let invoice = invoice_service
        .get_invoice(*path, auth_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(invoice))
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
) -> ApiResult<HttpResponse> {
    pagination
        .validate()
        .map_err(crate::error::ApiError::Validation)?;
    let result = invoice_service
        .get_invoices_by_tenant_paginated(
            auth_user.0.tenant_id,
            pagination.page,
            pagination.per_page,
        )
        .await?;
    Ok(HttpResponse::Ok().json(result))
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
) -> ApiResult<HttpResponse> {
    pagination
        .validate()
        .map_err(crate::error::ApiError::Validation)?;
    let result = invoice_service
        .get_invoices_by_status_paginated(
            auth_user.0.tenant_id,
            path.into_inner(),
            pagination.page,
            pagination.per_page,
        )
        .await?;
    Ok(HttpResponse::Ok().json(result))
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
) -> ApiResult<HttpResponse> {
    let invoices = invoice_service
        .get_outstanding_invoices(auth_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(invoices))
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
) -> ApiResult<HttpResponse> {
    let invoices = invoice_service
        .get_overdue_invoices(auth_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(invoices))
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
    _admin_user: AdminUser,
    invoice_service: web::Data<InvoiceService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateStatusRequest>,
) -> ApiResult<HttpResponse> {
    let invoice = invoice_service
        .update_invoice_status(*path, _admin_user.0.tenant_id, payload.into_inner().status)
        .await?;
    Ok(HttpResponse::Ok().json(invoice))
}

/// Delete invoice (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/invoices/{id}", tag = "Invoice",
    params(("id" = i64, Path, description = "Invoice ID")),
    responses((status = 204, description = "Invoice deleted"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn delete_invoice(
    _admin_user: AdminUser,
    invoice_service: web::Data<InvoiceService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    invoice_service
        .delete_invoice(*path, _admin_user.0.tenant_id)
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
) -> ApiResult<HttpResponse> {
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    let payment = invoice_service.create_payment(create).await?;
    Ok(HttpResponse::Created().json(payment))
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
) -> ApiResult<HttpResponse> {
    let payments = invoice_service.get_payments_by_invoice(*path).await?;
    Ok(HttpResponse::Ok().json(payments))
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
    );
}
