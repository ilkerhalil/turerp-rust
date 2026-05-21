//! Company API endpoints (v1)

use actix_web::{web, HttpResponse};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::common::pagination::PaginationParams;
use crate::common::MessageResponse;
use crate::domain::company::model::{CompanyResponse, CreateCompany, UpdateCompany};
use crate::domain::company::service::CompanyService;
use crate::domain::inter_company::{InterCompanyInvoiceLine, InterCompanyService};
use crate::domain::invoice::model::{InvoiceStatus, InvoiceType};
use crate::domain::invoice::service::InvoiceService;
use crate::error::{ApiError, ApiResult};
use crate::i18n::{resolve, I18n, Locale};
use crate::json_resp;
use crate::middleware::{AdminUser, AuthUser};

// ---------------------------------------------------------------------------
// Request / Response DTOs
// ---------------------------------------------------------------------------

/// Request to create a cross-company invoice.
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct CreateCrossCompanyInvoiceRequest {
    pub seller_company_id: i64,
    pub buyer_company_id: i64,
    #[validate(length(min = 1))]
    pub lines: Vec<InterCompanyInvoiceLine>,
}

/// Request to transfer stock between companies.
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct TransferStockRequest {
    pub from_company_id: i64,
    pub to_company_id: i64,
    pub product_id: i64,
    pub warehouse_id: i64,
    pub quantity: Decimal,
}

/// Financial summary for a single company within a consolidated report.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CompanyFinancialSummary {
    pub company: CompanyResponse,
    pub total_sales: rust_decimal::Decimal,
    pub total_purchases: rust_decimal::Decimal,
    pub outstanding_receivables: rust_decimal::Decimal,
    pub outstanding_payables: rust_decimal::Decimal,
    pub net_position: rust_decimal::Decimal,
}

/// Consolidated financial report across all companies in a tenant.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ConsolidatedReport {
    pub tenant_id: i64,
    pub total_companies: usize,
    pub companies: Vec<CompanyFinancialSummary>,
    pub consolidated_sales: rust_decimal::Decimal,
    pub consolidated_purchases: rust_decimal::Decimal,
    pub consolidated_net_position: rust_decimal::Decimal,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// Create a company (requires admin role)
