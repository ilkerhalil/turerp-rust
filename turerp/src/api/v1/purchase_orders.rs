//! Purchase Orders API endpoints (v1)

use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::common::pagination::PaginatedResult;
use crate::common::MessageResponse;
use crate::domain::purchase::{
    CreatePurchaseOrder, PurchaseOrder, PurchaseOrderResponse, PurchaseOrderStatus, PurchaseService,
};
use crate::error::{ApiError, ApiResult};
use crate::i18n::{resolve, I18n, Locale};
use crate::middleware::{AdminUser, AuthUser};

/// Query parameters for listing purchase orders (extends pagination with status filter)
#[derive(Debug, Deserialize, ToSchema)]
pub struct PurchaseOrderQueryParams {
    /// Filter by status
    pub status: Option<String>,
    /// Page number (1-based)
    #[serde(default = "default_page")]
    pub page: u32,
    /// Number of items per page
    #[serde(default = "default_per_page")]
    pub per_page: u32,
}

fn default_page() -> u32 {
    1
}

fn default_per_page() -> u32 {
    20
}

impl PurchaseOrderQueryParams {
    /// Validate and sanitize query parameters
    pub fn validate(&self) -> Result<(), String> {
        if self.page == 0 {
            return Err("page must be at least 1".to_string());
        }
        if self.per_page == 0 || self.per_page > 100 {
            return Err("per_page must be between 1 and 100".to_string());
        }
        Ok(())
    }
}

