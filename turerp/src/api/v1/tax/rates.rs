//! Tax rate handlers

use actix_web::{web, HttpResponse};
use serde::Deserialize;

use crate::common::pagination::{default_page, default_per_page, PaginationParams};
use crate::common::MessageResponse;
use crate::domain::tax::model::{
    BulkRestoreFailed, BulkRestoreResponse, CreateTaxRate, TaxRateResponse, TaxType, UpdateTaxRate,
};
use crate::domain::tax::service::TaxService;
use crate::error::ApiResult;
use crate::i18n::{resolve, I18n, Locale};
use crate::middleware::{AdminUser, AuthUser};

use super::{
    BulkRestoreRequest, CalculateInvoiceTaxRequest, CalculateTaxRequest, EffectiveRateQuery,
};

/// Query parameters for listing tax rates
#[derive(Debug, Deserialize)]
pub struct ListTaxRatesQuery {
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_per_page")]
    pub per_page: u32,
    pub tax_type: Option<String>,
}

impl From<ListTaxRatesQuery> for PaginationParams {
    fn from(q: ListTaxRatesQuery) -> Self {
        Self {
            page: q.page,
            per_page: q.per_page,
        }
    }
}

/// Create a tax rate (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/tax/rates", tag = "Tax",
    request_body = CreateTaxRate,
    responses((status = 201, description = "Tax rate created", body = TaxRateResponse), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_tax_rate(
    admin_user: AdminUser,
    tax_service: web::Data<TaxService>,
    payload: web::Json<CreateTaxRate>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let create = payload.into_inner();
    match tax_service
        .create_tax_rate(create, admin_user.0.tenant_id)
        .await
    {
        Ok(rate) => Ok(HttpResponse::Created().json(TaxRateResponse::from(rate))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// List tax rates (paginated, optional tax_type filter)
#[utoipa::path(
    get, path = "/api/v1/tax/rates", tag = "Tax",
    params(
        PaginationParams,
        ("tax_type" = Option<String>, Query, description = "Filter by tax type (KDV, OIV, BSMV, Damga, Stopaj, KV, GV)"),
    ),
    responses((status = 200, description = "List of tax rates")),
    security(("bearer_auth" = []))
)]
pub async fn list_tax_rates(
    auth_user: AuthUser,
    tax_service: web::Data<TaxService>,
    query: web::Query<ListTaxRatesQuery>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let q = query.into_inner();
    let tax_type_filter = q.tax_type.clone().and_then(|s| s.parse::<TaxType>().ok());
    match tax_service
        .list_tax_rates(auth_user.0.tenant_id, tax_type_filter, q.into())
        .await
    {
        Ok(result) => {
            let mapped = result.map(TaxRateResponse::from);
            Ok(HttpResponse::Ok().json(mapped))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get a tax rate by ID
#[utoipa::path(
    get, path = "/api/v1/tax/rates/{id}", tag = "Tax",
    params(("id" = i64, Path, description = "Tax rate ID")),
    responses((status = 200, description = "Tax rate found", body = TaxRateResponse), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_tax_rate(
    auth_user: AuthUser,
    tax_service: web::Data<TaxService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match tax_service.get_tax_rate(id, auth_user.0.tenant_id).await {
        Ok(rate) => Ok(HttpResponse::Ok().json(TaxRateResponse::from(rate))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Update a tax rate (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/tax/rates/{id}", tag = "Tax",
    params(("id" = i64, Path, description = "Tax rate ID")),
    request_body = UpdateTaxRate,
    responses((status = 200, description = "Tax rate updated", body = TaxRateResponse), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn update_tax_rate(
    admin_user: AdminUser,
    tax_service: web::Data<TaxService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateTaxRate>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    let update = payload.into_inner();
    match tax_service
        .update_tax_rate(id, admin_user.0.tenant_id, update)
        .await
    {
        Ok(rate) => Ok(HttpResponse::Ok().json(TaxRateResponse::from(rate))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get the effective tax rate for a type and date
#[utoipa::path(
    get, path = "/api/v1/tax/rates/effective", tag = "Tax",
    params(EffectiveRateQuery),
    responses((status = 200, description = "Effective tax rate", body = TaxRateResponse), (status = 404, description = "No effective rate found")),
    security(("bearer_auth" = []))
)]
pub async fn get_effective_rate(
    auth_user: AuthUser,
    tax_service: web::Data<TaxService>,
    query: web::Query<EffectiveRateQuery>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let params = query.into_inner();
    let tax_type = match params.tax_type.parse::<TaxType>() {
        Ok(tt) => tt,
        Err(e) => {
            return Ok(
                crate::error::ApiError::Validation(e).to_http_response(i18n, locale.as_str())
            );
        }
    };
    match tax_service
        .get_effective_rate(tax_type, params.date, auth_user.0.tenant_id)
        .await
    {
        Ok(rate) => Ok(HttpResponse::Ok().json(TaxRateResponse::from(rate))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Calculate tax on an amount
#[utoipa::path(
    post, path = "/api/v1/tax/calculate", tag = "Tax",
    request_body = CalculateTaxRequest,
    responses((status = 200, description = "Tax calculation result", body = TaxCalculationResult), (status = 404, description = "No effective rate found for date")),
    security(("bearer_auth" = []))
)]
pub async fn calculate_tax(
    auth_user: AuthUser,
    tax_service: web::Data<TaxService>,
    payload: web::Json<CalculateTaxRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let req = payload.into_inner();
    let tax_type = match req.tax_type.parse::<TaxType>() {
        Ok(tt) => tt,
        Err(e) => {
            return Ok(
                crate::error::ApiError::Validation(e).to_http_response(i18n, locale.as_str())
            );
        }
    };
    match tax_service
        .calculate_tax(
            tax_type,
            req.amount,
            req.date,
            auth_user.0.tenant_id,
            req.inclusive.unwrap_or(false),
        )
        .await
    {
        Ok(result) => Ok(HttpResponse::Ok().json(result)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Calculate taxes for an invoice
#[utoipa::path(
    post, path = "/api/v1/tax/calculate-invoice", tag = "Tax",
    request_body = CalculateInvoiceTaxRequest,
    responses((status = 200, description = "Invoice tax calculation result"), (status = 404, description = "Invoice not found")),
    security(("bearer_auth" = []))
)]
pub async fn calculate_invoice_tax(
    _auth_user: AuthUser,
    _tax_service: web::Data<TaxService>,
    _payload: web::Json<CalculateInvoiceTaxRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let msg = i18n.t(locale.as_str(), "tax.invoice_calculation_not_implemented");
    Ok(HttpResponse::NotImplemented().json(MessageResponse { message: msg }))
}

/// Soft delete a tax rate (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/tax/rates/{id}", tag = "Tax",
    params(("id" = i64, Path, description = "Tax rate ID")),
    responses((status = 204, description = "Tax rate soft deleted"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn delete_tax_rate(
    admin_user: AdminUser,
    tax_service: web::Data<TaxService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    let deleted_by = admin_user.0.user_id()?;
    match tax_service
        .delete_tax_rate(id, admin_user.0.tenant_id, deleted_by)
        .await
    {
        Ok(()) => Ok(HttpResponse::NoContent().finish()),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Restore a soft-deleted tax rate (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/tax/rates/{id}/restore", tag = "Tax",
    params(("id" = i64, Path, description = "Tax rate ID")),
    responses((status = 200, description = "Tax rate restored", body = TaxRateResponse), (status = 403, description = "Forbidden"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn restore_tax_rate(
    admin_user: AdminUser,
    tax_service: web::Data<TaxService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match tax_service
        .restore_tax_rate(id, admin_user.0.tenant_id)
        .await
    {
        Ok(rate) => Ok(HttpResponse::Ok().json(TaxRateResponse::from(rate))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// List soft-deleted tax rates (requires admin role)
#[utoipa::path(
    get, path = "/api/v1/tax/rates/deleted", tag = "Tax",
    responses((status = 200, description = "List of deleted tax rates", body = Vec<TaxRateResponse>), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn list_deleted_tax_rates(
    admin_user: AdminUser,
    tax_service: web::Data<TaxService>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match tax_service
        .list_deleted_tax_rates(admin_user.0.tenant_id)
        .await
    {
        Ok(rates) => {
            let responses: Vec<TaxRateResponse> =
                rates.into_iter().map(TaxRateResponse::from).collect();
            Ok(HttpResponse::Ok().json(responses))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Permanently destroy a soft-deleted tax rate (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/tax/rates/{id}/destroy", tag = "Tax",
    params(("id" = i64, Path, description = "Tax rate ID")),
    responses((status = 204, description = "Tax rate permanently deleted"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn destroy_tax_rate(
    admin_user: AdminUser,
    tax_service: web::Data<TaxService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match tax_service
        .destroy_tax_rate(id, admin_user.0.tenant_id)
        .await
    {
        Ok(()) => Ok(HttpResponse::NoContent().finish()),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Bulk restore soft-deleted tax rates (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/tax/rates/bulk-restore", tag = "Tax",
    request_body = BulkRestoreRequest,
    responses(
        (status = 200, description = "Tax rates restored", body = BulkRestoreResponse<TaxRateResponse>),
        (status = 400, description = "Bad request — empty or oversized IDs list"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn bulk_restore_tax_rates(
    admin_user: AdminUser,
    tax_service: web::Data<TaxService>,
    payload: web::Json<BulkRestoreRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let req = payload.into_inner();
    if req.ids.is_empty() {
        return Ok(
            crate::error::ApiError::BadRequest("IDs list cannot be empty".to_string())
                .to_http_response(i18n, locale.as_str()),
        );
    }
    if req.ids.len() > 100 {
        return Ok(crate::error::ApiError::BadRequest(
            "IDs list cannot exceed 100 items".to_string(),
        )
        .to_http_response(i18n, locale.as_str()));
    }
    match tax_service
        .bulk_restore_tax_rates(req.ids, admin_user.0.tenant_id)
        .await
    {
        Ok((restored_rates, failed_tuples)) => {
            let items: Vec<TaxRateResponse> = restored_rates
                .into_iter()
                .map(TaxRateResponse::from)
                .collect();
            let failed: Vec<BulkRestoreFailed> = failed_tuples
                .into_iter()
                .map(|(id, reason)| BulkRestoreFailed { id, reason })
                .collect();
            Ok(HttpResponse::Ok().json(BulkRestoreResponse {
                restored: items.len(),
                items,
                failed,
            }))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}
