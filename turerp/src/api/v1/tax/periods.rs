//! Tax period handlers

use actix_web::{web, HttpResponse};
use serde::Deserialize;

use crate::common::pagination::{default_page, default_per_page, PaginationParams};
use crate::domain::tax::model::{
    BulkRestoreFailed, BulkRestoreResponse, CreateTaxPeriod, TaxPeriodResponse, TaxType,
};
use crate::domain::tax::service::TaxService;
use crate::error::ApiResult;
use crate::i18n::{resolve, I18n, Locale};
use crate::middleware::{AdminUser, AuthUser};

use super::BulkRestoreRequest;

/// Query parameters for listing tax periods
#[derive(Debug, Deserialize)]
pub struct ListTaxPeriodsQuery {
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_per_page")]
    pub per_page: u32,
    pub tax_type: Option<String>,
}

impl From<ListTaxPeriodsQuery> for PaginationParams {
    fn from(q: ListTaxPeriodsQuery) -> Self {
        Self {
            page: q.page,
            per_page: q.per_page,
        }
    }
}

/// Create a tax period (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/tax/periods", tag = "Tax",
    request_body = CreateTaxPeriod,
    responses((status = 201, description = "Tax period created", body = TaxPeriodResponse), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_tax_period(
    admin_user: AdminUser,
    tax_service: web::Data<TaxService>,
    payload: web::Json<CreateTaxPeriod>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let create = payload.into_inner();
    match tax_service
        .create_tax_period(create, admin_user.0.tenant_id)
        .await
    {
        Ok(period) => Ok(HttpResponse::Created().json(TaxPeriodResponse::from(period))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// List tax periods (paginated, optional tax_type filter)
#[utoipa::path(
    get, path = "/api/v1/tax/periods", tag = "Tax",
    params(
        PaginationParams,
        ("tax_type" = Option<String>, Query, description = "Filter by tax type"),
    ),
    responses((status = 200, description = "List of tax periods")),
    security(("bearer_auth" = []))
)]
pub async fn list_tax_periods(
    auth_user: AuthUser,
    tax_service: web::Data<TaxService>,
    query: web::Query<ListTaxPeriodsQuery>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let q = query.into_inner();
    let tax_type_filter = q.tax_type.clone().and_then(|s| s.parse::<TaxType>().ok());
    match tax_service
        .list_tax_periods(auth_user.0.tenant_id, tax_type_filter, q.into())
        .await
    {
        Ok(result) => {
            let mapped = result.map(TaxPeriodResponse::from);
            Ok(HttpResponse::Ok().json(mapped))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get a tax period by ID
#[utoipa::path(
    get, path = "/api/v1/tax/periods/{id}", tag = "Tax",
    params(("id" = i64, Path, description = "Tax period ID")),
    responses((status = 200, description = "Tax period found", body = TaxPeriodResponse), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_tax_period(
    auth_user: AuthUser,
    tax_service: web::Data<TaxService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match tax_service.get_tax_period(id, auth_user.0.tenant_id).await {
        Ok(period) => Ok(HttpResponse::Ok().json(TaxPeriodResponse::from(period))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Calculate (recalculate) a tax period (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/tax/periods/{id}/calculate", tag = "Tax",
    params(("id" = i64, Path, description = "Tax period ID")),
    responses((status = 200, description = "Tax period calculated", body = TaxPeriodResponse), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn calculate_tax_period(
    admin_user: AdminUser,
    tax_service: web::Data<TaxService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match tax_service
        .calculate_period(id, admin_user.0.tenant_id)
        .await
    {
        Ok(period) => Ok(HttpResponse::Ok().json(TaxPeriodResponse::from(period))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// File a tax period (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/tax/periods/{id}/file", tag = "Tax",
    params(("id" = i64, Path, description = "Tax period ID")),
    responses((status = 200, description = "Tax period filed", body = TaxPeriodResponse), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn file_tax_period(
    admin_user: AdminUser,
    tax_service: web::Data<TaxService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match tax_service.file_period(id, admin_user.0.tenant_id).await {
        Ok(period) => Ok(HttpResponse::Ok().json(TaxPeriodResponse::from(period))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Soft delete a tax period (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/tax/periods/{id}", tag = "Tax",
    params(("id" = i64, Path, description = "Tax period ID")),
    responses((status = 204, description = "Tax period soft deleted"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn delete_tax_period(
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
        .delete_tax_period(id, admin_user.0.tenant_id, deleted_by)
        .await
    {
        Ok(()) => Ok(HttpResponse::NoContent().finish()),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Restore a soft-deleted tax period (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/tax/periods/{id}/restore", tag = "Tax",
    params(("id" = i64, Path, description = "Tax period ID")),
    responses((status = 200, description = "Tax period restored", body = TaxPeriodResponse), (status = 403, description = "Forbidden"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn restore_tax_period(
    admin_user: AdminUser,
    tax_service: web::Data<TaxService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match tax_service
        .restore_tax_period(id, admin_user.0.tenant_id)
        .await
    {
        Ok(period) => Ok(HttpResponse::Ok().json(TaxPeriodResponse::from(period))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// List soft-deleted tax periods (requires admin role)
#[utoipa::path(
    get, path = "/api/v1/tax/periods/deleted", tag = "Tax",
    responses((status = 200, description = "List of deleted tax periods", body = Vec<TaxPeriodResponse>), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn list_deleted_tax_periods(
    admin_user: AdminUser,
    tax_service: web::Data<TaxService>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match tax_service
        .list_deleted_tax_periods(admin_user.0.tenant_id)
        .await
    {
        Ok(periods) => {
            let responses: Vec<TaxPeriodResponse> =
                periods.into_iter().map(TaxPeriodResponse::from).collect();
            Ok(HttpResponse::Ok().json(responses))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Permanently destroy a soft-deleted tax period (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/tax/periods/{id}/destroy", tag = "Tax",
    params(("id" = i64, Path, description = "Tax period ID")),
    responses((status = 204, description = "Tax period permanently deleted"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn destroy_tax_period(
    admin_user: AdminUser,
    tax_service: web::Data<TaxService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match tax_service
        .destroy_tax_period(id, admin_user.0.tenant_id)
        .await
    {
        Ok(()) => Ok(HttpResponse::NoContent().finish()),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Bulk restore soft-deleted tax periods (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/tax/periods/bulk-restore", tag = "Tax",
    request_body = BulkRestoreRequest,
    responses(
        (status = 200, description = "Tax periods restored", body = BulkRestoreResponse<TaxPeriodResponse>),
        (status = 400, description = "Bad request — empty or oversized IDs list"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn bulk_restore_tax_periods(
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
        .bulk_restore_tax_periods(req.ids, admin_user.0.tenant_id)
        .await
    {
        Ok((restored_periods, failed_tuples)) => {
            let items: Vec<TaxPeriodResponse> = restored_periods
                .into_iter()
                .map(TaxPeriodResponse::from)
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
