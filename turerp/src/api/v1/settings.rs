//! Settings API endpoints

use actix_web::{delete, get, post, put, web, HttpResponse};
use serde::Deserialize;

use crate::app::AppState;
use crate::domain::settings::model::{
    BulkUpdateSettings, CreateSetting, SettingResponse, UpdateSetting,
};
use crate::error::ApiError;
use crate::middleware::auth::AdminUser;

/// Query params for listing settings
#[derive(Debug, Deserialize)]
pub struct ListSettingsQuery {
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub per_page: Option<u32>,
}

/// Create a new setting (admin only)
#[utoipa::path(
    post,
    path = "/api/v1/settings",
    request_body = CreateSetting,
    responses(
        (status = 201, description = "Setting created", body = SettingResponse),
        (status = 400, description = "Validation failed"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin required"),
        (status = 409, description = "Setting already exists"),
    ),
    tag = "settings"
)]
#[post("/settings")]
pub async fn create_setting(
    _admin: AdminUser,
    state: web::Data<AppState>,
    body: web::Json<CreateSetting>,
) -> Result<HttpResponse, ApiError> {
    let mut create = body.into_inner();
    create.tenant_id = _admin.0.tenant_id;

    let setting = state.settings_service.create(create).await?;
    Ok(HttpResponse::Created().json(SettingResponse::from(setting)))
}

/// List all settings for the current tenant
#[utoipa::path(
    get,
    path = "/api/v1/settings",
    params(
        ("group" = Option<String>, Query, description = "Filter by group"),
        ("page" = Option<u32>, Query, description = "Page number"),
        ("per_page" = Option<u32>, Query, description = "Items per page"),
    ),
    responses(
        (status = 200, description = "List of settings"),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "settings"
)]
#[get("/settings")]
pub async fn list_settings(
    state: web::Data<AppState>,
    query: web::Query<ListSettingsQuery>,
    user: crate::middleware::auth::AuthUser,
) -> Result<HttpResponse, ApiError> {
    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(50).min(200);

    let result = state
        .settings_service
        .list_paginated(user.0.tenant_id, query.group.as_deref(), page, per_page)
        .await?;

    let responses: Vec<SettingResponse> = result
        .items
        .into_iter()
        .map(SettingResponse::from)
        .collect();

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "data": responses,
        "page": result.page,
        "per_page": result.per_page,
        "total": result.total,
        "total_pages": result.total_pages,
    })))
}

/// Get a setting by key
#[utoipa::path(
    get,
    path = "/api/v1/settings/{key}",
    params(("key" = String, Path, description = "Setting key")),
    responses(
        (status = 200, description = "Setting found", body = SettingResponse),
        (status = 404, description = "Setting not found"),
    ),
    tag = "settings"
)]
#[get("/settings/{key}")]
pub async fn get_setting_by_key(
    state: web::Data<AppState>,
    path: web::Path<String>,
    user: crate::middleware::auth::AuthUser,
) -> Result<HttpResponse, ApiError> {
    let key = path.into_inner();
    let setting = state
        .settings_service
        .get_by_key(user.0.tenant_id, &key)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Setting '{}' not found", key)))?;

    Ok(HttpResponse::Ok().json(SettingResponse::from(setting)))
}

/// Update a setting by ID (admin only)
#[utoipa::path(
    put,
    path = "/api/v1/settings/{id}",
    params(("id" = i64, Path, description = "Setting ID")),
    request_body = UpdateSetting,
    responses(
        (status = 200, description = "Setting updated", body = SettingResponse),
        (status = 400, description = "Validation failed"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin required"),
        (status = 404, description = "Setting not found"),
    ),
    tag = "settings"
)]
#[put("/settings/{id}")]
pub async fn update_setting(
    _admin: AdminUser,
    state: web::Data<AppState>,
    path: web::Path<i64>,
    body: web::Json<UpdateSetting>,
) -> Result<HttpResponse, ApiError> {
    let id = path.into_inner();
    let tenant_id = _admin.0.tenant_id;
    let update = body.into_inner();

    let setting = state.settings_service.update(tenant_id, id, update).await?;
    Ok(HttpResponse::Ok().json(SettingResponse::from(setting)))
}

/// Bulk update settings by key (admin only)
#[utoipa::path(
    patch,
    path = "/api/v1/settings/bulk",
    request_body = BulkUpdateSettings,
    responses(
        (status = 200, description = "Settings updated"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin required"),
    ),
    tag = "settings"
)]
#[post("/settings/bulk")]
pub async fn bulk_update_settings(
    _admin: AdminUser,
    state: web::Data<AppState>,
    body: web::Json<BulkUpdateSettings>,
) -> Result<HttpResponse, ApiError> {
    let tenant_id = _admin.0.tenant_id;
    let bulk = body.into_inner();

    let updated = state
        .settings_service
        .bulk_update(tenant_id, bulk.updates)
        .await?;

    let responses: Vec<SettingResponse> = updated.into_iter().map(SettingResponse::from).collect();

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "updated": responses.len(),
        "data": responses,
    })))
}

