//! Feature Flags API endpoints (v1)

use actix_web::{web, HttpResponse};

use crate::common::pagination::PaginationParams;
use crate::common::MessageResponse;
use crate::domain::feature::{CreateFeatureFlag, FeatureFlagService, UpdateFeatureFlag};
use crate::error::{ApiError, ApiResult};
use crate::i18n::{resolve, I18n, Locale};
use crate::middleware::{AdminUser, AuthUser};

/// Create feature flag endpoint (admin only)
#[utoipa::path(
    post,
    path = "/api/v1/feature-flags",
    tag = "Feature Flags",
    request_body = CreateFeatureFlag,
    responses(
        (status = 201, description = "Feature flag created successfully", body = FeatureFlagResponse),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Not authenticated - missing or invalid JWT token"),
        (status = 403, description = "Forbidden - admin access required"),
        (status = 409, description = "Feature flag already exists")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn create_flag(
    _admin_user: AdminUser,
    feature_service: web::Data<FeatureFlagService>,
    payload: web::Json<CreateFeatureFlag>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match feature_service.create(payload.into_inner()).await {
        Ok(flag) => Ok(HttpResponse::Created().json(flag)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get all feature flags endpoint (authenticated)
#[utoipa::path(
    get,
    path = "/api/v1/feature-flags",
    tag = "Feature Flags",
    params(PaginationParams),
    responses(
        (status = 200, description = "Feature flags retrieved", body = PaginatedResult<FeatureFlagResponse>),
        (status = 401, description = "Not authenticated - missing or invalid JWT token")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_flags(
    auth_user: AuthUser,
    feature_service: web::Data<FeatureFlagService>,
    pagination: web::Query<PaginationParams>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    if let Err(e) = pagination.validate() {
        let err = ApiError::Validation(e.to_string());
        return Ok(err.to_http_response(i18n, locale.as_str()));
    }
    let tenant_id = Some(auth_user.0.tenant_id);
    match feature_service
        .get_all_paginated(tenant_id, pagination.page, pagination.per_page)
        .await
    {
        Ok(result) => Ok(HttpResponse::Ok().json(result)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get feature flag by ID endpoint (authenticated)
#[utoipa::path(
    get,
    path = "/api/v1/feature-flags/{id}",
    tag = "Feature Flags",
    params(
        ("id" = i64, Path, description = "Feature flag ID")
    ),
    responses(
        (status = 200, description = "Feature flag found", body = FeatureFlagResponse),
        (status = 401, description = "Not authenticated - missing or invalid JWT token"),
        (status = 404, description = "Feature flag not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_flag_by_id(
    _auth_user: AuthUser,
    feature_service: web::Data<FeatureFlagService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match feature_service.get_by_id(*path).await {
        Ok(Some(flag)) => Ok(HttpResponse::Ok().json(flag)),
        Ok(None) => {
            let msg = i18n.t_args(
                locale.as_str(),
                "errors.not_found",
                &[("resource", "Feature flag")],
            );
            Ok(HttpResponse::NotFound().json(crate::error::ErrorResponse { error: msg }))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Update feature flag endpoint (admin only)
#[utoipa::path(
    put,
    path = "/api/v1/feature-flags/{id}",
    tag = "Feature Flags",
    params(
        ("id" = i64, Path, description = "Feature flag ID")
    ),
    request_body = UpdateFeatureFlag,
    responses(
        (status = 200, description = "Feature flag updated", body = FeatureFlagResponse),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Not authenticated - missing or invalid JWT token"),
        (status = 403, description = "Forbidden - admin access required"),
        (status = 404, description = "Feature flag not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn update_flag(
    _admin_user: AdminUser,
    feature_service: web::Data<FeatureFlagService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateFeatureFlag>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match feature_service.update(*path, payload.into_inner()).await {
        Ok(Some(flag)) => Ok(HttpResponse::Ok().json(flag)),
        Ok(None) => {
            let msg = i18n.t_args(
                locale.as_str(),
                "errors.not_found",
                &[("resource", "Feature flag")],
            );
            Ok(HttpResponse::NotFound().json(crate::error::ErrorResponse { error: msg }))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Delete feature flag endpoint (admin only)
#[utoipa::path(
    delete,
    path = "/api/v1/feature-flags/{id}",
    tag = "Feature Flags",
    params(
        ("id" = i64, Path, description = "Feature flag ID")
    ),
    responses(
        (status = 200, description = "Feature flag deleted", body = MessageResponse),
        (status = 401, description = "Not authenticated - missing or invalid JWT token"),
        (status = 403, description = "Forbidden - admin access required"),
        (status = 404, description = "Feature flag not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn delete_flag(
    _admin_user: AdminUser,
    feature_service: web::Data<FeatureFlagService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match feature_service.delete(*path).await {
        Ok(_) => {
            let msg = i18n.t(locale.as_str(), "generic.deleted");
            Ok(HttpResponse::Ok().json(MessageResponse { message: msg }))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Enable feature flag endpoint (admin only)
#[utoipa::path(
    post,
    path = "/api/v1/feature-flags/{id}/enable",
    tag = "Feature Flags",
    params(
        ("id" = i64, Path, description = "Feature flag ID")
    ),
    responses(
        (status = 200, description = "Feature flag enabled", body = FeatureFlagResponse),
        (status = 401, description = "Not authenticated - missing or invalid JWT token"),
        (status = 403, description = "Forbidden - admin access required"),
        (status = 404, description = "Feature flag not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn enable_flag(
    _admin_user: AdminUser,
    feature_service: web::Data<FeatureFlagService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match feature_service.enable(*path).await {
        Ok(Some(flag)) => Ok(HttpResponse::Ok().json(flag)),
        Ok(None) => {
            let msg = i18n.t_args(
                locale.as_str(),
                "errors.not_found",
                &[("resource", "Feature flag")],
            );
            Ok(HttpResponse::NotFound().json(crate::error::ErrorResponse { error: msg }))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Disable feature flag endpoint (admin only)
#[utoipa::path(
    post,
    path = "/api/v1/feature-flags/{id}/disable",
    tag = "Feature Flags",
    params(
        ("id" = i64, Path, description = "Feature flag ID")
    ),
    responses(
        (status = 200, description = "Feature flag disabled", body = FeatureFlagResponse),
        (status = 401, description = "Not authenticated - missing or invalid JWT token"),
        (status = 403, description = "Forbidden - admin access required"),
        (status = 404, description = "Feature flag not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn disable_flag(
    _admin_user: AdminUser,
    feature_service: web::Data<FeatureFlagService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match feature_service.disable(*path).await {
        Ok(Some(flag)) => Ok(HttpResponse::Ok().json(flag)),
        Ok(None) => {
            let msg = i18n.t_args(
                locale.as_str(),
                "errors.not_found",
                &[("resource", "Feature flag")],
            );
            Ok(HttpResponse::NotFound().json(crate::error::ErrorResponse { error: msg }))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Check if feature is enabled endpoint (authenticated)
#[utoipa::path(
    get,
    path = "/api/v1/feature-flags/check/{name}",
    tag = "Feature Flags",
    params(
        ("name" = String, Path, description = "Feature flag name")
    ),
    responses(
        (status = 200, description = "Feature flag status", body = FeatureStatusResponse),
        (status = 401, description = "Not authenticated - missing or invalid JWT token")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn check_feature(
    auth_user: AuthUser,
    feature_service: web::Data<FeatureFlagService>,
    path: web::Path<String>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let name = path.into_inner();
    let tenant_id = Some(auth_user.0.tenant_id);
    match feature_service.is_enabled(&name, tenant_id).await {
        Ok(enabled) => Ok(HttpResponse::Ok().json(FeatureStatusResponse {
            name,
            enabled,
            tenant_id: auth_user.0.tenant_id,
        })),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Response for feature status check
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct FeatureStatusResponse {
    /// Feature flag name
    pub name: String,
    /// Whether the feature is enabled
    pub enabled: bool,
    /// Tenant ID context
    pub tenant_id: i64,
}

/// Configure feature flag routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/v1/feature-flags")
            .route(web::get().to(get_flags))
            .route(web::post().to(create_flag)),
    )
    .service(
        web::resource("/v1/feature-flags/{id}")
            .route(web::get().to(get_flag_by_id))
            .route(web::put().to(update_flag))
            .route(web::delete().to(delete_flag)),
    )
    .service(web::resource("/v1/feature-flags/{id}/enable").route(web::post().to(enable_flag)))
    .service(web::resource("/v1/feature-flags/{id}/disable").route(web::post().to(disable_flag)))
    .service(web::resource("/v1/feature-flags/check/{name}").route(web::get().to(check_feature)));
}
