//! Routing handlers

use actix_web::{web, HttpResponse};

use crate::common::MessageResponse;
use crate::domain::manufacturing::model::{CreateRouting, CreateRoutingOperation};
use crate::domain::manufacturing::service::ManufacturingService;
use crate::error::{ApiError, ApiResult};
use crate::i18n::{resolve, I18n, Locale};
use crate::json_resp;
use crate::middleware::{AdminUser, AuthUser};

/// Create routing (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/manufacturing/routings", tag = "Manufacturing",
    request_body = CreateRouting,
    responses((status = 201, description = "Routing created"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_routing(
    admin_user: AdminUser,
    mfg_service: web::Data<ManufacturingService>,
    payload: web::Json<CreateRouting>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    json_resp!(
        mfg_service.create_routing(create),
        HttpResponse::Created,
        i18n,
        locale.as_str()
    )
}

/// Get routing by ID
#[utoipa::path(
    get, path = "/api/v1/manufacturing/routings/{id}", tag = "Manufacturing",
    params(("id" = i64, Path, description = "Routing ID")),
    responses((status = 200, description = "Routing found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_routing(
    auth_user: AuthUser,
    mfg_service: web::Data<ManufacturingService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        mfg_service.get_routing(*path, auth_user.0.tenant_id),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Get routings by product
#[utoipa::path(
    get, path = "/api/v1/manufacturing/routings/product/{product_id}", tag = "Manufacturing",
    params(("product_id" = i64, Path, description = "Product ID")),
    responses((status = 200, description = "Routings for product")),
    security(("bearer_auth" = []))
)]
pub async fn get_routings_by_product(
    _auth_user: AuthUser,
    mfg_service: web::Data<ManufacturingService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        mfg_service.get_routings_by_product(*path),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Add routing operation (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/manufacturing/routings/operations", tag = "Manufacturing",
    request_body = CreateRoutingOperation,
    responses((status = 201, description = "Routing operation added"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn add_routing_operation(
    _admin_user: AdminUser,
    mfg_service: web::Data<ManufacturingService>,
    payload: web::Json<CreateRoutingOperation>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        mfg_service.add_routing_operation(payload.into_inner()),
        HttpResponse::Created,
        i18n,
        locale.as_str()
    )
}

/// Calculate material requirements
#[utoipa::path(
    get, path = "/api/v1/manufacturing/material-requirements/{product_id}", tag = "Manufacturing",
    params(("product_id" = i64, Path, description = "Product ID"), ("quantity" = String, Query, description = "Quantity")),
    responses((status = 200, description = "Material requirements")),
    security(("bearer_auth" = []))
)]
pub async fn calculate_material_requirements(
    _auth_user: AuthUser,
    mfg_service: web::Data<ManufacturingService>,
    path: web::Path<i64>,
    query: web::Query<QuantityQuery>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let quantity: rust_decimal::Decimal = query.quantity.parse().map_err(|_| {
        let msg = i18n.t(locale.as_str(), "generic.validation_error");
        ApiError::Validation(msg)
    })?;
    json_resp!(
        mfg_service.calculate_material_requirements(*path, quantity),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Soft-delete a routing (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/manufacturing/routings/{id}", tag = "Manufacturing",
    params(("id" = i64, Path, description = "Routing ID")),
    responses((status = 200, description = "Routing soft-deleted", body = MessageResponse), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn soft_delete_routing(
    admin_user: AdminUser,
    mfg_service: web::Data<ManufacturingService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match mfg_service
        .soft_delete_routing(*path, admin_user.0.tenant_id, admin_user.0.user_id()?)
        .await
    {
        Ok(()) => Ok(HttpResponse::Ok().json(MessageResponse {
            message: "Routing soft-deleted".to_string(),
        })),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Restore a soft-deleted routing (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/manufacturing/routings/{id}/restore", tag = "Manufacturing",
    params(("id" = i64, Path, description = "Routing ID")),
    responses((status = 200, description = "Routing restored"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn restore_routing(
    admin_user: AdminUser,
    mfg_service: web::Data<ManufacturingService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let routing = mfg_service
        .restore_routing(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(routing))
}

/// List soft-deleted routings (requires admin role)
#[utoipa::path(
    get, path = "/api/v1/manufacturing/routings/deleted", tag = "Manufacturing",
    responses((status = 200, description = "List of deleted routings")),
    security(("bearer_auth" = []))
)]
pub async fn list_deleted_routings(
    admin_user: AdminUser,
    mfg_service: web::Data<ManufacturingService>,
) -> ApiResult<HttpResponse> {
    let deleted = mfg_service
        .list_deleted_routings(admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(deleted))
}

/// Permanently destroy a routing (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/manufacturing/routings/{id}/destroy", tag = "Manufacturing",
    params(("id" = i64, Path, description = "Routing ID")),
    responses((status = 204, description = "Routing permanently deleted"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn destroy_routing(
    admin_user: AdminUser,
    mfg_service: web::Data<ManufacturingService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    mfg_service
        .destroy_routing(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct QuantityQuery {
    pub quantity: String,
}
