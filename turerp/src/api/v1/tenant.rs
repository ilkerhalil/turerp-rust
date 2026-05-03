//! Tenant API endpoints (v1)

use actix_web::{web, HttpResponse};
use serde::Serialize;

use crate::common::pagination::PaginationParams;
use crate::domain::tenant::model::{
    CreateTenant, CreateTenantConfig, UpdateTenant, UpdateTenantConfig,
};
use crate::domain::tenant::service::{TenantConfigService, TenantService};
use crate::error::{ApiError, ApiResult};
use crate::i18n::{resolve, I18n, Locale};
use crate::middleware::{AdminUser, AuthUser};

/// Simple localized success message payload.
#[derive(Serialize)]
pub struct MessageResponse {
    pub message: String,
}

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
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match tenant_service.create_tenant(payload.into_inner()).await {
        Ok(tenant) => Ok(HttpResponse::Created().json(tenant)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
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
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    if let Err(e) = pagination.validate() {
        let err = ApiError::Validation(e.to_string());
        return Ok(err.to_http_response(i18n, locale.as_str()));
    }
    match tenant_service
        .get_all_tenants_paginated(pagination.page, pagination.per_page)
        .await
    {
        Ok(result) => Ok(HttpResponse::Ok().json(result)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
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
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match tenant_service.get_tenant(*path).await {
        Ok(tenant) => Ok(HttpResponse::Ok().json(tenant)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
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
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match tenant_service
        .update_tenant(*path, payload.into_inner())
        .await
    {
        Ok(tenant) => Ok(HttpResponse::Ok().json(tenant)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Delete tenant (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/tenants/{id}", tag = "Tenant",
    params(("id" = i64, Path, description = "Tenant ID")),
    responses((status = 200, description = "Tenant deleted", body = MessageResponse), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn delete_tenant(
    _admin_user: AdminUser,
    tenant_service: web::Data<TenantService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match tenant_service.delete_tenant(*path).await {
        Ok(()) => {
            let msg = i18n.t(locale.as_str(), "tenant.deleted");
            Ok(HttpResponse::Ok().json(MessageResponse { message: msg }))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

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
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    match tenant_config_service.set_config(create).await {
        Ok(config) => Ok(HttpResponse::Created().json(config)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
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
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match tenant_config_service
        .get_all_configs(auth_user.0.tenant_id)
        .await
    {
        Ok(configs) => Ok(HttpResponse::Ok().json(configs)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get tenant config by key or ID
#[utoipa::path(
    get, path = "/api/v1/tenant-configs/{id_or_key}", tag = "Tenant Config",
    params(("id_or_key" = String, Path, description = "Config key")),
    responses((status = 200, description = "Config found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_tenant_config(
    auth_user: AuthUser,
    tenant_config_service: web::Data<TenantConfigService>,
    path: web::Path<String>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let key = path.into_inner();
    match tenant_config_service
        .get_config(auth_user.0.tenant_id, &key)
        .await
    {
        Ok(config) => Ok(HttpResponse::Ok().json(config)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Update tenant config (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/tenant-configs/{id_or_key}", tag = "Tenant Config",
    params(("id_or_key" = String, Path, description = "Config ID")),
    request_body = UpdateTenantConfig,
    responses((status = 200, description = "Config updated"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn update_tenant_config(
    _admin_user: AdminUser,
    tenant_config_service: web::Data<TenantConfigService>,
    path: web::Path<String>,
    payload: web::Json<UpdateTenantConfig>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner().parse::<i64>().map_err(|_| {
        let msg = i18n.t(locale.as_str(), "generic.validation_error");
        ApiError::Validation(msg)
    })?;
    match tenant_config_service
        .update_config(id, payload.into_inner())
        .await
    {
        Ok(config) => Ok(HttpResponse::Ok().json(config)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Delete tenant config (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/tenant-configs/{id_or_key}", tag = "Tenant Config",
    params(("id_or_key" = String, Path, description = "Config ID")),
    responses((status = 200, description = "Config deleted", body = MessageResponse), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn delete_tenant_config(
    _admin_user: AdminUser,
    tenant_config_service: web::Data<TenantConfigService>,
    path: web::Path<String>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner().parse::<i64>().map_err(|_| {
        let msg = i18n.t(locale.as_str(), "generic.validation_error");
        ApiError::Validation(msg)
    })?;
    match tenant_config_service.delete_config(id).await {
        Ok(()) => {
            let msg = i18n.t(locale.as_str(), "tenant.config_updated");
            Ok(HttpResponse::Ok().json(MessageResponse { message: msg }))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
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
        web::resource("/v1/tenant-configs/{id_or_key}")
            .route(web::get().to(get_tenant_config))
            .route(web::put().to(update_tenant_config))
            .route(web::delete().to(delete_tenant_config)),
    );
}
