//! Cari (Customer/Vendor) API endpoints (v1)

use actix_web::{web, HttpResponse};

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
    responses(
        (status = 200, description = "List of cari"),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_all_cari(
    auth_user: AuthUser,
    cari_service: web::Data<CariService>,
) -> ApiResult<HttpResponse> {
    let caris = cari_service.get_all_cari(auth_user.0.tenant_id).await?;
    Ok(HttpResponse::Ok().json(caris))
}

/// Get cari by type (requires authentication)
#[utoipa::path(
    get,
    path = "/api/v1/cari/type/{cari_type}",
    tag = "Cari",
    params(("cari_type" = CariType, Path, description = "Cari type filter")),
    responses(
        (status = 200, description = "List of cari by type"),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_cari_by_type(
    auth_user: AuthUser,
    cari_service: web::Data<CariService>,
    path: web::Path<CariType>,
) -> ApiResult<HttpResponse> {
    let caris = cari_service
        .get_cari_by_type(path.into_inner(), auth_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(caris))
}

/// Search cari (requires authentication)
#[utoipa::path(
    get,
    path = "/api/v1/cari/search",
    tag = "Cari",
    params(("q" = String, Query, description = "Search query")),
    responses(
        (status = 200, description = "Search results"),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn search_cari(
    auth_user: AuthUser,
    cari_service: web::Data<CariService>,
    query: web::Query<SearchQuery>,
) -> ApiResult<HttpResponse> {
    let caris = cari_service
        .search_cari(&query.q, auth_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(caris))
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
