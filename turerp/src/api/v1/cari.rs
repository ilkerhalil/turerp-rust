//! Cari (Customer/Vendor) API endpoints (v1)

use actix_web::{web, HttpResponse};

use crate::common::pagination::PaginationParams;
use crate::domain::cari::model::{CariType, CreateCari, UpdateCari};
use crate::domain::cari::service::CariService;
use crate::error::ApiResult;
use crate::middleware::{AdminUser, AuthUser};

/// Create a cari (requires admin role)
#[utoipa::path(
    post,
    path = "/api/v1/cari",
    tag = "Cari",
    request_body = CreateCari,
    responses(
        (status = 201, description = "Cari created successfully"),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required")
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_cari(
    admin_user: AdminUser,
    cari_service: web::Data<CariService>,
    payload: web::Json<CreateCari>,
) -> ApiResult<HttpResponse> {
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    let cari = cari_service.create_cari(create).await?;
    Ok(HttpResponse::Created().json(cari))
}

/// Get cari by ID (requires authentication)
#[utoipa::path(
    get,
    path = "/api/v1/cari/{id}",
    tag = "Cari",
    params(("id" = i64, Path, description = "Cari ID")),
    responses(
        (status = 200, description = "Cari found"),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "Cari not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_cari(
    auth_user: AuthUser,
    cari_service: web::Data<CariService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let cari = cari_service.get_cari(*path, auth_user.0.tenant_id).await?;
    Ok(HttpResponse::Ok().json(cari))
}

/// Get all cari (requires authentication)
#[utoipa::path(
    get,
    path = "/api/v1/cari",
    tag = "Cari",
    params(PaginationParams),
    responses(
        (status = 200, description = "Paginated list of cari"),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_all_cari(
    auth_user: AuthUser,
    cari_service: web::Data<CariService>,
    pagination: web::Query<PaginationParams>,
) -> ApiResult<HttpResponse> {
    pagination
        .validate()
        .map_err(crate::error::ApiError::Validation)?;
    let result = cari_service
        .get_all_cari_paginated(auth_user.0.tenant_id, pagination.page, pagination.per_page)
        .await?;
    Ok(HttpResponse::Ok().json(result))
}

/// Get cari by type (requires authentication)
#[utoipa::path(
    get,
    path = "/api/v1/cari/type/{cari_type}",
    tag = "Cari",
    params(("cari_type" = CariType, Path, description = "Cari type filter"), PaginationParams),
    responses(
        (status = 200, description = "Paginated list of cari by type"),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_cari_by_type(
    auth_user: AuthUser,
    cari_service: web::Data<CariService>,
    path: web::Path<CariType>,
    pagination: web::Query<PaginationParams>,
) -> ApiResult<HttpResponse> {
    pagination
        .validate()
        .map_err(crate::error::ApiError::Validation)?;
    let result = cari_service
        .get_cari_by_type_paginated(
            path.into_inner(),
            auth_user.0.tenant_id,
            pagination.page,
            pagination.per_page,
        )
        .await?;
    Ok(HttpResponse::Ok().json(result))
}

/// Search cari (requires authentication)
#[utoipa::path(
    get,
    path = "/api/v1/cari/search",
    tag = "Cari",
    params(("q" = String, Query, description = "Search query"), PaginationParams),
    responses(
        (status = 200, description = "Paginated search results"),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn search_cari(
    auth_user: AuthUser,
    cari_service: web::Data<CariService>,
    query: web::Query<SearchQuery>,
) -> ApiResult<HttpResponse> {
    query
        .validate()
        .map_err(crate::error::ApiError::Validation)?;
    let result = cari_service
        .search_cari_paginated(&query.q, auth_user.0.tenant_id, query.page, query.per_page)
        .await?;
    Ok(HttpResponse::Ok().json(result))
}

/// Update cari (requires admin role)
#[utoipa::path(
    put,
    path = "/api/v1/cari/{id}",
    tag = "Cari",
    params(("id" = i64, Path, description = "Cari ID")),
    request_body = UpdateCari,
    responses(
        (status = 200, description = "Cari updated"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 404, description = "Cari not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_cari(
    admin_user: AdminUser,
    cari_service: web::Data<CariService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateCari>,
) -> ApiResult<HttpResponse> {
    let cari = cari_service
        .update_cari(*path, admin_user.0.tenant_id, payload.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(cari))
}

/// Delete cari (requires admin role)
#[utoipa::path(
    delete,
    path = "/api/v1/cari/{id}",
    tag = "Cari",
    params(("id" = i64, Path, description = "Cari ID")),
    responses(
        (status = 204, description = "Cari deleted"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 404, description = "Cari not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_cari(
    admin_user: AdminUser,
    cari_service: web::Data<CariService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    cari_service
        .delete_cari(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct SearchQuery {
    pub q: String,
    #[serde(default = "crate::common::pagination::default_page")]
    pub page: u32,
    #[serde(default = "crate::common::pagination::default_per_page")]
    pub per_page: u32,
}

impl SearchQuery {
    /// Validate pagination parameters
    pub fn validate(&self) -> Result<(), String> {
        if self.page == 0 {
            return Err("page must be at least 1".to_string());
        }
        if self.per_page == 0 || self.per_page > 100 {
            return Err("per_page must be between 1 and 100".to_string());
        }
        Ok(())
    }
}

/// Configure cari routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/v1/cari")
            .route(web::get().to(get_all_cari))
            .route(web::post().to(create_cari)),
    )
    .service(web::resource("/v1/cari/search").route(web::get().to(search_cari)))
    .service(web::resource("/v1/cari/type/{cari_type}").route(web::get().to(get_cari_by_type)))
    .service(
        web::resource("/v1/cari/{id}")
            .route(web::get().to(get_cari))
            .route(web::put().to(update_cari))
            .route(web::delete().to(delete_cari)),
    );
}
