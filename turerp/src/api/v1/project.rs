//! Project API endpoints (v1)

use actix_web::{web, HttpResponse};

use crate::common::pagination::PaginationParams;
use crate::domain::project::model::{CreateProject, CreateWbsItem, ProjectStatus};
use crate::domain::project::service::ProjectService;
use crate::error::ApiResult;
use crate::middleware::{AdminUser, AuthUser};

/// Create project (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/projects", tag = "Project",
    request_body = CreateProject,
    responses((status = 201, description = "Project created"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_project(
    admin_user: AdminUser,
    project_service: web::Data<ProjectService>,
    payload: web::Json<CreateProject>,
) -> ApiResult<HttpResponse> {
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    let project = project_service.create_project(create).await?;
    Ok(HttpResponse::Created().json(project))
}

/// Get all projects
#[utoipa::path(
    get, path = "/api/v1/projects", tag = "Project",
    params(PaginationParams),
    responses((status = 200, description = "List of projects")),
    security(("bearer_auth" = []))
)]
pub async fn get_projects(
    auth_user: AuthUser,
    project_service: web::Data<ProjectService>,
    pagination: web::Query<PaginationParams>,
) -> ApiResult<HttpResponse> {
    let result = project_service
        .get_projects_paginated(auth_user.0.tenant_id, pagination.page, pagination.per_page)
        .await?;
    Ok(HttpResponse::Ok().json(result))
}

/// Get project by ID
#[utoipa::path(
    get, path = "/api/v1/projects/{id}", tag = "Project",
    params(("id" = i64, Path, description = "Project ID")),
    responses((status = 200, description = "Project found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_project(
    _auth_user: AuthUser,
    project_service: web::Data<ProjectService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let project = project_service.get_project(*path).await?;
    Ok(HttpResponse::Ok().json(project))
}

/// Update project status (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/projects/{id}/status", tag = "Project",
    params(("id" = i64, Path, description = "Project ID")),
    request_body = UpdateProjectStatusRequest,
    responses((status = 200, description = "Status updated"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn update_project_status(
    _admin_user: AdminUser,
    project_service: web::Data<ProjectService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateProjectStatusRequest>,
) -> ApiResult<HttpResponse> {
    let project = project_service
        .update_project_status(*path, payload.into_inner().status)
        .await?;
    Ok(HttpResponse::Ok().json(project))
}

/// Create WBS item (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/projects/wbs", tag = "Project",
    request_body = CreateWbsItem,
    responses((status = 201, description = "WBS item created"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_wbs_item(
    _admin_user: AdminUser,
    project_service: web::Data<ProjectService>,
    payload: web::Json<CreateWbsItem>,
) -> ApiResult<HttpResponse> {
    let wbs = project_service
        .create_wbs_item(payload.into_inner())
        .await?;
    Ok(HttpResponse::Created().json(wbs))
}

/// Get WBS items by project
#[utoipa::path(
    get, path = "/api/v1/projects/{project_id}/wbs", tag = "Project",
    params(("project_id" = i64, Path, description = "Project ID")),
    responses((status = 200, description = "WBS items for project")),
    security(("bearer_auth" = []))
)]
pub async fn get_wbs_by_project(
    _auth_user: AuthUser,
    project_service: web::Data<ProjectService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let wbs = project_service.get_wbs_by_project(*path).await?;
    Ok(HttpResponse::Ok().json(wbs))
}

/// Update WBS progress (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/projects/wbs/{id}/progress", tag = "Project",
    params(("id" = i64, Path, description = "WBS item ID")),
    request_body = UpdateWbsProgressRequest,
    responses((status = 200, description = "Progress updated"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn update_wbs_progress(
    _admin_user: AdminUser,
    project_service: web::Data<ProjectService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateWbsProgressRequest>,
) -> ApiResult<HttpResponse> {
    let req = payload.into_inner();
    let wbs = project_service
        .update_wbs_progress(*path, req.progress, req.hours)
        .await?;
    Ok(HttpResponse::Ok().json(wbs))
}

/// Create project cost (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/projects/costs", tag = "Project",
    request_body = crate::domain::project::model::CreateProjectCost,
    responses((status = 201, description = "Project cost created"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_project_cost(
    _admin_user: AdminUser,
    project_service: web::Data<ProjectService>,
    payload: web::Json<crate::domain::project::model::CreateProjectCost>,
) -> ApiResult<HttpResponse> {
    let cost = project_service
        .create_project_cost(payload.into_inner())
        .await?;
    Ok(HttpResponse::Created().json(cost))
}

/// Get project costs
#[utoipa::path(
    get, path = "/api/v1/projects/{project_id}/costs", tag = "Project",
    params(("project_id" = i64, Path, description = "Project ID")),
    responses((status = 200, description = "Project costs")),
    security(("bearer_auth" = []))
)]
pub async fn get_project_costs(
    _auth_user: AuthUser,
    project_service: web::Data<ProjectService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let costs = project_service.get_project_costs(*path).await?;
    Ok(HttpResponse::Ok().json(costs))
}

/// Get project profitability
#[utoipa::path(
    get, path = "/api/v1/projects/{project_id}/profitability", tag = "Project",
    params(("project_id" = i64, Path, description = "Project ID"), ("revenue" = String, Query, description = "Revenue amount")),
    responses((status = 200, description = "Project profitability")),
    security(("bearer_auth" = []))
)]
pub async fn get_profitability(
    _auth_user: AuthUser,
    project_service: web::Data<ProjectService>,
    path: web::Path<i64>,
    query: web::Query<ProfitabilityQuery>,
) -> ApiResult<HttpResponse> {
    let revenue: rust_decimal::Decimal = query.revenue.parse().unwrap_or_default();
    let profitability = project_service.get_profitability(*path, revenue).await?;
    Ok(HttpResponse::Ok().json(profitability))
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct UpdateProjectStatusRequest {
    pub status: ProjectStatus,
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct UpdateWbsProgressRequest {
    pub progress: rust_decimal::Decimal,
    pub hours: rust_decimal::Decimal,
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct ProfitabilityQuery {
    pub revenue: String,
}

/// Configure project routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/v1/projects")
            .route(web::get().to(get_projects))
            .route(web::post().to(create_project)),
    )
    .service(web::resource("/v1/projects/{id}").route(web::get().to(get_project)))
    .service(web::resource("/v1/projects/{id}/status").route(web::put().to(update_project_status)))
    .service(
        web::resource("/v1/projects/{project_id}/wbs").route(web::get().to(get_wbs_by_project)),
    )
    .service(
        web::resource("/v1/projects/{project_id}/costs").route(web::get().to(get_project_costs)),
    )
    .service(
        web::resource("/v1/projects/{project_id}/profitability")
            .route(web::get().to(get_profitability)),
    )
    .service(web::resource("/v1/projects/wbs").route(web::post().to(create_wbs_item)))
    .service(
        web::resource("/v1/projects/wbs/{id}/progress").route(web::put().to(update_wbs_progress)),
    )
    .service(web::resource("/v1/projects/costs").route(web::post().to(create_project_cost)));
}
