//! Cost Center / Profit Center API endpoints (v1)

use actix_web::{web, HttpResponse};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::common::pagination::{default_page, default_per_page, PaginationParams};
use crate::domain::cost_center::model::{
    CostCenterResponse, CostCenterType, CreateAllocation, CreateCostCenter, UpdateCostCenter,
};
use crate::domain::cost_center::service::CostCenterService;
use crate::error::ApiResult;
use crate::i18n::{resolve, I18n, Locale};
use crate::middleware::{AdminUser, AuthUser};

/// Query parameters for listing cost centers
#[derive(Debug, Deserialize)]
pub struct ListCostCentersQuery {
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_per_page")]
    pub per_page: u32,
    pub center_type: Option<String>,
}

impl From<ListCostCentersQuery> for PaginationParams {
    fn from(q: ListCostCentersQuery) -> Self {
        Self {
            page: q.page,
            per_page: q.per_page,
        }
    }
}

/// Query parameters for profitability report
#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct ProfitabilityQuery {
    pub period_start: Option<DateTime<Utc>>,
    pub period_end: Option<DateTime<Utc>>,
}

/// Request body for bulk restore operations
#[derive(Debug, Deserialize, ToSchema)]
pub struct BulkRestoreRequest {
    pub ids: Vec<i64>,
}

