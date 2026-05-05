//! Goods Receipts API endpoints (v1)

use actix_web::{web, HttpResponse};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::common::MessageResponse;
use crate::domain::purchase::{CreateGoodsReceipt, GoodsReceiptStatus, PurchaseService};
use crate::error::ApiResult;
use crate::i18n::{resolve, I18n, Locale};
use crate::middleware::{AdminUser, AuthUser};

/// Request body for updating goods receipt status
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateReceiptStatusRequest {
    pub status: GoodsReceiptStatus,
}

/// Create goods receipt endpoint (requires authentication)
#[utoipa::path(
    post,
    path = "/api/v1/goods-receipts",
    tag = "Goods Receipts",
    request_body = CreateGoodsReceipt,
    responses(
        (status = 201, description = "Goods receipt created successfully", body = crate::domain::purchase::model::GoodsReceiptResponse),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Not authenticated - missing or invalid JWT token")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn create_receipt(
    auth_user: AuthUser,
    service: web::Data<PurchaseService>,
    payload: web::Json<CreateGoodsReceipt>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let tenant_id = auth_user.0.tenant_id;
    let create = payload.into_inner();

    match service.create_goods_receipt(create, tenant_id).await {
        Ok(receipt) => Ok(HttpResponse::Created().json(receipt)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get goods receipt by ID endpoint (requires authentication)
#[utoipa::path(
    get,
    path = "/api/v1/goods-receipts/{id}",
    tag = "Goods Receipts",
    params(
        ("id" = i64, Path, description = "Goods receipt ID")
    ),
    responses(
        (status = 200, description = "Goods receipt found", body = crate::domain::purchase::model::GoodsReceiptResponse),
        (status = 401, description = "Not authenticated - missing or invalid JWT token"),
        (status = 404, description = "Goods receipt not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_receipt(
    auth_user: AuthUser,
    service: web::Data<PurchaseService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match service
        .get_goods_receipt(*path, auth_user.0.tenant_id)
        .await
    {
        Ok(receipt) => Ok(HttpResponse::Ok().json(receipt)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get goods receipts by purchase order ID endpoint (requires authentication)
#[utoipa::path(
    get,
    path = "/api/v1/goods-receipts/order/{order_id}",
    tag = "Goods Receipts",
    params(
        ("order_id" = i64, Path, description = "Purchase order ID")
    ),
    responses(
        (status = 200, description = "List of goods receipts for order", body = Vec<crate::domain::purchase::model::GoodsReceipt>),
        (status = 401, description = "Not authenticated - missing or invalid JWT token")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_receipts_by_order(
    _auth_user: AuthUser,
    service: web::Data<PurchaseService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let receipts = service.get_receipts_by_order(*path).await?;
    Ok(HttpResponse::Ok().json(receipts))
}

/// Update goods receipt status endpoint (requires authentication)
#[utoipa::path(
    put,
    path = "/api/v1/goods-receipts/{id}/status",
    tag = "Goods Receipts",
    params(
        ("id" = i64, Path, description = "Goods receipt ID")
    ),
    request_body = UpdateReceiptStatusRequest,
    responses(
        (status = 200, description = "Goods receipt status updated", body = crate::domain::purchase::model::GoodsReceipt),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Not authenticated - missing or invalid JWT token"),
        (status = 404, description = "Goods receipt not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn update_receipt_status(
    _auth_user: AuthUser,
    service: web::Data<PurchaseService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateReceiptStatusRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match service
        .update_receipt_status(*path, payload.status.clone())
        .await
    {
        Ok(receipt) => Ok(HttpResponse::Ok().json(receipt)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Delete goods receipt endpoint (requires admin role, soft delete)
#[utoipa::path(
    delete,
    path = "/api/v1/goods-receipts/{id}",
    tag = "Goods Receipts",
    params(
        ("id" = i64, Path, description = "Goods receipt ID")
    ),
    responses(
        (status = 200, description = "Goods receipt deleted", body = MessageResponse),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Goods receipt not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn delete_receipt(
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
        .soft_delete_receipt(*path, tenant_id, deleted_by)
        .await
    {
        Ok(()) => {
            let msg = i18n.t(locale.as_str(), "goods_receipt.deleted");
            Ok(HttpResponse::Ok().json(MessageResponse { message: msg }))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Restore a soft-deleted goods receipt (admin only)
#[utoipa::path(
    post,
    path = "/api/v1/goods-receipts/{id}/restore",
    tag = "Goods Receipts",
    params(
        ("id" = i64, Path, description = "Goods receipt ID")
    ),
    responses(
        (status = 200, description = "Goods receipt restored", body = crate::domain::purchase::model::GoodsReceiptResponse),
        (status = 404, description = "Not found or not deleted")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn restore_receipt(
    admin_user: AdminUser,
    service: web::Data<PurchaseService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let response = service
        .restore_receipt_response(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(response))
}

/// List soft-deleted goods receipts (admin only)
#[utoipa::path(
    get,
    path = "/api/v1/goods-receipts/deleted",
    tag = "Goods Receipts",
    responses(
        (status = 200, description = "List of deleted goods receipts", body = Vec<crate::domain::purchase::model::GoodsReceipt>)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn list_deleted_receipts(
    admin_user: AdminUser,
    service: web::Data<PurchaseService>,
) -> ApiResult<HttpResponse> {
    let receipts = service
        .list_deleted_receipts(admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(receipts))
}

/// Permanently delete a goods receipt (admin only)
#[utoipa::path(
    delete,
    path = "/api/v1/goods-receipts/{id}/destroy",
    tag = "Goods Receipts",
    params(
        ("id" = i64, Path, description = "Goods receipt ID")
    ),
    responses(
        (status = 204, description = "Goods receipt permanently deleted"),
        (status = 404, description = "Not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn destroy_receipt(
    admin_user: AdminUser,
    service: web::Data<PurchaseService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    service
        .destroy_receipt(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

/// Configure goods receipt routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/v1/goods-receipts").route(web::post().to(create_receipt)))
        .service(
            web::resource("/v1/goods-receipts/deleted").route(web::get().to(list_deleted_receipts)),
        )
        .service(
            web::resource("/v1/goods-receipts/{id}")
                .route(web::get().to(get_receipt))
                .route(web::delete().to(delete_receipt)),
        )
        .service(
            web::resource("/v1/goods-receipts/{id}/status")
                .route(web::put().to(update_receipt_status)),
        )
        .service(
            web::resource("/v1/goods-receipts/{id}/restore").route(web::post().to(restore_receipt)),
        )
        .service(
            web::resource("/v1/goods-receipts/{id}/destroy")
                .route(web::delete().to(destroy_receipt)),
        )
        .service(
            web::resource("/v1/goods-receipts/order/{order_id}")
                .route(web::get().to(get_receipts_by_order)),
        );
}