#[utoipa::path(
    post,
    path = "/api/v1/companies",
    tag = "Companies",
    request_body = CreateCompany,
    responses(
        (status = 201, description = "Company created successfully"),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required")
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_company(
    admin_user: AdminUser,
    company_service: web::Data<CompanyService>,
    payload: web::Json<CreateCompany>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    json_resp!(
        company_service.create_company(create),
        HttpResponse::Created,
        i18n,
        locale.as_str()
    )
}

/// Get all companies (requires authentication)
#[utoipa::path(
    get,
    path = "/api/v1/companies",
    tag = "Companies",
    params(PaginationParams),
    responses(
        (status = 200, description = "Paginated list of companies"),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_companies(
    auth_user: AuthUser,
    company_service: web::Data<CompanyService>,
    pagination: web::Query<PaginationParams>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    if let Err(e) = pagination.validate() {
        let err = ApiError::Validation(e.to_string());
        return Ok(err.to_http_response(i18n, locale.as_str()));
    }
    json_resp!(
        company_service.get_all_companies_paginated(
            auth_user.0.tenant_id,
            pagination.page,
            pagination.per_page
        ),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Get company by ID (requires authentication)
#[utoipa::path(
    get,
    path = "/api/v1/companies/{id}",
    tag = "Companies",
    params(("id" = i64, Path, description = "Company ID")),
    responses(
        (status = 200, description = "Company found"),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "Company not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_company(
    auth_user: AuthUser,
    company_service: web::Data<CompanyService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        company_service.get_company(*path, auth_user.0.tenant_id),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Update a company (requires admin role)
#[utoipa::path(
    put,
    path = "/api/v1/companies/{id}",
    tag = "Companies",
    params(("id" = i64, Path, description = "Company ID")),
    request_body = UpdateCompany,
    responses(
        (status = 200, description = "Company updated successfully"),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 404, description = "Company not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_company(
    admin_user: AdminUser,
    company_service: web::Data<CompanyService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateCompany>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let update = payload.into_inner();
    json_resp!(
        company_service.update_company(*path, admin_user.0.tenant_id, update),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Soft-delete a company (requires admin role)
#[utoipa::path(
    delete,
    path = "/api/v1/companies/{id}",
    tag = "Companies",
    params(("id" = i64, Path, description = "Company ID")),
    responses(
        (status = 200, description = "Company deleted successfully"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 404, description = "Company not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_company(
    admin_user: AdminUser,
    company_service: web::Data<CompanyService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match company_service
        .delete_company(*path, admin_user.0.tenant_id, admin_user.0.user_id()?)
        .await
    {
        Ok(_) => Ok(HttpResponse::Ok().json(MessageResponse {
            message: "Company deleted successfully".to_string(),
        })),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Restore a soft-deleted company (requires admin role)
#[utoipa::path(
    post,
    path = "/api/v1/companies/{id}/restore",
    tag = "Companies",
    params(("id" = i64, Path, description = "Company ID")),
    responses(
        (status = 200, description = "Company restored successfully"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 404, description = "Company not found or not deleted")
    ),
    security(("bearer_auth" = []))
)]
pub async fn restore_company(
    admin_user: AdminUser,
    company_service: web::Data<CompanyService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        company_service.restore_company(*path, admin_user.0.tenant_id),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// List soft-deleted companies (requires admin role)
#[utoipa::path(
    get,
    path = "/api/v1/companies/deleted",
    tag = "Companies",
    responses(
        (status = 200, description = "List of deleted companies"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required")
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_deleted_companies(
    admin_user: AdminUser,
    company_service: web::Data<CompanyService>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        company_service.list_deleted_companies(admin_user.0.tenant_id),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Permanently delete a company (requires admin role)
#[utoipa::path(
    delete,
    path = "/api/v1/companies/{id}/destroy",
    tag = "Companies",
    params(("id" = i64, Path, description = "Company ID")),
    responses(
        (status = 200, description = "Company destroyed successfully"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 404, description = "Company not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn destroy_company(
    admin_user: AdminUser,
    company_service: web::Data<CompanyService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match company_service
        .destroy_company(*path, admin_user.0.tenant_id)
        .await
    {
        Ok(_) => Ok(HttpResponse::Ok().json(MessageResponse {
            message: "Company destroyed successfully".to_string(),
        })),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Create a cross-company invoice (requires admin role)
#[utoipa::path(
    post,
    path = "/api/v1/companies/cross-invoice",
    tag = "Companies",
    request_body = CreateCrossCompanyInvoiceRequest,
    responses(
        (status = 201, description = "Cross-company invoice created"),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required")
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_cross_company_invoice(
    admin_user: AdminUser,
    inter_company_service: web::Data<InterCompanyService>,
    payload: web::Json<CreateCrossCompanyInvoiceRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let req = payload.into_inner();
    if let Err(e) = req.validate() {
        let err = ApiError::Validation(e.to_string());
        return Ok(err.to_http_response(i18n, locale.as_str()));
    }
    json_resp!(
        inter_company_service.create_cross_company_invoice(
            admin_user.0.tenant_id,
            req.seller_company_id,
            req.buyer_company_id,
            req.lines,
        ),
        HttpResponse::Created,
        i18n,
        locale.as_str()
    )
}

/// Transfer stock between companies (requires admin role)
#[utoipa::path(
    post,
    path = "/api/v1/companies/stock-transfer",
    tag = "Companies",
    request_body = TransferStockRequest,
    responses(
        (status = 201, description = "Stock transfer completed"),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required")
    ),
    security(("bearer_auth" = []))
)]
pub async fn transfer_stock(
    admin_user: AdminUser,
    inter_company_service: web::Data<InterCompanyService>,
    payload: web::Json<TransferStockRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let req = payload.into_inner();
    if let Err(e) = req.validate() {
        let err = ApiError::Validation(e.to_string());
        return Ok(err.to_http_response(i18n, locale.as_str()));
    }
    let user_id = match admin_user.0.user_id() {
        Ok(id) => id,
        Err(e) => return Ok(e.to_http_response(i18n, locale.as_str())),
    };
    json_resp!(
        inter_company_service.transfer_stock_between_companies(
            admin_user.0.tenant_id,
            req.from_company_id,
            req.to_company_id,
            req.product_id,
            req.warehouse_id,
            req.quantity,
            user_id,
        ),
        HttpResponse::Created,
        i18n,
        locale.as_str()
    )
}

/// Consolidated report across companies (requires authentication)
#[utoipa::path(
    get,
    path = "/api/v1/companies/consolidated-report",
    tag = "Companies",
    responses(
        (status = 200, description = "Consolidated report"),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn consolidated_report(
    auth_user: AuthUser,
    company_service: web::Data<CompanyService>,
    invoice_service: web::Data<InvoiceService>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let tenant_id = auth_user.0.tenant_id;

    let companies = match company_service.get_all_companies(tenant_id).await {
        Ok(c) => c,
        Err(e) => return Ok(e.to_http_response(i18n, locale.as_str())),
    };

    let invoices = match invoice_service.get_invoices_by_tenant(tenant_id).await {
        Ok(i) => i,
        Err(e) => return Ok(e.to_http_response(i18n, locale.as_str())),
    };

    let mut summaries = Vec::with_capacity(companies.len());
    let mut consolidated_sales = rust_decimal::Decimal::ZERO;
    let mut consolidated_purchases = rust_decimal::Decimal::ZERO;

    for company in companies {
        let company_id = company.id;
        let mut total_sales = rust_decimal::Decimal::ZERO;
        let mut total_purchases = rust_decimal::Decimal::ZERO;
        let mut outstanding_receivables = rust_decimal::Decimal::ZERO;
        let mut outstanding_payables = rust_decimal::Decimal::ZERO;

        for invoice in &invoices {
            if invoice.company_id != company_id {
                continue;
            }
            // Skip cancelled and draft invoices from financial totals
            if matches!(
                invoice.status,
                InvoiceStatus::Cancelled | InvoiceStatus::Draft
            ) {
                continue;
            }

            match invoice.invoice_type {
                InvoiceType::SalesInvoice => {
                    total_sales += invoice.total_amount;
                    outstanding_receivables += invoice.total_amount - invoice.paid_amount;
                }
                InvoiceType::PurchaseInvoice => {
                    total_purchases += invoice.total_amount;
                    outstanding_payables += invoice.total_amount - invoice.paid_amount;
                }
                _ => {}
            }
        }

        let net_position = total_sales - total_purchases;
        consolidated_sales += total_sales;
        consolidated_purchases += total_purchases;

        summaries.push(CompanyFinancialSummary {
            company,
            total_sales,
            total_purchases,
            outstanding_receivables,
            outstanding_payables,
            net_position,
        });
    }

    let report = ConsolidatedReport {
        tenant_id,
        total_companies: summaries.len(),
        companies: summaries,
        consolidated_sales,
        consolidated_purchases,
        consolidated_net_position: consolidated_sales - consolidated_purchases,
    };

    Ok(HttpResponse::Ok().json(report))
}

// ---------------------------------------------------------------------------
// Route configuration
// ---------------------------------------------------------------------------

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/v1/companies")
            .route(web::get().to(get_companies))
            .route(web::post().to(create_company)),
    )
    .service(web::resource("/v1/companies/deleted").route(web::get().to(list_deleted_companies)))
    .service(
        web::resource("/v1/companies/consolidated-report")
            .route(web::get().to(consolidated_report)),
    )
    .service(
        web::resource("/v1/companies/cross-invoice")
            .route(web::post().to(create_cross_company_invoice)),
    )
    .service(web::resource("/v1/companies/stock-transfer").route(web::post().to(transfer_stock)))
    .service(
        web::resource("/v1/companies/{id}")
            .route(web::get().to(get_company))
            .route(web::put().to(update_company))
            .route(web::delete().to(delete_company)),
    )
    .service(web::resource("/v1/companies/{id}/restore").route(web::post().to(restore_company)))
    .service(web::resource("/v1/companies/{id}/destroy").route(web::delete().to(destroy_company)));
}
