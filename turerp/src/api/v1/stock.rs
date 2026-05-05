//! Stock API endpoints (v1)

use actix_web::{web, HttpResponse};

use crate::common::pagination::PaginationParams;
use crate::common::MessageResponse;
use crate::domain::stock::model::{
    CreateStockMovement, CreateWarehouse, StockLevelResponse, StockMovementResponse,
    WarehouseResponse,
};
use crate::domain::stock::service::StockService;
use crate::error::ApiResult;
use crate::i18n::{resolve, I18n, Locale};
use crate::middleware::{AdminUser, AuthUser};

/// Create warehouse (requires admin role)
#[utoipa::path(
    post,
    path = "/api/v1/stock/warehouses",
    tag = "Stock",
    request_body = CreateWarehouse,
    responses(
        (status = 201, description = "Warehouse created"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required")
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_warehouse(
    admin_user: AdminUser,
    stock_service: web::Data<StockService>,
    payload: web::Json<CreateWarehouse>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    match stock_service.create_warehouse(create).await {
        Ok(warehouse) => Ok(HttpResponse::Created().json(warehouse)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get all warehouses (requires authentication)
#[utoipa::path(
    get,
    path = "/api/v1/stock/warehouses",
    tag = "Stock",
    responses(
        (status = 200, description = "List of warehouses"),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_warehouses(
    auth_user: AuthUser,
    stock_service: web::Data<StockService>,
    query: web::Query<PaginationParams>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match stock_service
        .get_warehouses_paginated(auth_user.0.tenant_id, query.page, query.per_page)
        .await
    {
        Ok(result) => Ok(HttpResponse::Ok().json(result)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get warehouse by ID (requires authentication)
#[utoipa::path(
    get,
    path = "/api/v1/stock/warehouses/{id}",
    tag = "Stock",
    params(("id" = i64, Path, description = "Warehouse ID")),
    responses(
        (status = 200, description = "Warehouse found"),
        (status = 404, description = "Warehouse not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_warehouse(
    auth_user: AuthUser,
    stock_service: web::Data<StockService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match stock_service
        .get_warehouse(*path, auth_user.0.tenant_id)
        .await
    {
        Ok(warehouse) => Ok(HttpResponse::Ok().json(warehouse)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Update warehouse (requires admin role)
#[utoipa::path(
    put,
    path = "/api/v1/stock/warehouses/{id}",
    tag = "Stock",
    params(("id" = i64, Path, description = "Warehouse ID")),
    request_body = UpdateWarehouseRequest,
    responses(
        (status = 200, description = "Warehouse updated"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 404, description = "Warehouse not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_warehouse(
    admin_user: AdminUser,
    stock_service: web::Data<StockService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateWarehouseRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let req = payload.into_inner();
    match stock_service
        .update_warehouse(
            *path,
            admin_user.0.tenant_id,
            req.code,
            req.name,
            req.address,
            req.is_active,
        )
        .await
    {
        Ok(warehouse) => Ok(HttpResponse::Ok().json(warehouse)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Delete warehouse (soft delete, requires admin role)
#[utoipa::path(
    delete,
    path = "/api/v1/stock/warehouses/{id}",
    tag = "Stock",
    params(("id" = i64, Path, description = "Warehouse ID")),
    responses(
        (status = 200, description = "Warehouse deleted", body = MessageResponse),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 404, description = "Warehouse not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_warehouse(
    admin_user: AdminUser,
    stock_service: web::Data<StockService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match stock_service
        .delete_warehouse(*path, admin_user.0.tenant_id, admin_user.0.user_id()?)
        .await
    {
        Ok(()) => {
            let msg = i18n.t(locale.as_str(), "stock.warehouse.deleted");
            Ok(HttpResponse::Ok().json(MessageResponse { message: msg }))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Restore a soft-deleted warehouse (admin only)
#[utoipa::path(
    put,
    path = "/api/v1/stock/warehouses/{id}/restore",
    params(("id" = i64, Path, description = "Warehouse ID")),
    responses(
        (status = 200, description = "Warehouse restored", body = WarehouseResponse),
        (status = 404, description = "Warehouse not found or not deleted"),
    ),
    tag = "Stock",
    security(("bearer_auth" = []))
)]
pub async fn restore_warehouse(
    admin_user: AdminUser,
    stock_service: web::Data<StockService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let warehouse = stock_service
        .restore_warehouse(*path, admin_user.0.tenant_id)
        .await?;
    let response: WarehouseResponse = warehouse.into();
    Ok(HttpResponse::Ok().json(response))
}

/// List soft-deleted warehouses (admin only)
#[utoipa::path(
    get,
    path = "/api/v1/stock/warehouses/deleted",
    responses(
        (status = 200, description = "List of deleted warehouses"),
    ),
    tag = "Stock",
    security(("bearer_auth" = []))
)]
pub async fn list_deleted_warehouses(
    admin_user: AdminUser,
    stock_service: web::Data<StockService>,
) -> ApiResult<HttpResponse> {
    let warehouses: Vec<_> = stock_service
        .list_deleted_warehouses(admin_user.0.tenant_id)
        .await?
        .into_iter()
        .map(WarehouseResponse::from)
        .collect();
    Ok(HttpResponse::Ok().json(warehouses))
}

/// Permanently delete a warehouse (admin only)
#[utoipa::path(
    delete,
    path = "/api/v1/stock/warehouses/{id}/destroy",
    params(("id" = i64, Path, description = "Warehouse ID")),
    responses(
        (status = 204, description = "Warehouse permanently deleted"),
        (status = 404, description = "Warehouse not found"),
    ),
    tag = "Stock",
    security(("bearer_auth" = []))
)]
pub async fn destroy_warehouse(
    admin_user: AdminUser,
    stock_service: web::Data<StockService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    stock_service
        .destroy_warehouse(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

/// Create stock movement (requires admin role)
#[utoipa::path(
    post,
    path = "/api/v1/stock/movements",
    tag = "Stock",
    request_body = CreateStockMovement,
    responses(
        (status = 201, description = "Stock movement created"),
        (status = 403, description = "Forbidden - admin role required")
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_stock_movement(
    admin_user: AdminUser,
    stock_service: web::Data<StockService>,
    payload: web::Json<CreateStockMovement>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let mut create = payload.into_inner();
    create.created_by = admin_user.0.user_id()?;
    match stock_service
        .create_stock_movement(create, admin_user.0.tenant_id)
        .await
    {
        Ok(movement) => Ok(HttpResponse::Created().json(movement)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// List all stock movements for current tenant (requires authentication)
#[utoipa::path(
    get,
    path = "/api/v1/stock/movements",
    tag = "Stock",
    responses(
        (status = 200, description = "List of stock movements"),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_stock_movements(
    auth_user: AuthUser,
    stock_service: web::Data<StockService>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match stock_service
        .get_stock_movements_by_tenant(auth_user.0.tenant_id)
        .await
    {
        Ok(movements) => Ok(HttpResponse::Ok().json(movements)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get stock movements by product (requires authentication)
#[utoipa::path(
    get,
    path = "/api/v1/stock/movements/product/{product_id}",
    tag = "Stock",
    params(("product_id" = i64, Path, description = "Product ID")),
    responses(
        (status = 200, description = "Stock movements for product"),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_stock_movements_by_product(
    _auth_user: AuthUser,
    stock_service: web::Data<StockService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match stock_service.get_stock_movements_by_product(*path).await {
        Ok(movements) => Ok(HttpResponse::Ok().json(movements)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get stock movements by warehouse (requires authentication)
#[utoipa::path(
    get,
    path = "/api/v1/stock/movements/warehouse/{warehouse_id}",
    tag = "Stock",
    params(("warehouse_id" = i64, Path, description = "Warehouse ID")),
    responses(
        (status = 200, description = "Stock movements for warehouse"),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_stock_movements_by_warehouse(
    _auth_user: AuthUser,
    stock_service: web::Data<StockService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match stock_service.get_stock_movements_by_warehouse(*path).await {
        Ok(movements) => Ok(HttpResponse::Ok().json(movements)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Soft delete a stock movement (admin only)
#[utoipa::path(
    delete,
    path = "/api/v1/stock/movements/{id}",
    tag = "Stock",
    params(("id" = i64, Path, description = "Movement ID")),
    responses(
        (status = 200, description = "Stock movement deleted", body = MessageResponse),
        (status = 404, description = "Stock movement not found"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_stock_movement(
    admin_user: AdminUser,
    stock_service: web::Data<StockService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match stock_service
        .delete_stock_movement(*path, admin_user.0.tenant_id, admin_user.0.user_id()?)
        .await
    {
        Ok(()) => {
            let msg = i18n.t(locale.as_str(), "stock.movement.deleted");
            Ok(HttpResponse::Ok().json(MessageResponse { message: msg }))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Restore a soft-deleted stock movement (admin only)
#[utoipa::path(
    put,
    path = "/api/v1/stock/movements/{id}/restore",
    params(("id" = i64, Path, description = "Movement ID")),
    responses(
        (status = 200, description = "Stock movement restored", body = StockMovementResponse),
        (status = 404, description = "Stock movement not found or not deleted"),
    ),
    tag = "Stock",
    security(("bearer_auth" = []))
)]
pub async fn restore_stock_movement(
    admin_user: AdminUser,
    stock_service: web::Data<StockService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let movement = stock_service
        .restore_stock_movement(*path, admin_user.0.tenant_id)
        .await?;
    let response: StockMovementResponse = movement.into();
    Ok(HttpResponse::Ok().json(response))
}

/// List soft-deleted stock movements for a warehouse (admin only)
#[utoipa::path(
    get,
    path = "/api/v1/stock/movements/warehouse/{warehouse_id}/deleted",
    params(("warehouse_id" = i64, Path, description = "Warehouse ID")),
    responses(
        (status = 200, description = "List of deleted stock movements"),
    ),
    tag = "Stock",
    security(("bearer_auth" = []))
)]
pub async fn list_deleted_stock_movements(
    _admin_user: AdminUser,
    stock_service: web::Data<StockService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let movements: Vec<_> = stock_service
        .list_deleted_stock_movements(*path)
        .await?
        .into_iter()
        .map(StockMovementResponse::from)
        .collect();
    Ok(HttpResponse::Ok().json(movements))
}

/// Permanently delete a stock movement (admin only)
#[utoipa::path(
    delete,
    path = "/api/v1/stock/movements/{id}/destroy",
    params(("id" = i64, Path, description = "Movement ID")),
    responses(
        (status = 204, description = "Stock movement permanently deleted"),
        (status = 404, description = "Stock movement not found"),
    ),
    tag = "Stock",
    security(("bearer_auth" = []))
)]
pub async fn destroy_stock_movement(
    admin_user: AdminUser,
    stock_service: web::Data<StockService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    stock_service
        .destroy_stock_movement(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

/// Get stock levels by product (requires authentication)
#[utoipa::path(
    get,
    path = "/api/v1/stock/levels/product/{product_id}",
    tag = "Stock",
    params(("product_id" = i64, Path, description = "Product ID")),
    responses(
        (status = 200, description = "Stock levels for product"),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_stock_by_product(
    _auth_user: AuthUser,
    stock_service: web::Data<StockService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match stock_service.get_stock_by_product(*path).await {
        Ok(levels) => Ok(HttpResponse::Ok().json(levels)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get stock levels by warehouse (requires authentication)
#[utoipa::path(
    get,
    path = "/api/v1/stock/levels/warehouse/{warehouse_id}",
    tag = "Stock",
    params(("warehouse_id" = i64, Path, description = "Warehouse ID")),
    responses(
        (status = 200, description = "Stock levels for warehouse"),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_stock_by_warehouse(
    _auth_user: AuthUser,
    stock_service: web::Data<StockService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match stock_service.get_stock_by_warehouse(*path).await {
        Ok(levels) => Ok(HttpResponse::Ok().json(levels)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Soft delete a stock level (admin only)
#[utoipa::path(
    delete,
    path = "/api/v1/stock/levels/warehouse/{warehouse_id}/product/{product_id}",
    tag = "Stock",
    params(("warehouse_id" = i64, Path, description = "Warehouse ID"), ("product_id" = i64, Path, description = "Product ID")),
    responses(
        (status = 200, description = "Stock level deleted", body = MessageResponse),
        (status = 404, description = "Stock level not found"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_stock_level(
    admin_user: AdminUser,
    stock_service: web::Data<StockService>,
    path: web::Path<(i64, i64)>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let (warehouse_id, product_id) = path.into_inner();
    match stock_service
        .delete_stock_level(warehouse_id, product_id, admin_user.0.user_id()?)
        .await
    {
        Ok(()) => {
            let msg = i18n.t(locale.as_str(), "stock.level.deleted");
            Ok(HttpResponse::Ok().json(MessageResponse { message: msg }))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Restore a soft-deleted stock level (admin only)
#[utoipa::path(
    put,
    path = "/api/v1/stock/levels/warehouse/{warehouse_id}/product/{product_id}/restore",
    params(("warehouse_id" = i64, Path, description = "Warehouse ID"), ("product_id" = i64, Path, description = "Product ID")),
    responses(
        (status = 200, description = "Stock level restored", body = StockLevelResponse),
        (status = 404, description = "Stock level not found or not deleted"),
    ),
    tag = "Stock",
    security(("bearer_auth" = []))
)]
pub async fn restore_stock_level(
    _admin_user: AdminUser,
    stock_service: web::Data<StockService>,
    path: web::Path<(i64, i64)>,
) -> ApiResult<HttpResponse> {
    let (warehouse_id, product_id) = path.into_inner();
    let level = stock_service
        .restore_stock_level(warehouse_id, product_id)
        .await?;
    let response: StockLevelResponse = level.into();
    Ok(HttpResponse::Ok().json(response))
}

/// List soft-deleted stock levels for a warehouse (admin only)
#[utoipa::path(
    get,
    path = "/api/v1/stock/levels/warehouse/{warehouse_id}/deleted",
    params(("warehouse_id" = i64, Path, description = "Warehouse ID")),
    responses(
        (status = 200, description = "List of deleted stock levels"),
    ),
    tag = "Stock",
    security(("bearer_auth" = []))
)]
pub async fn list_deleted_stock_levels(
    _admin_user: AdminUser,
    stock_service: web::Data<StockService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let levels: Vec<_> = stock_service
        .list_deleted_stock_levels(*path)
        .await?
        .into_iter()
        .map(StockLevelResponse::from)
        .collect();
    Ok(HttpResponse::Ok().json(levels))
}

/// Permanently delete a stock level (admin only)
#[utoipa::path(
    delete,
    path = "/api/v1/stock/levels/warehouse/{warehouse_id}/product/{product_id}/destroy",
    params(("warehouse_id" = i64, Path, description = "Warehouse ID"), ("product_id" = i64, Path, description = "Product ID")),
    responses(
        (status = 204, description = "Stock level permanently deleted"),
        (status = 404, description = "Stock level not found"),
    ),
    tag = "Stock",
    security(("bearer_auth" = []))
)]
pub async fn destroy_stock_level(
    _admin_user: AdminUser,
    stock_service: web::Data<StockService>,
    path: web::Path<(i64, i64)>,
) -> ApiResult<HttpResponse> {
    let (warehouse_id, product_id) = path.into_inner();
    stock_service
        .destroy_stock_level(warehouse_id, product_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

/// Get stock summary (requires authentication)
#[utoipa::path(
    get,
    path = "/api/v1/stock/summary/{product_id}",
    tag = "Stock",
    params(("product_id" = i64, Path, description = "Product ID")),
    responses(
        (status = 200, description = "Stock summary"),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_stock_summary(
    auth_user: AuthUser,
    stock_service: web::Data<StockService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match stock_service
        .get_stock_summary(*path, auth_user.0.tenant_id)
        .await
    {
        Ok(summary) => Ok(HttpResponse::Ok().json(summary)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct UpdateWarehouseRequest {
    pub code: Option<String>,
    pub name: Option<String>,
    pub address: Option<String>,
    pub is_active: Option<bool>,
}

/// Configure stock routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/v1/stock/warehouses")
            .route(web::get().to(get_warehouses))
            .route(web::post().to(create_warehouse)),
    )
    .service(
        web::resource("/v1/stock/warehouses/deleted").route(web::get().to(list_deleted_warehouses)),
    )
    .service(
        web::resource("/v1/stock/warehouses/{id}")
            .route(web::get().to(get_warehouse))
            .route(web::put().to(update_warehouse))
            .route(web::delete().to(delete_warehouse)),
    )
    .service(
        web::resource("/v1/stock/warehouses/{id}/restore").route(web::put().to(restore_warehouse)),
    )
    .service(
        web::resource("/v1/stock/warehouses/{id}/destroy")
            .route(web::delete().to(destroy_warehouse)),
    )
    .service(
        web::resource("/v1/stock/movements")
            .route(web::get().to(list_stock_movements))
            .route(web::post().to(create_stock_movement)),
    )
    .service(
        web::resource("/v1/stock/movements/product/{product_id}")
            .route(web::get().to(get_stock_movements_by_product)),
    )
    .service(
        web::resource("/v1/stock/movements/warehouse/{warehouse_id}")
            .route(web::get().to(get_stock_movements_by_warehouse)),
    )
    .service(
        web::resource("/v1/stock/movements/warehouse/{warehouse_id}/deleted")
            .route(web::get().to(list_deleted_stock_movements)),
    )
    .service(
        web::resource("/v1/stock/movements/{id}").route(web::delete().to(delete_stock_movement)),
    )
    .service(
        web::resource("/v1/stock/movements/{id}/restore")
            .route(web::put().to(restore_stock_movement)),
    )
    .service(
        web::resource("/v1/stock/movements/{id}/destroy")
            .route(web::delete().to(destroy_stock_movement)),
    )
    .service(
        web::resource("/v1/stock/levels/product/{product_id}")
            .route(web::get().to(get_stock_by_product)),
    )
    .service(
        web::resource("/v1/stock/levels/warehouse/{warehouse_id}")
            .route(web::get().to(get_stock_by_warehouse)),
    )
    .service(
        web::resource("/v1/stock/levels/warehouse/{warehouse_id}/deleted")
            .route(web::get().to(list_deleted_stock_levels)),
    )
    .service(
        web::resource("/v1/stock/levels/warehouse/{warehouse_id}/product/{product_id}")
            .route(web::delete().to(delete_stock_level)),
    )
    .service(
        web::resource("/v1/stock/levels/warehouse/{warehouse_id}/product/{product_id}/restore")
            .route(web::put().to(restore_stock_level)),
    )
    .service(
        web::resource("/v1/stock/levels/warehouse/{warehouse_id}/product/{product_id}/destroy")
            .route(web::delete().to(destroy_stock_level)),
    )
    .service(
        web::resource("/v1/stock/summary/{product_id}").route(web::get().to(get_stock_summary)),
    );
}
