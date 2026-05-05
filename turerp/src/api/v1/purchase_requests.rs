//! Purchase Requests API endpoints (v1)

use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::common::pagination::PaginatedResult;
use crate::common::MessageResponse;
use crate::domain::purchase::{
    CreatePurchaseRequest, PurchaseRequest, PurchaseRequestResponse, PurchaseRequestStatus,
    PurchaseService, UpdatePurchaseRequest,
};
use crate::error::{ApiError, ApiResult};
use crate::i18n::{resolve, I18n, Locale};
use crate::middleware::{AdminUser, AuthUser};

/// Query parameters for listing purchase requests (extends pagination with status filter)
#[derive(Debug, Deserialize, ToSchema)]
pub struct PurchaseRequestQueryParams {
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

impl PurchaseRequestQueryParams {
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

/// Create purchase request endpoint (requires authentication)
#[utoipa::path(
    post,
    path = "/api/v1/purchase-requests",
    tag = "Purchase Requests",
    request_body = CreatePurchaseRequest,
    responses(
        (status = 201, description = "Purchase request created successfully", body = PurchaseRequestResponse),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Not authenticated - missing or invalid JWT token")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn create_request(
    auth_user: AuthUser,
    service: web::Data<PurchaseService>,
    payload: web::Json<CreatePurchaseRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let tenant_id = auth_user.0.tenant_id;
    let mut create = payload.into_inner();
    create.tenant_id = tenant_id;

    match service.create_purchase_request(create).await {
        Ok(request) => Ok(HttpResponse::Created().json(request)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get all purchase requests for tenant endpoint (requires authentication)
#[utoipa::path(
    get,
    path = "/api/v1/purchase-requests",
    tag = "Purchase Requests",
    params(
        ("status" = Option<String>, Query, description = "Filter by status"),
        ("page" = Option<u32>, Query, description = "Page number (default: 1)"),
        ("per_page" = Option<u32>, Query, description = "Items per page (default: 20, max: 100)")
    ),
    responses(
        (status = 200, description = "Paginated list of purchase requests", body = PurchaseRequestListResponse),
        (status = 401, description = "Not authenticated - missing or invalid JWT token")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_requests(
    auth_user: AuthUser,
    service: web::Data<PurchaseService>,
    query: web::Query<PurchaseRequestQueryParams>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    if let Err(e) = query.validate() {
        let err = ApiError::Validation(e);
        return Ok(err.to_http_response(i18n, locale.as_str()));
    }

    let tenant_id = auth_user.0.tenant_id;

    let result = if let Some(status_str) = &query.status {
        let status = parse_status(status_str)?;
        service
            .get_requests_by_status_paginated(tenant_id, status, query.page, query.per_page)
            .await?
    } else {
        service
            .get_requests_by_tenant_paginated(tenant_id, query.page, query.per_page)
            .await?
    };

    Ok(HttpResponse::Ok().json(PurchaseRequestListResponse::from(result)))
}

/// Response for paginated purchase request list
#[derive(Debug, Serialize, ToSchema)]
pub struct PurchaseRequestListResponse {
    /// The items on this page
    pub items: Vec<PurchaseRequest>,
    /// Current page number (1-based)
    pub page: u32,
    /// Number of items per page
    pub per_page: u32,
    /// Total number of items
    pub total: u64,
    /// Total number of pages
    pub total_pages: u32,
}

impl From<PaginatedResult<PurchaseRequest>> for PurchaseRequestListResponse {
    fn from(result: PaginatedResult<PurchaseRequest>) -> Self {
        Self {
            items: result.items,
            page: result.page,
            per_page: result.per_page,
            total: result.total,
            total_pages: result.total_pages,
        }
    }
}

/// Parse status string to PurchaseRequestStatus
fn parse_status(status_str: &str) -> Result<PurchaseRequestStatus, crate::error::ApiError> {
    match status_str {
        "Draft" => Ok(PurchaseRequestStatus::Draft),
        "PendingApproval" => Ok(PurchaseRequestStatus::PendingApproval),
        "Approved" => Ok(PurchaseRequestStatus::Approved),
        "Rejected" => Ok(PurchaseRequestStatus::Rejected),
        "ConvertedToOrder" => Ok(PurchaseRequestStatus::ConvertedToOrder),
        _ => Err(crate::error::ApiError::Validation(format!(
            "Invalid status: {}. Valid values: Draft, PendingApproval, Approved, Rejected, ConvertedToOrder",
            status_str
        ))),
    }
}

/// Get purchase request by ID endpoint (requires authentication)
#[utoipa::path(
    get,
    path = "/api/v1/purchase-requests/{id}",
    tag = "Purchase Requests",
    params(
        ("id" = i64, Path, description = "Purchase request ID")
    ),
    responses(
        (status = 200, description = "Purchase request found", body = PurchaseRequestResponse),
        (status = 401, description = "Not authenticated - missing or invalid JWT token"),
        (status = 404, description = "Purchase request not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_request(
    auth_user: AuthUser,
    service: web::Data<PurchaseService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match service
        .get_purchase_request(*path, auth_user.0.tenant_id)
        .await
    {
        Ok(request) => Ok(HttpResponse::Ok().json(request)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Update purchase request endpoint (requires authentication)
#[utoipa::path(
    put,
    path = "/api/v1/purchase-requests/{id}",
    tag = "Purchase Requests",
    params(
        ("id" = i64, Path, description = "Purchase request ID")
    ),
    request_body = UpdatePurchaseRequest,
    responses(
        (status = 200, description = "Purchase request updated", body = PurchaseRequest),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Not authenticated - missing or invalid JWT token"),
        (status = 404, description = "Purchase request not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn update_request(
    _auth_user: AuthUser,
    service: web::Data<PurchaseService>,
    path: web::Path<i64>,
    payload: web::Json<UpdatePurchaseRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match service
        .update_purchase_request(*path, payload.into_inner())
        .await
    {
        Ok(request) => Ok(HttpResponse::Ok().json(request)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Submit purchase request for approval endpoint (requires authentication)
#[utoipa::path(
    post,
    path = "/api/v1/purchase-requests/{id}/submit",
    tag = "Purchase Requests",
    params(
        ("id" = i64, Path, description = "Purchase request ID")
    ),
    responses(
        (status = 200, description = "Purchase request submitted for approval", body = PurchaseRequest),
        (status = 401, description = "Not authenticated - missing or invalid JWT token"),
        (status = 404, description = "Purchase request not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn submit_request(
    auth_user: AuthUser,
    service: web::Data<PurchaseService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match service
        .update_request_status(
            *path,
            PurchaseRequestStatus::PendingApproval,
            auth_user.0.tenant_id,
        )
        .await
    {
        Ok(request) => Ok(HttpResponse::Ok().json(request)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Approve purchase request endpoint (requires admin role)
#[utoipa::path(
    post,
    path = "/api/v1/purchase-requests/{id}/approve",
    tag = "Purchase Requests",
    params(
        ("id" = i64, Path, description = "Purchase request ID")
    ),
    responses(
        (status = 200, description = "Purchase request approved", body = PurchaseRequest),
        (status = 401, description = "Not authenticated - missing or invalid JWT token"),
        (status = 403, description = "Not authorized - admin role required"),
        (status = 404, description = "Purchase request not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn approve_request(
    admin_user: AdminUser,
    service: web::Data<PurchaseService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match service
        .update_request_status(
            *path,
            PurchaseRequestStatus::Approved,
            admin_user.0.tenant_id,
        )
        .await
    {
        Ok(request) => Ok(HttpResponse::Ok().json(request)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Reject purchase request endpoint (requires admin role)
#[utoipa::path(
    post,
    path = "/api/v1/purchase-requests/{id}/reject",
    tag = "Purchase Requests",
    params(
        ("id" = i64, Path, description = "Purchase request ID")
    ),
    responses(
        (status = 200, description = "Purchase request rejected", body = PurchaseRequest),
        (status = 401, description = "Not authenticated - missing or invalid JWT token"),
        (status = 403, description = "Not authorized - admin role required"),
        (status = 404, description = "Purchase request not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn reject_request(
    admin_user: AdminUser,
    service: web::Data<PurchaseService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match service
        .update_request_status(
            *path,
            PurchaseRequestStatus::Rejected,
            admin_user.0.tenant_id,
        )
        .await
    {
        Ok(request) => Ok(HttpResponse::Ok().json(request)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Delete purchase request endpoint (requires admin role, soft delete)
#[utoipa::path(
    delete,
    path = "/api/v1/purchase-requests/{id}",
    tag = "Purchase Requests",
    params(
        ("id" = i64, Path, description = "Purchase request ID")
    ),
    responses(
        (status = 200, description = "Purchase request deleted", body = MessageResponse),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Purchase request not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn delete_request(
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
        .soft_delete_request(*path, tenant_id, deleted_by)
        .await
    {
        Ok(()) => {
            let msg = i18n.t(locale.as_str(), "purchase.request.deleted");
            Ok(HttpResponse::Ok().json(MessageResponse { message: msg }))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Restore a soft-deleted purchase request (admin only)
#[utoipa::path(
    put,
    path = "/api/v1/purchase-requests/{id}/restore",
    tag = "Purchase Requests",
    params(
        ("id" = i64, Path, description = "Purchase request ID")
    ),
    responses(
        (status = 200, description = "Purchase request restored", body = PurchaseRequestResponse),
        (status = 404, description = "Not found or not deleted")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn restore_request(
    admin_user: AdminUser,
    service: web::Data<PurchaseService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let response = service
        .restore_request_response(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(response))
}

/// List soft-deleted purchase requests (admin only)
#[utoipa::path(
    get,
    path = "/api/v1/purchase-requests/deleted",
    tag = "Purchase Requests",
    responses(
        (status = 200, description = "List of deleted purchase requests")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn list_deleted_requests(
    admin_user: AdminUser,
    service: web::Data<PurchaseService>,
) -> ApiResult<HttpResponse> {
    let requests: Vec<_> = service
        .list_deleted_requests(admin_user.0.tenant_id)
        .await?
        .into_iter()
        .map(|r| {
            let lines: Vec<_> = Vec::new();
            PurchaseRequestResponse::from((r, lines))
        })
        .collect();
    Ok(HttpResponse::Ok().json(requests))
}

/// Permanently delete a purchase request (admin only)
#[utoipa::path(
    delete,
    path = "/api/v1/purchase-requests/{id}/destroy",
    tag = "Purchase Requests",
    params(
        ("id" = i64, Path, description = "Purchase request ID")
    ),
    responses(
        (status = 204, description = "Purchase request permanently deleted"),
        (status = 404, description = "Not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn destroy_request(
    admin_user: AdminUser,
    service: web::Data<PurchaseService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    service
        .destroy_request(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

/// Configure purchase request routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/v1/purchase-requests")
            .route(web::get().to(get_requests))
            .route(web::post().to(create_request)),
    )
    .service(
        web::resource("/v1/purchase-requests/deleted").route(web::get().to(list_deleted_requests)),
    )
    .service(
        web::resource("/v1/purchase-requests/{id}")
            .route(web::get().to(get_request))
            .route(web::put().to(update_request))
            .route(web::delete().to(delete_request)),
    )
    .service(
        web::resource("/v1/purchase-requests/{id}/restore").route(web::put().to(restore_request)),
    )
    .service(
        web::resource("/v1/purchase-requests/{id}/destroy")
            .route(web::delete().to(destroy_request)),
    )
    .service(
        web::resource("/v1/purchase-requests/{id}/submit").route(web::post().to(submit_request)),
    )
    .service(
        web::resource("/v1/purchase-requests/{id}/approve").route(web::post().to(approve_request)),
    )
    .service(
        web::resource("/v1/purchase-requests/{id}/reject").route(web::post().to(reject_request)),
    );
}
