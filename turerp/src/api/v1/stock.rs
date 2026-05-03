//! Stock API endpoints (v1)

use actix_web::{web, HttpResponse};

use crate::common::pagination::PaginationParams;
use crate::common::MessageResponse;
use crate::domain::stock::model::{CreateStockMovement, CreateWarehouse};
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
    _auth_user: AuthUser,
    stock_service: web::Data<StockService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match stock_service.get_warehouse(*path).await {
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
    _admin_user: AdminUser,
    stock_service: web::Data<StockService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateWarehouseRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let req = payload.into_inner();
    match stock_service
        .update_warehouse(*path, req.code, req.name, req.address, req.is_active)
        .await
    {
        Ok(warehouse) => Ok(HttpResponse::Ok().json(warehouse)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Delete warehouse (requires admin role)
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
    _admin_user: AdminUser,
    stock_service: web::Data<StockService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match stock_service.delete_warehouse(*path).await {
        Ok(()) => {
            let msg = i18n.t(locale.as_str(), "stock.warehouse.deleted");
            Ok(HttpResponse::Ok().json(MessageResponse { message: msg }))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
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
    create.created_by = admin_user
        .0
        .sub
        .parse()
        .map_err(|_| crate::error::ApiError::InvalidToken("Invalid user ID in token".into()))?;
    match stock_service.create_stock_movement(create).await {
        Ok(movement) => Ok(HttpResponse::Created().json(movement)),
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
    _auth_user: AuthUser,
    stock_service: web::Data<StockService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match stock_service.get_stock_summary(*path).await {
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
        web::resource("/v1/stock/warehouses/{id}")
            .route(web::get().to(get_warehouse))
            .route(web::put().to(update_warehouse))
            .route(web::delete().to(delete_warehouse)),
    )
    .service(web::resource("/v1/stock/movements").route(web::post().to(create_stock_movement)))
    .service(
        web::resource("/v1/stock/movements/product/{product_id}")
            .route(web::get().to(get_stock_movements_by_product)),
    )
    .service(
        web::resource("/v1/stock/movements/warehouse/{warehouse_id}")
            .route(web::get().to(get_stock_movements_by_warehouse)),
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
        web::resource("/v1/stock/summary/{product_id}").route(web::get().to(get_stock_summary)),
    );
}
