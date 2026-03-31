//! Feature Flags API endpoints (v1)

use actix_web::{web, HttpResponse};

use crate::domain::feature::{CreateFeatureFlag, FeatureFlagService, UpdateFeatureFlag};
use crate::error::ApiResult;
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
) -> ApiResult<HttpResponse> {
    let flag = feature_service.create(payload.into_inner()).await?;
    Ok(HttpResponse::Created().json(flag))
}

/// Get all feature flags endpoint (authenticated)
#[utoipa::path(
    get,
    path = "/api/v1/feature-flags",
    tag = "Feature Flags",
    responses(
        (status = 200, description = "Feature flags retrieved", body = Vec<FeatureFlagResponse>),
        (status = 401, description = "Not authenticated - missing or invalid JWT token")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_flags(
    auth_user: AuthUser,
    feature_service: web::Data<FeatureFlagService>,
    _query: web::Query<GetFlagsParams>,
) -> ApiResult<HttpResponse> {
    // Always use authenticated user's tenant for isolation
    // Regular users can only see their own tenant's feature flags
    let tenant_id = Some(auth_user.0.tenant_id);
    let flags = feature_service.get_all(tenant_id).await?;
    Ok(HttpResponse::Ok().json(flags))
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
) -> ApiResult<HttpResponse> {
    let flag = feature_service.get_by_id(*path).await?.ok_or_else(|| {
        crate::error::ApiError::NotFound(format!("Feature flag with id {} not found", path))
    })?;
    Ok(HttpResponse::Ok().json(flag))
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
) -> ApiResult<HttpResponse> {
    let flag = feature_service
        .update(*path, payload.into_inner())
        .await?
        .ok_or_else(|| {
            crate::error::ApiError::NotFound(format!("Feature flag with id {} not found", path))
        })?;
    Ok(HttpResponse::Ok().json(flag))
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
        (status = 204, description = "Feature flag deleted"),
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
) -> ApiResult<HttpResponse> {
    feature_service.delete(*path).await?;
    Ok(HttpResponse::NoContent().finish())
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
) -> ApiResult<HttpResponse> {
    let flag = feature_service.enable(*path).await?.ok_or_else(|| {
        crate::error::ApiError::NotFound(format!("Feature flag with id {} not found", path))
    })?;
    Ok(HttpResponse::Ok().json(flag))
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
) -> ApiResult<HttpResponse> {
    let flag = feature_service.disable(*path).await?.ok_or_else(|| {
        crate::error::ApiError::NotFound(format!("Feature flag with id {} not found", path))
    })?;
    Ok(HttpResponse::Ok().json(flag))
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
) -> ApiResult<HttpResponse> {
    let name = path.into_inner();
    let tenant_id = Some(auth_user.0.tenant_id);
    let enabled = feature_service.is_enabled(&name, tenant_id).await?;
    Ok(HttpResponse::Ok().json(FeatureStatusResponse {
        name,
        enabled,
        tenant_id: auth_user.0.tenant_id,
    }))
}

/// Query parameters for listing feature flags
#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct GetFlagsParams {
    /// Filter by tenant ID
    pub tenant_id: Option<i64>,
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
