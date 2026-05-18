//! Work order handlers

use actix_web::{web, HttpResponse};

use crate::common::pagination::PaginationParams;
use crate::common::MessageResponse;
use crate::domain::manufacturing::model::{
    CreateWorkOrder, CreateWorkOrderMaterial, CreateWorkOrderOperation, WorkOrderStatus,
};
use crate::domain::manufacturing::service::ManufacturingService;
use crate::error::{ApiError, ApiResult};
use crate::i18n::{resolve, I18n, Locale};
use crate::json_resp;
use crate::middleware::{AdminUser, AuthUser};

/// Create work order (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/manufacturing/work-orders", tag = "Manufacturing",
    request_body = CreateWorkOrder,
    responses((status = 201, description = "Work order created"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_work_order(
    admin_user: AdminUser,
    mfg_service: web::Data<ManufacturingService>,
    payload: web::Json<CreateWorkOrder>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    json_resp!(
        mfg_service.create_work_order(create),
        HttpResponse::Created,
        i18n,
        locale.as_str()
    )
}

/// Get all work orders
#[utoipa::path(
    get, path = "/api/v1/manufacturing/work-orders", tag = "Manufacturing",
    responses((status = 200, description = "List of work orders")),
    security(("bearer_auth" = []))
)]
pub async fn get_work_orders(
    auth_user: AuthUser,
    mfg_service: web::Data<ManufacturingService>,
    query: web::Query<PaginationParams>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let pagination = query.into_inner();
    if let Err(e) = pagination.validate() {
        let err = ApiError::Validation(e.to_string());
        return Ok(err.to_http_response(i18n, locale.as_str()));
    }
    json_resp!(
        mfg_service.get_work_orders_by_tenant_paginated(
            auth_user.0.tenant_id,
            pagination.page,
            pagination.per_page
        ),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Get work order by ID
#[utoipa::path(
    get, path = "/api/v1/manufacturing/work-orders/{id}", tag = "Manufacturing",
    params(("id" = i64, Path, description = "Work order ID")),
    responses((status = 200, description = "Work order found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_work_order(
    auth_user: AuthUser,
    mfg_service: web::Data<ManufacturingService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        mfg_service.get_work_order(*path, auth_user.0.tenant_id),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Update work order status (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/manufacturing/work-orders/{id}/status", tag = "Manufacturing",
    params(("id" = i64, Path, description = "Work order ID")),
    request_body = UpdateWorkOrderStatusRequest,
    responses((status = 200, description = "Status updated"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn update_work_order_status(
    _admin_user: AdminUser,
    mfg_service: web::Data<ManufacturingService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateWorkOrderStatusRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        mfg_service.update_work_order_status(*path, payload.into_inner().status),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Add work order operation (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/manufacturing/work-orders/operations", tag = "Manufacturing",
    request_body = CreateWorkOrderOperation,
    responses((status = 201, description = "Operation added"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn add_work_order_operation(
    _admin_user: AdminUser,
    mfg_service: web::Data<ManufacturingService>,
    payload: web::Json<CreateWorkOrderOperation>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        mfg_service.add_work_order_operation(payload.into_inner()),
        HttpResponse::Created,
        i18n,
        locale.as_str()
    )
}

/// Get work order operations
#[utoipa::path(
    get, path = "/api/v1/manufacturing/work-orders/{work_order_id}/operations", tag = "Manufacturing",
    params(("work_order_id" = i64, Path, description = "Work order ID")),
    responses((status = 200, description = "Operations for work order")),
    security(("bearer_auth" = []))
)]
pub async fn get_work_order_operations(
    _auth_user: AuthUser,
    mfg_service: web::Data<ManufacturingService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        mfg_service.get_work_order_operations(*path),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Add work order material (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/manufacturing/work-orders/materials", tag = "Manufacturing",
    request_body = CreateWorkOrderMaterial,
    responses((status = 201, description = "Material added"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn add_work_order_material(
    _admin_user: AdminUser,
    mfg_service: web::Data<ManufacturingService>,
    payload: web::Json<CreateWorkOrderMaterial>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        mfg_service.add_work_order_material(payload.into_inner()),
        HttpResponse::Created,
        i18n,
        locale.as_str()
    )
}

/// Get work order materials
#[utoipa::path(
    get, path = "/api/v1/manufacturing/work-orders/{work_order_id}/materials", tag = "Manufacturing",
    params(("work_order_id" = i64, Path, description = "Work order ID")),
    responses((status = 200, description = "Materials for work order")),
    security(("bearer_auth" = []))
)]
pub async fn get_work_order_materials(
    _auth_user: AuthUser,
    mfg_service: web::Data<ManufacturingService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        mfg_service.get_work_order_materials(*path),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Soft-delete a work order (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/manufacturing/work-orders/{id}", tag = "Manufacturing",
    params(("id" = i64, Path, description = "Work order ID")),
    responses((status = 200, description = "Work order soft-deleted", body = MessageResponse), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn soft_delete_work_order(
    admin_user: AdminUser,
    mfg_service: web::Data<ManufacturingService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match mfg_service
        .soft_delete_work_order(*path, admin_user.0.tenant_id, admin_user.0.user_id()?)
        .await
    {
        Ok(()) => Ok(HttpResponse::Ok().json(MessageResponse {
            message: "Work order soft-deleted".to_string(),
        })),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Restore a soft-deleted work order (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/manufacturing/work-orders/{id}/restore", tag = "Manufacturing",
    params(("id" = i64, Path, description = "Work order ID")),
    responses((status = 200, description = "Work order restored"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn restore_work_order(
    admin_user: AdminUser,
    mfg_service: web::Data<ManufacturingService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let work_order = mfg_service
        .restore_work_order(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(work_order))
}

/// List soft-deleted work orders (requires admin role)
#[utoipa::path(
    get, path = "/api/v1/manufacturing/work-orders/deleted", tag = "Manufacturing",
    responses((status = 200, description = "List of deleted work orders")),
    security(("bearer_auth" = []))
)]
pub async fn list_deleted_work_orders(
    admin_user: AdminUser,
    mfg_service: web::Data<ManufacturingService>,
) -> ApiResult<HttpResponse> {
    let deleted = mfg_service
        .list_deleted_work_orders(admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(deleted))
}

/// Permanently destroy a work order (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/manufacturing/work-orders/{id}/destroy", tag = "Manufacturing",
    params(("id" = i64, Path, description = "Work order ID")),
    responses((status = 204, description = "Work order permanently deleted"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn destroy_work_order(
    admin_user: AdminUser,
    mfg_service: web::Data<ManufacturingService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    mfg_service
        .destroy_work_order(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct UpdateWorkOrderStatusRequest {
    pub status: WorkOrderStatus,
}
