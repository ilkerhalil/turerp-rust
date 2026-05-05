//! Cari (Customer/Vendor) API endpoints (v1)

use actix_web::{web, HttpResponse};

use crate::common::pagination::PaginationParams;
use crate::common::MessageResponse;
use crate::domain::cari::model::{CariResponse, CariType, CreateCari, UpdateCari};
use crate::domain::cari::service::CariService;
use crate::error::{ApiError, ApiResult};
use crate::i18n::{resolve, I18n, Locale};
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
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    match cari_service.create_cari(create).await {
        Ok(cari) => Ok(HttpResponse::Created().json(cari)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
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
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match cari_service.get_cari(*path, auth_user.0.tenant_id).await {
        Ok(cari) => Ok(HttpResponse::Ok().json(cari)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
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
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    if let Err(e) = pagination.validate() {
        let err = ApiError::Validation(e.to_string());
        return Ok(err.to_http_response(i18n, locale.as_str()));
    }
    match cari_service
        .get_all_cari_paginated(auth_user.0.tenant_id, pagination.page, pagination.per_page)
        .await
    {
        Ok(result) => Ok(HttpResponse::Ok().json(result)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
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
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    if let Err(e) = pagination.validate() {
        let err = ApiError::Validation(e.to_string());
        return Ok(err.to_http_response(i18n, locale.as_str()));
    }
    match cari_service
        .get_cari_by_type_paginated(
            path.into_inner(),
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
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    if let Err(e) = query.validate() {
        let err = ApiError::Validation(e);
        return Ok(err.to_http_response(i18n, locale.as_str()));
    }
    match cari_service
        .search_cari_paginated(&query.q, auth_user.0.tenant_id, query.page, query.per_page)
        .await
    {
        Ok(result) => Ok(HttpResponse::Ok().json(result)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
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
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match cari_service
        .update_cari(*path, admin_user.0.tenant_id, payload.into_inner())
        .await
    {
        Ok(cari) => Ok(HttpResponse::Ok().json(cari)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Delete cari (requires admin role)
#[utoipa::path(
    delete,
    path = "/api/v1/cari/{id}",
    tag = "Cari",
    params(("id" = i64, Path, description = "Cari ID")),
    responses(
        (status = 200, description = "Cari deleted", body = MessageResponse),
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
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match cari_service
        .delete_cari(*path, admin_user.0.tenant_id, admin_user.0.user_id()?)
        .await
    {
        Ok(()) => {
            let msg = i18n.t(locale.as_str(), "cari.deleted");
            Ok(HttpResponse::Ok().json(MessageResponse { message: msg }))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Restore a soft-deleted cari (admin only)
#[utoipa::path(
    put,
    path = "/api/v1/cari/{id}/restore",
    tag = "Cari",
    params(("id" = i64, Path, description = "Cari ID")),
    responses(
        (status = 200, description = "Cari restored", body = CariResponse),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 404, description = "Cari not found or not deleted")
    ),
    security(("bearer_auth" = []))
)]
pub async fn restore_cari(
    admin_user: AdminUser,
    cari_service: web::Data<CariService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let cari = cari_service
        .restore_cari(*path, admin_user.0.tenant_id)
        .await?;
    let response: CariResponse = cari.into();
    Ok(HttpResponse::Ok().json(response))
}

/// List soft-deleted cari accounts (admin only)
#[utoipa::path(
    get,
    path = "/api/v1/cari/deleted",
    tag = "Cari",
    responses(
        (status = 200, description = "List of deleted cari accounts"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required")
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_deleted_cari(
    admin_user: AdminUser,
    cari_service: web::Data<CariService>,
) -> ApiResult<HttpResponse> {
    let caris: Vec<_> = cari_service
        .list_deleted_cari(admin_user.0.tenant_id)
        .await?
        .into_iter()
        .map(CariResponse::from)
        .collect();
    Ok(HttpResponse::Ok().json(caris))
}

/// Permanently delete a cari (admin only)
#[utoipa::path(
    delete,
    path = "/api/v1/cari/{id}/destroy",
    tag = "Cari",
    params(("id" = i64, Path, description = "Cari ID")),
    responses(
        (status = 204, description = "Cari permanently deleted"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 404, description = "Cari not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn destroy_cari(
    admin_user: AdminUser,
    cari_service: web::Data<CariService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    cari_service
        .destroy_cari(*path, admin_user.0.tenant_id)
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
    .service(web::resource("/v1/cari/deleted").route(web::get().to(list_deleted_cari)))
    .service(web::resource("/v1/cari/search").route(web::get().to(search_cari)))
    .service(web::resource("/v1/cari/type/{cari_type}").route(web::get().to(get_cari_by_type)))
    .service(
        web::resource("/v1/cari/{id}")
            .route(web::get().to(get_cari))
            .route(web::put().to(update_cari))
            .route(web::delete().to(delete_cari)),
    )
    .service(web::resource("/v1/cari/{id}/restore").route(web::put().to(restore_cari)))
    .service(web::resource("/v1/cari/{id}/destroy").route(web::delete().to(destroy_cari)));
}