/// Create a cost center (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/cost-centers", tag = "Cost Centers",
    request_body = CreateCostCenter,
    responses((status = 201, description = "Cost center created", body = CostCenterResponse), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_cost_center(
    admin_user: AdminUser,
    cost_center_service: web::Data<CostCenterService>,
    payload: web::Json<CreateCostCenter>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let create = payload.into_inner();
    match cost_center_service
        .create_cost_center(create, admin_user.0.tenant_id)
        .await
    {
        Ok(center) => Ok(HttpResponse::Created().json(CostCenterResponse::from(center))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// List cost centers (paginated, optional type filter)
#[utoipa::path(
    get, path = "/api/v1/cost-centers", tag = "Cost Centers",
    params(
        PaginationParams,
        ("center_type" = Option<String>, Query, description = "Filter by type (cost, profit)"),
    ),
    responses((status = 200, description = "List of cost centers")),
    security(("bearer_auth" = []))
)]
pub async fn list_cost_centers(
    auth_user: AuthUser,
    cost_center_service: web::Data<CostCenterService>,
    query: web::Query<ListCostCentersQuery>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let q = query.into_inner();
    let type_filter = q
        .center_type
        .clone()
        .and_then(|s| s.parse::<CostCenterType>().ok());
    match cost_center_service
        .list_cost_centers(auth_user.0.tenant_id, type_filter, q.into())
        .await
    {
        Ok(result) => {
            let mapped = result.map(CostCenterResponse::from);
            Ok(HttpResponse::Ok().json(mapped))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get a cost center by ID
#[utoipa::path(
    get, path = "/api/v1/cost-centers/{id}", tag = "Cost Centers",
    params(("id" = i64, Path, description = "Cost center ID")),
    responses((status = 200, description = "Cost center found", body = CostCenterResponse), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_cost_center(
    auth_user: AuthUser,
    cost_center_service: web::Data<CostCenterService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match cost_center_service
        .get_cost_center(id, auth_user.0.tenant_id)
        .await
    {
        Ok(center) => Ok(HttpResponse::Ok().json(CostCenterResponse::from(center))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Update a cost center (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/cost-centers/{id}", tag = "Cost Centers",
    params(("id" = i64, Path, description = "Cost center ID")),
    request_body = UpdateCostCenter,
    responses((status = 200, description = "Cost center updated", body = CostCenterResponse), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn update_cost_center(
    admin_user: AdminUser,
    cost_center_service: web::Data<CostCenterService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateCostCenter>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    let update = payload.into_inner();
    match cost_center_service
        .update_cost_center(id, admin_user.0.tenant_id, update)
        .await
    {
        Ok(center) => Ok(HttpResponse::Ok().json(CostCenterResponse::from(center))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Soft delete a cost center (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/cost-centers/{id}", tag = "Cost Centers",
    params(("id" = i64, Path, description = "Cost center ID")),
    responses((status = 204, description = "Cost center soft deleted"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn delete_cost_center(
    admin_user: AdminUser,
    cost_center_service: web::Data<CostCenterService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    let deleted_by = admin_user.0.sub.parse::<i64>().unwrap_or(0);
    match cost_center_service
        .delete_cost_center(id, admin_user.0.tenant_id, deleted_by)
        .await
    {
        Ok(()) => Ok(HttpResponse::NoContent().finish()),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Restore a soft-deleted cost center (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/cost-centers/{id}/restore", tag = "Cost Centers",
    params(("id" = i64, Path, description = "Cost center ID")),
    responses((status = 200, description = "Cost center restored", body = CostCenterResponse), (status = 403, description = "Forbidden"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn restore_cost_center(
    admin_user: AdminUser,
    cost_center_service: web::Data<CostCenterService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match cost_center_service
        .restore_cost_center(id, admin_user.0.tenant_id)
        .await
    {
        Ok(center) => Ok(HttpResponse::Ok().json(CostCenterResponse::from(center))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// List soft-deleted cost centers (requires admin role)
#[utoipa::path(
    get, path = "/api/v1/cost-centers/deleted", tag = "Cost Centers",
    responses((status = 200, description = "List of deleted cost centers", body = Vec<CostCenterResponse>), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn list_deleted_cost_centers(
    admin_user: AdminUser,
    cost_center_service: web::Data<CostCenterService>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match cost_center_service
        .list_deleted_cost_centers(admin_user.0.tenant_id)
        .await
    {
        Ok(centers) => {
            let responses: Vec<CostCenterResponse> =
                centers.into_iter().map(CostCenterResponse::from).collect();
            Ok(HttpResponse::Ok().json(responses))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Permanently destroy a soft-deleted cost center (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/cost-centers/{id}/destroy", tag = "Cost Centers",
    params(("id" = i64, Path, description = "Cost center ID")),
    responses((status = 204, description = "Cost center permanently deleted"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn destroy_cost_center(
    admin_user: AdminUser,
    cost_center_service: web::Data<CostCenterService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match cost_center_service
        .destroy_cost_center(id, admin_user.0.tenant_id)
        .await
    {
        Ok(()) => Ok(HttpResponse::NoContent().finish()),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Bulk restore soft-deleted cost centers (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/cost-centers/bulk-restore", tag = "Cost Centers",
    request_body = BulkRestoreRequest,
    responses(
        (status = 200, description = "Cost centers restored", body = crate::domain::cost_center::model::BulkRestoreResponse<CostCenterResponse>),
        (status = 400, description = "Bad request — empty or oversized IDs list"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn bulk_restore_cost_centers(
    admin_user: AdminUser,
    cost_center_service: web::Data<CostCenterService>,
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
    match cost_center_service
        .bulk_restore_cost_centers(req.ids, admin_user.0.tenant_id)
        .await
    {
        Ok(response) => Ok(HttpResponse::Ok().json(response)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

// ---- Allocation Endpoints ----

/// Create an allocation for a cost center
#[utoipa::path(
    post, path = "/api/v1/cost-centers/{id}/allocations", tag = "Cost Centers",
    params(("id" = i64, Path, description = "Cost center ID")),
    request_body = CreateAllocation,
    responses((status = 201, description = "Allocation created", body = crate::domain::cost_center::model::AllocationResponse), (status = 404, description = "Cost center not found")),
    security(("bearer_auth" = []))
)]
pub async fn create_allocation(
    auth_user: AuthUser,
    cost_center_service: web::Data<CostCenterService>,
    path: web::Path<i64>,
    payload: web::Json<CreateAllocation>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let cost_center_id = path.into_inner();
    let mut allocation = payload.into_inner();
    allocation.cost_center_id = cost_center_id;
    match cost_center_service
        .create_allocation(allocation, auth_user.0.tenant_id)
        .await
    {
        Ok(alloc) => Ok(HttpResponse::Created().json(
            crate::domain::cost_center::model::AllocationResponse::from(alloc),
        )),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get allocations for a cost center
#[utoipa::path(
    get, path = "/api/v1/cost-centers/{id}/allocations", tag = "Cost Centers",
    params(("id" = i64, Path, description = "Cost center ID")),
    responses((status = 200, description = "List of allocations", body = Vec<crate::domain::cost_center::model::AllocationResponse>), (status = 404, description = "Cost center not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_allocations(
    auth_user: AuthUser,
    cost_center_service: web::Data<CostCenterService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let cost_center_id = path.into_inner();
    match cost_center_service
        .get_allocations(cost_center_id, auth_user.0.tenant_id)
        .await
    {
        Ok(allocs) => {
            let responses: Vec<crate::domain::cost_center::model::AllocationResponse> = allocs
                .into_iter()
                .map(crate::domain::cost_center::model::AllocationResponse::from)
                .collect();
            Ok(HttpResponse::Ok().json(responses))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

// ---- Profitability Report Endpoint ----

/// Get profitability report for a cost center
#[utoipa::path(
    get, path = "/api/v1/cost-centers/{id}/profitability", tag = "Cost Centers",
    params(
        ("id" = i64, Path, description = "Cost center ID"),
        ProfitabilityQuery,
    ),
    responses((status = 200, description = "Profitability report", body = crate::domain::cost_center::model::ProfitabilityReport), (status = 404, description = "Cost center not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_profitability(
    auth_user: AuthUser,
    cost_center_service: web::Data<CostCenterService>,
    path: web::Path<i64>,
    query: web::Query<ProfitabilityQuery>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    let q = query.into_inner();
    match cost_center_service
        .get_profitability_report(id, auth_user.0.tenant_id, q.period_start, q.period_end)
        .await
    {
        Ok(report) => Ok(HttpResponse::Ok().json(report)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Configure cost center routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/v1/cost-centers")
            .route(web::get().to(list_cost_centers))
            .route(web::post().to(create_cost_center)),
    )
    .service(
        web::resource("/v1/cost-centers/deleted").route(web::get().to(list_deleted_cost_centers)),
    )
    .service(
        web::resource("/v1/cost-centers/bulk-restore")
            .route(web::post().to(bulk_restore_cost_centers)),
    )
    .service(
        web::resource("/v1/cost-centers/{id}")
            .route(web::get().to(get_cost_center))
            .route(web::put().to(update_cost_center))
            .route(web::delete().to(delete_cost_center)),
    )
    .service(
        web::resource("/v1/cost-centers/{id}/restore").route(web::put().to(restore_cost_center)),
    )
    .service(
        web::resource("/v1/cost-centers/{id}/destroy").route(web::delete().to(destroy_cost_center)),
    )
    .service(
        web::resource("/v1/cost-centers/{id}/allocations")
            .route(web::get().to(get_allocations))
            .route(web::post().to(create_allocation)),
    )
    .service(
        web::resource("/v1/cost-centers/{id}/profitability")
            .route(web::get().to(get_profitability)),
    );
}