/// Delete a setting by ID (admin only)
#[utoipa::path(
    delete,
    path = "/api/v1/settings/{id}",
    params(("id" = i64, Path, description = "Setting ID")),
    responses(
        (status = 204, description = "Setting deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin required"),
        (status = 404, description = "Setting not found"),
    ),
    tag = "settings"
)]
#[delete("/settings/{id}")]
pub async fn delete_setting(
    _admin: AdminUser,
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    let id = path.into_inner();
    let tenant_id = _admin.0.tenant_id;

    state.settings_service.delete(tenant_id, id).await?;
    Ok(HttpResponse::NoContent().finish())
}

/// Soft delete a setting by ID (admin only)
#[utoipa::path(
    delete,
    path = "/api/v1/settings/{id}/soft",
    params(("id" = i64, Path, description = "Setting ID")),
    responses(
        (status = 204, description = "Setting soft deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin required"),
        (status = 404, description = "Setting not found"),
        (status = 409, description = "Setting already deleted"),
    ),
    tag = "settings"
)]
#[delete("/settings/{id}/soft")]
pub async fn soft_delete_setting(
    _admin: AdminUser,
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    let id = path.into_inner();
    let tenant_id = _admin.0.tenant_id;
    let deleted_by = _admin.0.sub.parse::<i64>().unwrap_or(0);

    state
        .settings_service
        .soft_delete_setting(tenant_id, id, deleted_by)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

/// Restore a soft-deleted setting (admin only)
#[utoipa::path(
    post,
    path = "/api/v1/settings/{id}/restore",
    params(("id" = i64, Path, description = "Setting ID")),
    responses(
        (status = 204, description = "Setting restored"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin required"),
        (status = 404, description = "Deleted setting not found"),
    ),
    tag = "settings"
)]
#[post("/settings/{id}/restore")]
pub async fn restore_setting(
    _admin: AdminUser,
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    let id = path.into_inner();
    let tenant_id = _admin.0.tenant_id;

    state
        .settings_service
        .restore_setting(tenant_id, id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

/// List deleted settings for the current tenant (admin only)
#[utoipa::path(
    get,
    path = "/api/v1/settings/deleted",
    responses(
        (status = 200, description = "List of deleted settings"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin required"),
    ),
    tag = "settings"
)]
#[get("/settings/deleted")]
pub async fn list_deleted_settings(
    _admin: AdminUser,
    state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
    let tenant_id = _admin.0.tenant_id;
    let settings = state
        .settings_service
        .list_deleted_settings(tenant_id)
        .await?;
    let responses: Vec<SettingResponse> = settings.into_iter().map(SettingResponse::from).collect();
    Ok(HttpResponse::Ok().json(responses))
}

/// Permanently destroy a soft-deleted setting (admin only)
#[utoipa::path(
    delete,
    path = "/api/v1/settings/{id}/destroy",
    params(("id" = i64, Path, description = "Setting ID")),
    responses(
        (status = 204, description = "Setting permanently destroyed"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin required"),
        (status = 404, description = "Deleted setting not found"),
    ),
    tag = "settings"
)]
#[delete("/settings/{id}/destroy")]
pub async fn destroy_setting(
    _admin: AdminUser,
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    let id = path.into_inner();
    let tenant_id = _admin.0.tenant_id;

    state
        .settings_service
        .destroy_setting(tenant_id, id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

/// Seed default settings for the current tenant (admin only)
#[utoipa::path(
    post,
    path = "/api/v1/settings/seed",
    responses(
        (status = 201, description = "Default settings seeded"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin required"),
    ),
    tag = "settings"
)]
#[post("/settings/seed")]
pub async fn seed_settings(
    _admin: AdminUser,
    state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
    let tenant_id = _admin.0.tenant_id;
    let created = state.settings_service.seed_defaults(tenant_id).await?;

    let responses: Vec<SettingResponse> = created.into_iter().map(SettingResponse::from).collect();

    Ok(HttpResponse::Created().json(serde_json::json!({
        "message": "Default settings seeded",
        "created": responses.len(),
        "data": responses,
    })))
}

/// Configure routes for settings API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(create_setting)
        .service(list_settings)
        .service(get_setting_by_key)
        .service(update_setting)
        .service(bulk_update_settings)
        .service(delete_setting)
        .service(soft_delete_setting)
        .service(restore_setting)
        .service(list_deleted_settings)
        .service(destroy_setting)
        .service(seed_settings);
}
