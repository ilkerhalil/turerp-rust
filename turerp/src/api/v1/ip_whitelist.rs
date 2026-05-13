//! IP whitelist API endpoints (admin only)

use actix_web::{delete, get, post, put, web, HttpResponse};

use crate::app::AppState;
use crate::domain::ip_whitelist::model::{
    CreateIpWhitelistEntry, IpWhitelistEntryResponse, UpdateIpWhitelistEntry,
};
use crate::error::ApiError;
use crate::middleware::auth::AdminUser;

/// List all IP whitelist entries for the current tenant (admin only)
#[utoipa::path(
    get,
    path = "/api/v1/ip-whitelist",
    responses(
        (status = 200, description = "List of IP whitelist entries"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin required"),
    ),
    tag = "IP Whitelist"
)]
#[get("/ip-whitelist")]
pub async fn list_ip_whitelist(
    _admin: AdminUser,
    state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
    let tenant_id = _admin.0.tenant_id;
    let entries = state
        .admin
        .ip_whitelist_service
        .list_entries(tenant_id)
        .await?;

    let responses: Vec<IpWhitelistEntryResponse> = entries
        .into_iter()
        .map(IpWhitelistEntryResponse::from)
        .collect();

    Ok(HttpResponse::Ok().json(responses))
}

/// Add a new IP whitelist entry (admin only)
#[utoipa::path(
    post,
    path = "/api/v1/ip-whitelist",
    request_body = CreateIpWhitelistEntry,
    responses(
        (status = 201, description = "IP whitelist entry created", body = IpWhitelistEntryResponse),
        (status = 400, description = "Validation failed"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin required"),
    ),
    tag = "IP Whitelist"
)]
#[post("/ip-whitelist")]
pub async fn add_ip_whitelist_entry(
    _admin: AdminUser,
    state: web::Data<AppState>,
    body: web::Json<CreateIpWhitelistEntry>,
) -> Result<HttpResponse, ApiError> {
    let tenant_id = _admin.0.tenant_id;
    let entry = state
        .admin
        .ip_whitelist_service
        .add_entry(tenant_id, body.into_inner())
        .await?;

    Ok(HttpResponse::Created().json(IpWhitelistEntryResponse::from(entry)))
}

/// Remove an IP whitelist entry (admin only)
#[utoipa::path(
    delete,
    path = "/api/v1/ip-whitelist/{id}",
    params(
        ("id" = i64, Path, description = "IP whitelist entry ID"),
    ),
    responses(
        (status = 204, description = "IP whitelist entry deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin required"),
        (status = 404, description = "IP whitelist entry not found"),
    ),
    tag = "IP Whitelist"
)]
#[delete("/ip-whitelist/{id}")]
pub async fn remove_ip_whitelist_entry(
    _admin: AdminUser,
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    let id = path.into_inner();
    let tenant_id = _admin.0.tenant_id;

    state
        .admin
        .ip_whitelist_service
        .remove_entry(tenant_id, id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

/// Update an IP whitelist entry (admin only)
#[utoipa::path(
    put,
    path = "/api/v1/ip-whitelist/{id}",
    params(
        ("id" = i64, Path, description = "IP whitelist entry ID"),
    ),
    request_body = UpdateIpWhitelistEntry,
    responses(
        (status = 200, description = "IP whitelist entry updated", body = IpWhitelistEntryResponse),
        (status = 400, description = "Validation failed"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin required"),
        (status = 404, description = "IP whitelist entry not found"),
    ),
    tag = "IP Whitelist"
)]
#[put("/ip-whitelist/{id}")]
pub async fn update_ip_whitelist_entry(
    _admin: AdminUser,
    state: web::Data<AppState>,
    path: web::Path<i64>,
    body: web::Json<UpdateIpWhitelistEntry>,
) -> Result<HttpResponse, ApiError> {
    let id = path.into_inner();
    let tenant_id = _admin.0.tenant_id;

    let entry = state
        .admin
        .ip_whitelist_service
        .update_entry(tenant_id, id, body.into_inner())
        .await?;

    Ok(HttpResponse::Ok().json(IpWhitelistEntryResponse::from(entry)))
}

/// Get a single IP whitelist entry (admin only)
#[utoipa::path(
    get,
    path = "/api/v1/ip-whitelist/{id}",
    params(
        ("id" = i64, Path, description = "IP whitelist entry ID"),
    ),
    responses(
        (status = 200, description = "IP whitelist entry found", body = IpWhitelistEntryResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin required"),
        (status = 404, description = "IP whitelist entry not found"),
    ),
    tag = "IP Whitelist"
)]
#[get("/ip-whitelist/{id}")]
pub async fn get_ip_whitelist_entry(
    _admin: AdminUser,
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    let id = path.into_inner();
    let tenant_id = _admin.0.tenant_id;

    let entry = state
        .admin
        .ip_whitelist_service
        .get_entry(tenant_id, id)
        .await?;

    Ok(HttpResponse::Ok().json(IpWhitelistEntryResponse::from(entry)))
}

/// Configure routes for IP whitelist API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(list_ip_whitelist)
        .service(add_ip_whitelist_entry)
        .service(remove_ip_whitelist_entry)
        .service(update_ip_whitelist_entry)
        .service(get_ip_whitelist_entry);
}
