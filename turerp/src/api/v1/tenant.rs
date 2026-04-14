//! Tenant API endpoints (v1)

use actix_web::{web, HttpResponse};

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
    responses((status = 200, description = "List of tenants")),
    security(("bearer_auth" = []))
)]
pub async fn get_tenants(
    _auth_user: AuthUser,
    tenant_service: web::Data<TenantService>,
) -> ApiResult<HttpResponse> {
    let tenants = tenant_service.get_all_tenants().await?;
    Ok(HttpResponse::Ok().json(tenants))
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
    );
}
