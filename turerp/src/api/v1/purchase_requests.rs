//! Purchase Requests API endpoints (v1)

use actix_web::{web, HttpResponse};
use serde::Deserialize;

use crate::domain::purchase::{
    CreatePurchaseRequest, PurchaseRequestStatus, PurchaseService, UpdatePurchaseRequest,
};
use crate::error::ApiResult;
use crate::middleware::{AdminUser, AuthUser};

/// Query parameters for listing purchase requests
#[derive(Debug, Deserialize)]
pub struct QueryParams {
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

impl QueryParams {
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
) -> ApiResult<HttpResponse> {
    let tenant_id = auth_user.0.tenant_id;
    let mut create = payload.into_inner();
    create.tenant_id = tenant_id;

    let request = service.create_purchase_request(create).await?;
    Ok(HttpResponse::Created().json(request))
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
        (status = 200, description = "List of purchase requests", body = Vec<PurchaseRequest>),
        (status = 401, description = "Not authenticated - missing or invalid JWT token")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_requests(
    auth_user: AuthUser,
    service: web::Data<PurchaseService>,
    query: web::Query<QueryParams>,
) -> ApiResult<HttpResponse> {
    query
        .validate()
        .map_err(crate::error::ApiError::Validation)?;

    let tenant_id = auth_user.0.tenant_id;

    let requests = if let Some(status_str) = &query.status {
        let status = parse_status(status_str)?;
        service.get_requests_by_status(tenant_id, status).await?
    } else {
        service.get_requests_by_tenant(tenant_id).await?
    };

    // TODO: Implement pagination at repository level
    // For now, return all results (this should be fixed for production)
    Ok(HttpResponse::Ok().json(requests))
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
    _auth_user: AuthUser,
    service: web::Data<PurchaseService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let request = service.get_purchase_request(*path).await?;
    Ok(HttpResponse::Ok().json(request))
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
) -> ApiResult<HttpResponse> {
    let request = service
        .update_purchase_request(*path, payload.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(request))
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
    _auth_user: AuthUser,
    service: web::Data<PurchaseService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let request = service
        .update_request_status(*path, PurchaseRequestStatus::PendingApproval)
        .await?;
    Ok(HttpResponse::Ok().json(request))
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
    _admin_user: AdminUser,
    service: web::Data<PurchaseService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let request = service
        .update_request_status(*path, PurchaseRequestStatus::Approved)
        .await?;
    Ok(HttpResponse::Ok().json(request))
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
    _admin_user: AdminUser,
    service: web::Data<PurchaseService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let request = service
        .update_request_status(*path, PurchaseRequestStatus::Rejected)
        .await?;
    Ok(HttpResponse::Ok().json(request))
}

/// Delete purchase request endpoint (requires authentication)
#[utoipa::path(
    delete,
    path = "/api/v1/purchase-requests/{id}",
    tag = "Purchase Requests",
    params(
        ("id" = i64, Path, description = "Purchase request ID")
    ),
    responses(
        (status = 204, description = "Purchase request deleted"),
        (status = 401, description = "Not authenticated - missing or invalid JWT token"),
        (status = 404, description = "Purchase request not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn delete_request(
    _auth_user: AuthUser,
    service: web::Data<PurchaseService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    service.delete_purchase_request(*path).await?;
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
        web::resource("/v1/purchase-requests/{id}")
            .route(web::get().to(get_request))
            .route(web::put().to(update_request))
            .route(web::delete().to(delete_request)),
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
