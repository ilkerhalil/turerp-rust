//! Tenant API endpoints (v1)

use actix_web::{web, HttpResponse};

use crate::common::pagination::PaginationParams;
use crate::domain::tenant::model::{CreateTenant, UpdateTenant};
use crate::domain::tenant::service::TenantService;
use crate::error::ApiResult;
use crate::middleware::{AdminUser, AuthUser};

/// Create tenant (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/tenants", tag = "Tenant",
    request_body = CreateTenant,
    responses((status = 201, description = "Tenant created"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_tenant(
    _admin_user: AdminUser,
    tenant_service: web::Data<TenantService>,
    payload: web::Json<CreateTenant>,
) -> ApiResult<HttpResponse> {
    let tenant = tenant_service.create_tenant(payload.into_inner()).await?;
    Ok(HttpResponse::Created().json(tenant))
}

/// Get all tenants (requires authentication)
#[utoipa::path(
    get, path = "/api/v1/tenants", tag = "Tenant",
    params(PaginationParams),
    responses((status = 200, description = "Paginated list of tenants")),
    security(("bearer_auth" = []))
)]
pub async fn get_tenants(
    _auth_user: AuthUser,
    tenant_service: web::Data<TenantService>,
    pagination: web::Query<PaginationParams>,
) -> ApiResult<HttpResponse> {
    pagination
        .validate()
        .map_err(crate::error::ApiError::Validation)?;
    let result = tenant_service
        .get_all_tenants_paginated(pagination.page, pagination.per_page)
        .await?;
    Ok(HttpResponse::Ok().json(result))
}

/// Get tenant by ID (requires authentication)
#[utoipa::path(
    get, path = "/api/v1/tenants/{id}", tag = "Tenant",
    params(("id" = i64, Path, description = "Tenant ID")),
    responses((status = 200, description = "Tenant found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_tenant(
    _auth_user: AuthUser,
    tenant_service: web::Data<TenantService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let tenant = tenant_service.get_tenant(*path).await?;
    Ok(HttpResponse::Ok().json(tenant))
}

/// Update tenant (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/tenants/{id}", tag = "Tenant",
    params(("id" = i64, Path, description = "Tenant ID")),
    request_body = UpdateTenant,
    responses((status = 200, description = "Tenant updated"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn update_tenant(
    _admin_user: AdminUser,
    tenant_service: web::Data<TenantService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateTenant>,
) -> ApiResult<HttpResponse> {
    let tenant = tenant_service
        .update_tenant(*path, payload.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(tenant))
}

/// Delete tenant (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/tenants/{id}", tag = "Tenant",
    params(("id" = i64, Path, description = "Tenant ID")),
    responses((status = 204, description = "Tenant deleted"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn delete_tenant(
    _admin_user: AdminUser,
    tenant_service: web::Data<TenantService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    tenant_service.delete_tenant(*path).await?;
    Ok(HttpResponse::NoContent().finish())
}

use crate::domain::tenant::model::{CreateTenantConfig, UpdateTenantConfig};
use crate::domain::tenant::service::TenantConfigService;

/// Create tenant config (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/tenant-configs", tag = "Tenant Config",
    request_body = CreateTenantConfig,
    responses((status = 201, description = "Tenant config created"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_tenant_config(
    admin_user: AdminUser,
    tenant_config_service: web::Data<TenantConfigService>,
    payload: web::Json<CreateTenantConfig>,
) -> ApiResult<HttpResponse> {
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    let config = tenant_config_service.set_config(create).await?;
    Ok(HttpResponse::Created().json(config))
}

/// Get all tenant configs for current tenant
#[utoipa::path(
    get, path = "/api/v1/tenant-configs", tag = "Tenant Config",
    responses((status = 200, description = "List of tenant configs")),
    security(("bearer_auth" = []))
)]
pub async fn get_tenant_configs(
    auth_user: AuthUser,
    tenant_config_service: web::Data<TenantConfigService>,
) -> ApiResult<HttpResponse> {
    let configs = tenant_config_service
        .get_all_configs(auth_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(configs))
}

/// Get tenant config by key
#[utoipa::path(
    get, path = "/api/v1/tenant-configs/{key}", tag = "Tenant Config",
    params(("key" = String, Path, description = "Config key")),
    responses((status = 200, description = "Config found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_tenant_config(
    auth_user: AuthUser,
    tenant_config_service: web::Data<TenantConfigService>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    let config = tenant_config_service
        .get_config(auth_user.0.tenant_id, &path.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(config))
}

/// Update tenant config (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/tenant-configs/{id}", tag = "Tenant Config",
    params(("id" = i64, Path, description = "Config ID")),
    request_body = UpdateTenantConfig,
    responses((status = 200, description = "Config updated"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn update_tenant_config(
    _admin_user: AdminUser,
    tenant_config_service: web::Data<TenantConfigService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateTenantConfig>,
) -> ApiResult<HttpResponse> {
    let config = tenant_config_service
        .update_config(*path, payload.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(config))
}

/// Delete tenant config (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/tenant-configs/{id}", tag = "Tenant Config",
    params(("id" = i64, Path, description = "Config ID")),
    responses((status = 204, description = "Config deleted"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn delete_tenant_config(
    _admin_user: AdminUser,
    tenant_config_service: web::Data<TenantConfigService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    tenant_config_service.delete_config(*path).await?;
    Ok(HttpResponse::NoContent().finish())
}

/// Configure tenant routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/v1/tenants")
            .route(web::get().to(get_tenants))
            .route(web::post().to(create_tenant)),
    )
    .service(
        web::resource("/v1/tenants/{id}")
            .route(web::get().to(get_tenant))
            .route(web::put().to(update_tenant))
            .route(web::delete().to(delete_tenant)),
    )
    .service(
        web::resource("/v1/tenant-configs")
            .route(web::get().to(get_tenant_configs))
            .route(web::post().to(create_tenant_config)),
    )
    .service(
        web::resource("/v1/tenant-configs/{id}")
            .route(web::put().to(update_tenant_config))
            .route(web::delete().to(delete_tenant_config)),
    )
    .service(web::resource("/v1/tenant-configs/{key}").route(web::get().to(get_tenant_config)));
}
