//! Sales API endpoints (v1)

use actix_web::{web, HttpResponse};

use crate::domain::sales::model::{
    CreateQuotation, CreateSalesOrder, QuotationStatus, SalesOrderStatus,
};
use crate::domain::sales::service::SalesService;
use crate::error::ApiResult;
use crate::middleware::{AdminUser, AuthUser};

// --- Sales Orders ---

/// Create sales order (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/sales/orders", tag = "Sales",
    request_body = CreateSalesOrder,
    responses((status = 201, description = "Sales order created"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_sales_order(
    admin_user: AdminUser,
    sales_service: web::Data<SalesService>,
    payload: web::Json<CreateSalesOrder>,
) -> ApiResult<HttpResponse> {
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    let order = sales_service.create_sales_order(create).await?;
    Ok(HttpResponse::Created().json(order))
}

/// Get all sales orders
#[utoipa::path(
    get, path = "/api/v1/sales/orders", tag = "Sales",
    responses((status = 200, description = "List of sales orders")),
    security(("bearer_auth" = []))
)]
pub async fn get_sales_orders(
    auth_user: AuthUser,
    sales_service: web::Data<SalesService>,
) -> ApiResult<HttpResponse> {
    let orders = sales_service
        .get_orders_by_tenant(auth_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(orders))
}

/// Get sales order by ID
#[utoipa::path(
    get, path = "/api/v1/sales/orders/{id}", tag = "Sales",
    params(("id" = i64, Path, description = "Sales order ID")),
    responses((status = 200, description = "Sales order found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_sales_order(
    _auth_user: AuthUser,
    sales_service: web::Data<SalesService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let order = sales_service.get_sales_order(*path).await?;
    Ok(HttpResponse::Ok().json(order))
}

/// Get sales orders by status
#[utoipa::path(
    get, path = "/api/v1/sales/orders/status/{status}", tag = "Sales",
    params(("status" = SalesOrderStatus, Path, description = "Order status")),
    responses((status = 200, description = "List of sales orders by status")),
    security(("bearer_auth" = []))
)]
pub async fn get_sales_orders_by_status(
    auth_user: AuthUser,
    sales_service: web::Data<SalesService>,
    path: web::Path<SalesOrderStatus>,
) -> ApiResult<HttpResponse> {
    let orders = sales_service
        .get_orders_by_status(auth_user.0.tenant_id, path.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(orders))
}