/// Create purchase order endpoint (requires authentication)
#[utoipa::path(
    post,
    path = "/api/v1/purchase-orders",
    tag = "Purchase Orders",
    request_body = CreatePurchaseOrder,
    responses(
        (status = 201, description = "Purchase order created successfully", body = PurchaseOrderResponse),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Not authenticated - missing or invalid JWT token")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn create_order(
    auth_user: AuthUser,
    service: web::Data<PurchaseService>,
    payload: web::Json<CreatePurchaseOrder>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let tenant_id = auth_user.0.tenant_id;
    let mut create = payload.into_inner();
    create.tenant_id = tenant_id;

    match service.create_purchase_order(create).await {
        Ok(order) => Ok(HttpResponse::Created().json(order)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get all purchase orders for tenant endpoint (requires authentication)
#[utoipa::path(
    get,
    path = "/api/v1/purchase-orders",
    tag = "Purchase Orders",
    params(
        ("status" = Option<String>, Query, description = "Filter by status"),
        ("page" = Option<u32>, Query, description = "Page number (default: 1)"),
        ("per_page" = Option<u32>, Query, description = "Items per page (default: 20, max: 100)")
    ),
    responses(
        (status = 200, description = "Paginated list of purchase orders", body = PurchaseOrderListResponse),
        (status = 401, description = "Not authenticated - missing or invalid JWT token")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_orders(
    auth_user: AuthUser,
    service: web::Data<PurchaseService>,
    query: web::Query<PurchaseOrderQueryParams>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    if let Err(e) = query.validate() {
        let err = ApiError::Validation(e);
        return Ok(err.to_http_response(i18n, locale.as_str()));
    }

    let tenant_id = auth_user.0.tenant_id;

    let items = if let Some(status_str) = &query.status {
        let status = parse_status(status_str)?;
        service.get_orders_by_status(tenant_id, status).await?
    } else {
        service.get_orders_by_tenant(tenant_id).await?
    };

    // Manual pagination since service does not expose paginated methods for orders
    let total = items.len() as u64;
    let offset = ((query.page.saturating_sub(1)) * query.per_page) as usize;
    let paginated_items: Vec<PurchaseOrder> = items
        .into_iter()
        .skip(offset)
        .take(query.per_page as usize)
        .collect();
    let total_pages = ((total as f64) / (query.per_page as f64)).ceil() as u32;
    let total_pages = if total_pages == 0 && total > 0 {
        1
    } else {
        total_pages
    };

    let result = PaginatedResult {
        items: paginated_items,
        page: query.page,
        per_page: query.per_page,
        total,
        total_pages,
    };

    Ok(HttpResponse::Ok().json(PurchaseOrderListResponse::from(result)))
}

/// Response for paginated purchase order list
#[derive(Debug, Serialize, ToSchema)]
pub struct PurchaseOrderListResponse {
    /// The items on this page
    pub items: Vec<PurchaseOrder>,
    /// Current page number (1-based)
    pub page: u32,
    /// Number of items per page
    pub per_page: u32,
    /// Total number of items
    pub total: u64,
    /// Total number of pages
    pub total_pages: u32,
}

impl From<PaginatedResult<PurchaseOrder>> for PurchaseOrderListResponse {
    fn from(result: PaginatedResult<PurchaseOrder>) -> Self {
        Self {
            items: result.items,
            page: result.page,
            per_page: result.per_page,
            total: result.total,
            total_pages: result.total_pages,
        }
    }
}

/// Parse status string to PurchaseOrderStatus
fn parse_status(status_str: &str) -> Result<PurchaseOrderStatus, crate::error::ApiError> {
    match status_str {
        "Draft" => Ok(PurchaseOrderStatus::Draft),
        "PendingApproval" => Ok(PurchaseOrderStatus::PendingApproval),
        "Approved" => Ok(PurchaseOrderStatus::Approved),
        "SentToVendor" => Ok(PurchaseOrderStatus::SentToVendor),
        "PartialReceived" => Ok(PurchaseOrderStatus::PartialReceived),
        "Received" => Ok(PurchaseOrderStatus::Received),
        "Cancelled" => Ok(PurchaseOrderStatus::Cancelled),
        "OnHold" => Ok(PurchaseOrderStatus::OnHold),
        _ => Err(crate::error::ApiError::Validation(format!(
            "Invalid status: {}. Valid values: Draft, PendingApproval, Approved, SentToVendor, PartialReceived, Received, Cancelled, OnHold",
            status_str
        ))),
    }
}

/// Get purchase order by ID endpoint (requires authentication)
#[utoipa::path(
    get,
    path = "/api/v1/purchase-orders/{id}",
    tag = "Purchase Orders",
    params(
        ("id" = i64, Path, description = "Purchase order ID")
    ),
    responses(
        (status = 200, description = "Purchase order found", body = PurchaseOrderResponse),
        (status = 401, description = "Not authenticated - missing or invalid JWT token"),
        (status = 404, description = "Purchase order not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_order(
    auth_user: AuthUser,
    service: web::Data<PurchaseService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match service
        .get_purchase_order(*path, auth_user.0.tenant_id)
        .await
    {
        Ok(order) => Ok(HttpResponse::Ok().json(order)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Update purchase order status endpoint (requires authentication)
#[utoipa::path(
    put,
    path = "/api/v1/purchase-orders/{id}/status",
    tag = "Purchase Orders",
    params(
        ("id" = i64, Path, description = "Purchase order ID")
    ),
    request_body = UpdateOrderStatusRequest,
    responses(
        (status = 200, description = "Purchase order status updated", body = PurchaseOrder),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Not authenticated - missing or invalid JWT token"),
        (status = 404, description = "Purchase order not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn update_order_status(
    _auth_user: AuthUser,
    service: web::Data<PurchaseService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateOrderStatusRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let status = parse_status(&payload.status)?;
    match service.update_order_status(*path, status).await {
        Ok(order) => Ok(HttpResponse::Ok().json(order)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Request body for updating purchase order status
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateOrderStatusRequest {
    /// New status value
    pub status: String,
}

/// Delete purchase order endpoint (requires admin role, soft delete)
#[utoipa::path(
    delete,
    path = "/api/v1/purchase-orders/{id}",
    tag = "Purchase Orders",
    params(
        ("id" = i64, Path, description = "Purchase order ID")
    ),
    responses(
        (status = 200, description = "Purchase order deleted", body = MessageResponse),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Purchase order not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn delete_order(
    admin_user: AdminUser,
    service: web::Data<PurchaseService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let deleted_by = admin_user.0.user_id()?;
    let tenant_id = admin_user.0.tenant_id;
    match service
        .soft_delete_order(*path, tenant_id, deleted_by)
        .await
    {
        Ok(()) => {
            let msg = i18n.t(locale.as_str(), "purchase.order.deleted");
            Ok(HttpResponse::Ok().json(MessageResponse { message: msg }))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Restore a soft-deleted purchase order (admin only)
#[utoipa::path(
    post,
    path = "/api/v1/purchase-orders/{id}/restore",
    tag = "Purchase Orders",
    params(
        ("id" = i64, Path, description = "Purchase order ID")
    ),
    responses(
        (status = 200, description = "Purchase order restored", body = PurchaseOrderResponse),
        (status = 404, description = "Not found or not deleted")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn restore_order(
    admin_user: AdminUser,
    service: web::Data<PurchaseService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let response = service
        .restore_order_response(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(response))
}

/// List soft-deleted purchase orders (admin only)
#[utoipa::path(
    get,
    path = "/api/v1/purchase-orders/deleted",
    tag = "Purchase Orders",
    responses(
        (status = 200, description = "List of deleted purchase orders")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn list_deleted_orders(
    admin_user: AdminUser,
    service: web::Data<PurchaseService>,
) -> ApiResult<HttpResponse> {
    let orders: Vec<_> = service
        .list_deleted_orders(admin_user.0.tenant_id)
        .await?
        .into_iter()
        .map(|o| {
            let lines: Vec<_> = Vec::new();
            PurchaseOrderResponse::from((o, lines))
        })
        .collect();
    Ok(HttpResponse::Ok().json(orders))
}

/// Permanently delete a purchase order (admin only)
#[utoipa::path(
    delete,
    path = "/api/v1/purchase-orders/{id}/destroy",
    tag = "Purchase Orders",
    params(
        ("id" = i64, Path, description = "Purchase order ID")
    ),
    responses(
        (status = 204, description = "Purchase order permanently deleted"),
        (status = 404, description = "Not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn destroy_order(
    admin_user: AdminUser,
    service: web::Data<PurchaseService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    service.destroy_order(*path, admin_user.0.tenant_id).await?;
    Ok(HttpResponse::NoContent().finish())
}

/// Configure purchase order routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/v1/purchase-orders")
            .route(web::get().to(get_orders))
            .route(web::post().to(create_order)),
    )
    .service(web::resource("/v1/purchase-orders/deleted").route(web::get().to(list_deleted_orders)))
    .service(
        web::resource("/v1/purchase-orders/{id}")
            .route(web::get().to(get_order))
            .route(web::delete().to(delete_order)),
    )
    .service(
        web::resource("/v1/purchase-orders/{id}/status").route(web::put().to(update_order_status)),
    )
    .service(web::resource("/v1/purchase-orders/{id}/restore").route(web::post().to(restore_order)))
    .service(
        web::resource("/v1/purchase-orders/{id}/destroy").route(web::delete().to(destroy_order)),
    );
}
