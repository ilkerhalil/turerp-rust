//! Project API endpoints (v1)

use actix_web::{web, HttpResponse};

use crate::common::pagination::PaginationParams;
use crate::common::MessageResponse;
use crate::domain::project::model::{CreateProject, CreateWbsItem, ProjectStatus};
use crate::domain::project::service::ProjectService;
use crate::error::{ApiError, ApiResult};
use crate::i18n::{resolve, I18n, Locale};
use crate::json_resp;
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
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    json_resp!(
        project_service.create_project(create),
        HttpResponse::Created,
        i18n,
        locale.as_str()
    )
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
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        project_service.get_projects_paginated(
            auth_user.0.tenant_id,
            pagination.page,
            pagination.per_page
        ),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Get project by ID
#[utoipa::path(
    get, path = "/api/v1/projects/{id}", tag = "Project",
    params(("id" = i64, Path, description = "Project ID")),
    responses((status = 200, description = "Project found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_project(
    auth_user: AuthUser,
    project_service: web::Data<ProjectService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        project_service.get_project(*path, auth_user.0.tenant_id),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
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
    admin_user: AdminUser,
    project_service: web::Data<ProjectService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateProjectStatusRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        project_service.update_project_status(
            *path,
            admin_user.0.tenant_id,
            payload.into_inner().status
        ),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Create WBS item (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/projects/wbs", tag = "Project",
    request_body = CreateWbsItem,
    responses((status = 201, description = "WBS item created"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_wbs_item(
    admin_user: AdminUser,
    project_service: web::Data<ProjectService>,
    payload: web::Json<CreateWbsItem>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let tenant_id = admin_user.0.tenant_id;
    json_resp!(
        project_service.create_wbs_item(tenant_id, payload.into_inner()),
        HttpResponse::Created,
        i18n,
        locale.as_str()
    )
}

/// Get WBS items by project
#[utoipa::path(
    get, path = "/api/v1/projects/{project_id}/wbs", tag = "Project",
    params(("project_id" = i64, Path, description = "Project ID")),
    responses((status = 200, description = "WBS items for project")),
    security(("bearer_auth" = []))
)]
pub async fn get_wbs_by_project(
    auth_user: AuthUser,
    project_service: web::Data<ProjectService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        project_service.get_wbs_by_project(*path, auth_user.0.tenant_id),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
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
    admin_user: AdminUser,
    project_service: web::Data<ProjectService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateWbsProgressRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let req = payload.into_inner();
    json_resp!(
        project_service.update_wbs_progress(*path, admin_user.0.tenant_id, req.progress, req.hours),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Create project cost (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/projects/costs", tag = "Project",
    request_body = crate::domain::project::model::CreateProjectCost,
    responses((status = 201, description = "Project cost created"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_project_cost(
    admin_user: AdminUser,
    project_service: web::Data<ProjectService>,
    payload: web::Json<crate::domain::project::model::CreateProjectCost>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        project_service.create_project_cost(admin_user.0.tenant_id, payload.into_inner()),
        HttpResponse::Created,
        i18n,
        locale.as_str()
    )
}

/// Get project costs
#[utoipa::path(
    get, path = "/api/v1/projects/{project_id}/costs", tag = "Project",
    params(("project_id" = i64, Path, description = "Project ID")),
    responses((status = 200, description = "Project costs")),
    security(("bearer_auth" = []))
)]
pub async fn get_project_costs(
    auth_user: AuthUser,
    project_service: web::Data<ProjectService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        project_service.get_project_costs(*path, auth_user.0.tenant_id),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Get project profitability
#[utoipa::path(
    get, path = "/api/v1/projects/{project_id}/profitability", tag = "Project",
    params(("project_id" = i64, Path, description = "Project ID"), ("revenue" = String, Query, description = "Revenue amount")),
    responses((status = 200, description = "Project profitability")),
    security(("bearer_auth" = []))
)]
pub async fn get_profitability(
    auth_user: AuthUser,
    project_service: web::Data<ProjectService>,
    path: web::Path<i64>,
    query: web::Query<ProfitabilityQuery>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let revenue: rust_decimal::Decimal = query.revenue.parse().map_err(|_| {
        let msg = i18n.t(locale.as_str(), "generic.validation_error");
        ApiError::Validation(msg)
    })?;
    json_resp!(
        project_service.get_profitability(*path, auth_user.0.tenant_id, revenue),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
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

/// Soft delete a project (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/projects/{id}", tag = "Project",
    params(("id" = i64, Path, description = "Project ID")),
    responses((status = 200, description = "Project soft deleted", body = MessageResponse), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn soft_delete_project(
    admin_user: AdminUser,
    project_service: web::Data<ProjectService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let user_id: i64 = admin_user.0.user_id()?;
    match project_service
        .soft_delete_project(*path, admin_user.0.tenant_id, user_id)
        .await
    {
        Ok(()) => {
            let msg = i18n.t(locale.as_str(), "project.deleted");
            Ok(HttpResponse::Ok().json(MessageResponse { message: msg }))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Restore a soft-deleted project (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/projects/{id}/restore", tag = "Project",
    params(("id" = i64, Path, description = "Project ID")),
    responses((status = 200, description = "Project restored"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn restore_project(
    admin_user: AdminUser,
    project_service: web::Data<ProjectService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        project_service.restore_project(*path, admin_user.0.tenant_id),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// List soft-deleted projects (requires admin role)
#[utoipa::path(
    get, path = "/api/v1/projects/deleted", tag = "Project",
    responses((status = 200, description = "List of deleted projects")),
    security(("bearer_auth" = []))
)]
pub async fn list_deleted_projects(
    admin_user: AdminUser,
    project_service: web::Data<ProjectService>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        project_service.list_deleted_projects(admin_user.0.tenant_id),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Hard delete (destroy) a project (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/projects/{id}/destroy", tag = "Project",
    params(("id" = i64, Path, description = "Project ID")),
    responses((status = 204, description = "Project permanently deleted"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn destroy_project(
    admin_user: AdminUser,
    project_service: web::Data<ProjectService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    project_service
        .destroy_project(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

/// Configure project routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/v1/projects")
            .route(web::get().to(get_projects))
            .route(web::post().to(create_project)),
    )
    .service(web::resource("/v1/projects/deleted").route(web::get().to(list_deleted_projects)))
    .service(
        web::resource("/v1/projects/{id}")
            .route(web::get().to(get_project))
            .route(web::delete().to(soft_delete_project)),
    )
    .service(web::resource("/v1/projects/{id}/status").route(web::put().to(update_project_status)))
    .service(web::resource("/v1/projects/{id}/restore").route(web::put().to(restore_project)))
    .service(web::resource("/v1/projects/{id}/destroy").route(web::delete().to(destroy_project)))
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