/// Update sales order status (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/sales/orders/{id}/status", tag = "Sales",
    params(("id" = i64, Path, description = "Sales order ID")),
    request_body = UpdateOrderStatusRequest,
    responses((status = 200, description = "Status updated"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn update_sales_order_status(
    _admin_user: AdminUser,
    sales_service: web::Data<SalesService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateOrderStatusRequest>,
) -> ApiResult<HttpResponse> {
    let order = sales_service
        .update_order_status(*path, payload.into_inner().status)
        .await?;
    Ok(HttpResponse::Ok().json(order))
}

/// Delete sales order (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/sales/orders/{id}", tag = "Sales",
    params(("id" = i64, Path, description = "Sales order ID")),
    responses((status = 204, description = "Order deleted"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn delete_sales_order(
    _admin_user: AdminUser,
    sales_service: web::Data<SalesService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    sales_service.delete_order(*path).await?;
    Ok(HttpResponse::NoContent().finish())
}

// --- Quotations ---

/// Create quotation (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/sales/quotations", tag = "Sales",
    request_body = CreateQuotation,
    responses((status = 201, description = "Quotation created"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_quotation(
    admin_user: AdminUser,
    sales_service: web::Data<SalesService>,
    payload: web::Json<CreateQuotation>,
) -> ApiResult<HttpResponse> {
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    let quotation = sales_service.create_quotation(create).await?;
    Ok(HttpResponse::Created().json(quotation))
}

/// Get all quotations
#[utoipa::path(
    get, path = "/api/v1/sales/quotations", tag = "Sales",
    responses((status = 200, description = "List of quotations")),
    security(("bearer_auth" = []))
)]
pub async fn get_quotations(
    auth_user: AuthUser,
    sales_service: web::Data<SalesService>,
) -> ApiResult<HttpResponse> {
    let quotations = sales_service
        .get_quotations_by_tenant(auth_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(quotations))
}

/// Get quotation by ID
#[utoipa::path(
    get, path = "/api/v1/sales/quotations/{id}", tag = "Sales",
    params(("id" = i64, Path, description = "Quotation ID")),
    responses((status = 200, description = "Quotation found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_quotation(
    _auth_user: AuthUser,
    sales_service: web::Data<SalesService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let quotation = sales_service.get_quotation(*path).await?;
    Ok(HttpResponse::Ok().json(quotation))
}

/// Get quotations by status
#[utoipa::path(
    get, path = "/api/v1/sales/quotations/status/{status}", tag = "Sales",
    params(("status" = QuotationStatus, Path, description = "Quotation status")),
    responses((status = 200, description = "List of quotations by status")),
    security(("bearer_auth" = []))
)]
pub async fn get_quotations_by_status(
    auth_user: AuthUser,
    sales_service: web::Data<SalesService>,
    path: web::Path<QuotationStatus>,
) -> ApiResult<HttpResponse> {
    let quotations = sales_service
        .get_quotations_by_status(auth_user.0.tenant_id, path.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(quotations))
}

/// Update quotation status (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/sales/quotations/{id}/status", tag = "Sales",
    params(("id" = i64, Path, description = "Quotation ID")),
    request_body = UpdateQuotationStatusRequest,
    responses((status = 200, description = "Status updated"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn update_quotation_status(
    _admin_user: AdminUser,
    sales_service: web::Data<SalesService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateQuotationStatusRequest>,
) -> ApiResult<HttpResponse> {
    let quotation = sales_service
        .update_quotation_status(*path, payload.into_inner().status)
        .await?;
    Ok(HttpResponse::Ok().json(quotation))
}

/// Convert quotation to sales order (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/sales/quotations/{id}/convert", tag = "Sales",
    params(("id" = i64, Path, description = "Quotation ID")),
    responses((status = 200, description = "Quotation converted to order"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn convert_quotation_to_order(
    _admin_user: AdminUser,
    sales_service: web::Data<SalesService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let order = sales_service.convert_quotation_to_order(*path).await?;
    Ok(HttpResponse::Ok().json(order))
}

/// Delete quotation (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/sales/quotations/{id}", tag = "Sales",
    params(("id" = i64, Path, description = "Quotation ID")),
    responses((status = 204, description = "Quotation deleted"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn delete_quotation(
    _admin_user: AdminUser,
    sales_service: web::Data<SalesService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    sales_service.delete_quotation(*path).await?;
    Ok(HttpResponse::NoContent().finish())
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct UpdateOrderStatusRequest {
    pub status: SalesOrderStatus,
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct UpdateQuotationStatusRequest {
    pub status: QuotationStatus,
}

/// Configure sales routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/v1/sales/orders")
            .route(web::get().to(get_sales_orders))
            .route(web::post().to(create_sales_order)),
    )
    .service(
        web::resource("/v1/sales/orders/status/{status}")
            .route(web::get().to(get_sales_orders_by_status)),
    )
    .service(
        web::resource("/v1/sales/orders/{id}")
            .route(web::get().to(get_sales_order))
            .route(web::delete().to(delete_sales_order)),
    )
    .service(
        web::resource("/v1/sales/orders/{id}/status")
            .route(web::put().to(update_sales_order_status)),
    )
    .service(
        web::resource("/v1/sales/quotations")
            .route(web::get().to(get_quotations))
            .route(web::post().to(create_quotation)),
    )
    .service(
        web::resource("/v1/sales/quotations/status/{status}")
            .route(web::get().to(get_quotations_by_status)),
    )
    .service(
        web::resource("/v1/sales/quotations/{id}")
            .route(web::get().to(get_quotation))
            .route(web::delete().to(delete_quotation)),
    )
    .service(
        web::resource("/v1/sales/quotations/{id}/status")
            .route(web::put().to(update_quotation_status)),
    )
    .service(
        web::resource("/v1/sales/quotations/{id}/convert")
            .route(web::post().to(convert_quotation_to_order)),
    );
}
