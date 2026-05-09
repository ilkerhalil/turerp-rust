//! Unit handlers

use actix_web::{web, HttpResponse};

use crate::common::pagination::PaginationParams;
use crate::common::MessageResponse;
use crate::domain::product::{CreateUnit, ProductService, UnitResponse, UpdateUnit};
use crate::error::ApiResult;
use crate::i18n::{resolve, I18n, Locale};
use crate::middleware::{AdminUser, AuthUser};

/// Get all units (paginated)
#[utoipa::path(
    get,
    path = "/api/v1/units",
    tag = "Products",
    params(PaginationParams),
    responses(
        (status = 200, description = "Paginated list of units"),
        (status = 401, description = "Not authenticated - missing or invalid JWT token")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_units(
    auth_user: AuthUser,
    service: web::Data<ProductService>,
    query: web::Query<PaginationParams>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match service
        .get_units_paginated(auth_user.0.tenant_id, query.page, query.per_page)
        .await
    {
        Ok(result) => {
            let response = crate::common::pagination::PaginatedResult::new(
                result.items.into_iter().map(UnitResponse::from).collect(),
                result.page,
                result.per_page,
                result.total,
            );
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Create a new unit (admin only)
#[utoipa::path(
    post,
    path = "/api/v1/units",
    tag = "Products",
    request_body = CreateUnit,
    responses(
        (status = 201, description = "Unit created successfully", body = UnitResponse),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn create_unit(
    admin_user: AdminUser,
    service: web::Data<ProductService>,
    payload: web::Json<CreateUnit>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;

    match service.create_unit(create).await {
        Ok(unit) => Ok(HttpResponse::Created().json(UnitResponse::from(unit))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get a single unit by ID
#[utoipa::path(
    get,
    path = "/api/v1/units/{id}",
    tag = "Products",
    params(("id" = i64, Path, description = "Unit ID")),
    responses(
        (status = 200, description = "Unit found", body = UnitResponse),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "Unit not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_unit(
    auth_user: AuthUser,
    service: web::Data<ProductService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match service.get_unit(*path, auth_user.0.tenant_id).await {
        Ok(unit) => Ok(HttpResponse::Ok().json(UnitResponse::from(unit))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Update a unit (admin only)
#[utoipa::path(
    put,
    path = "/api/v1/units/{id}",
    tag = "Products",
    params(("id" = i64, Path, description = "Unit ID")),
    request_body = UpdateUnit,
    responses(
        (status = 200, description = "Unit updated", body = UnitResponse),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 404, description = "Unit not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn update_unit(
    admin_user: AdminUser,
    service: web::Data<ProductService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateUnit>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match service
        .update_unit(*path, admin_user.0.tenant_id, payload.into_inner())
        .await
    {
        Ok(unit) => Ok(HttpResponse::Ok().json(UnitResponse::from(unit))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Soft delete a unit (admin only)
#[utoipa::path(
    delete,
    path = "/api/v1/units/{id}",
    tag = "Products",
    params(("id" = i64, Path, description = "Unit ID")),
    responses(
        (status = 200, description = "Unit soft deleted", body = MessageResponse),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 404, description = "Unit not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_unit(
    admin_user: AdminUser,
    service: web::Data<ProductService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match service
        .soft_delete_unit(*path, admin_user.0.tenant_id, admin_user.0.user_id()?)
        .await
    {
        Ok(()) => {
            let msg = i18n.t(locale.as_str(), "unit.deleted");
            Ok(HttpResponse::Ok().json(MessageResponse { message: msg }))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Restore a soft-deleted unit (admin only)
#[utoipa::path(
    put,
    path = "/api/v1/units/{id}/restore",
    params(("id" = i64, Path, description = "Unit ID")),
    responses(
        (status = 200, description = "Unit restored", body = UnitResponse),
        (status = 404, description = "Unit not found or not deleted"),
    ),
    tag = "Products",
    security(("bearer_auth" = []))
)]
pub async fn restore_unit(
    admin_user: AdminUser,
    service: web::Data<ProductService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let unit = service.restore_unit(*path, admin_user.0.tenant_id).await?;
    let response: UnitResponse = unit.into();
    Ok(HttpResponse::Ok().json(response))
}

/// List soft-deleted units (admin only)
#[utoipa::path(
    get,
    path = "/api/v1/units/deleted",
    responses(
        (status = 200, description = "List of deleted units"),
    ),
    tag = "Products",
    security(("bearer_auth" = []))
)]
pub async fn list_deleted_units(
    admin_user: AdminUser,
    service: web::Data<ProductService>,
) -> ApiResult<HttpResponse> {
    let units: Vec<_> = service
        .list_deleted_units(admin_user.0.tenant_id)
        .await?
        .into_iter()
        .map(UnitResponse::from)
        .collect();
    Ok(HttpResponse::Ok().json(units))
}

/// Permanently delete a unit (admin only, after soft delete)
#[utoipa::path(
    delete,
    path = "/api/v1/units/{id}/destroy",
    params(("id" = i64, Path, description = "Unit ID")),
    responses(
        (status = 204, description = "Unit permanently deleted"),
        (status = 404, description = "Unit not found"),
    ),
    tag = "Products",
    security(("bearer_auth" = []))
)]
pub async fn destroy_unit(
    admin_user: AdminUser,
    service: web::Data<ProductService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    service.destroy_unit(*path, admin_user.0.tenant_id).await?;
    Ok(HttpResponse::NoContent().finish())
}
