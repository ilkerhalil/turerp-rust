//! Assets API endpoints (v1)

use actix_web::{web, HttpResponse};

use crate::common::pagination::PaginationParams;
use crate::common::MessageResponse;
use crate::domain::assets::model::{
    AssetStatus, CreateAsset, CreateMaintenanceRecord, UpdateAsset,
};
use crate::domain::assets::service::AssetsService;
use crate::error::ApiResult;
use crate::i18n::{resolve, I18n, Locale};
use crate::middleware::{AdminUser, AuthUser};

/// Create asset (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/assets", tag = "Assets",
    request_body = CreateAsset,
    responses((status = 201, description = "Asset created"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_asset(
    admin_user: AdminUser,
    assets_service: web::Data<AssetsService>,
    payload: web::Json<CreateAsset>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    match assets_service.create_asset(create).await {
        Ok(asset) => Ok(HttpResponse::Created().json(asset)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get all assets
#[utoipa::path(
    get, path = "/api/v1/assets", tag = "Assets",
    responses((status = 200, description = "List of assets")),
    security(("bearer_auth" = []))
)]
pub async fn get_assets(
    auth_user: AuthUser,
    assets_service: web::Data<AssetsService>,
    query: web::Query<PaginationParams>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match assets_service
        .get_assets_by_tenant_paginated(auth_user.0.tenant_id, query.page, query.per_page)
        .await
    {
        Ok(result) => Ok(HttpResponse::Ok().json(result)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get asset by ID
#[utoipa::path(
    get, path = "/api/v1/assets/{id}", tag = "Assets",
    params(("id" = i64, Path, description = "Asset ID")),
    responses((status = 200, description = "Asset found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_asset(
    auth_user: AuthUser,
    assets_service: web::Data<AssetsService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match assets_service.get_asset(*path, auth_user.0.tenant_id).await {
        Ok(asset) => Ok(HttpResponse::Ok().json(asset)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get assets by status
#[utoipa::path(
    get, path = "/api/v1/assets/status/{status}", tag = "Assets",
    params(("status" = AssetStatus, Path, description = "Asset status")),
    responses((status = 200, description = "Assets by status")),
    security(("bearer_auth" = []))
)]
pub async fn get_assets_by_status(
    auth_user: AuthUser,
    assets_service: web::Data<AssetsService>,
    path: web::Path<AssetStatus>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match assets_service
        .get_assets_by_status(auth_user.0.tenant_id, path.into_inner())
        .await
    {
        Ok(assets) => Ok(HttpResponse::Ok().json(assets)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Update asset (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/assets/{id}", tag = "Assets",
    params(("id" = i64, Path, description = "Asset ID")),
    request_body = UpdateAsset,
    responses((status = 200, description = "Asset updated"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn update_asset(
    admin_user: AdminUser,
    assets_service: web::Data<AssetsService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateAsset>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match assets_service
        .update_asset(*path, admin_user.0.tenant_id, payload.into_inner())
        .await
    {
        Ok(asset) => Ok(HttpResponse::Ok().json(asset)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Update asset status (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/assets/{id}/status", tag = "Assets",
    params(("id" = i64, Path, description = "Asset ID")),
    request_body = UpdateAssetStatusRequest,
    responses((status = 200, description = "Status updated"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn update_asset_status(
    admin_user: AdminUser,
    assets_service: web::Data<AssetsService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateAssetStatusRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match assets_service
        .update_asset_status(*path, admin_user.0.tenant_id, payload.into_inner().status)
        .await
    {
        Ok(asset) => Ok(HttpResponse::Ok().json(asset)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Calculate depreciation
#[utoipa::path(
    post, path = "/api/v1/assets/{id}/depreciation", tag = "Assets",
    params(("id" = i64, Path, description = "Asset ID")),
    responses((status = 200, description = "Depreciation calculated")),
    security(("bearer_auth" = []))
)]
pub async fn calculate_depreciation(
    admin_user: AdminUser,
    assets_service: web::Data<AssetsService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match assets_service
        .calculate_depreciation(*path, admin_user.0.tenant_id)
        .await
    {
        Ok(asset) => Ok(HttpResponse::Ok().json(asset)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Record depreciation (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/assets/{id}/depreciation/record", tag = "Assets",
    params(("id" = i64, Path, description = "Asset ID")),
    request_body = RecordDepreciationRequest,
    responses((status = 200, description = "Depreciation recorded"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn record_depreciation(
    admin_user: AdminUser,
    assets_service: web::Data<AssetsService>,
    path: web::Path<i64>,
    payload: web::Json<RecordDepreciationRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match assets_service
        .record_depreciation(*path, admin_user.0.tenant_id, payload.amount)
        .await
    {
        Ok(asset) => Ok(HttpResponse::Ok().json(asset)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Dispose asset (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/assets/{id}/dispose", tag = "Assets",
    params(("id" = i64, Path, description = "Asset ID")),
    responses((status = 200, description = "Asset disposed"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn dispose_asset(
    admin_user: AdminUser,
    assets_service: web::Data<AssetsService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match assets_service
        .dispose_asset(*path, admin_user.0.tenant_id)
        .await
    {
        Ok(asset) => Ok(HttpResponse::Ok().json(asset)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Write off asset (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/assets/{id}/write-off", tag = "Assets",
    params(("id" = i64, Path, description = "Asset ID")),
    responses((status = 200, description = "Asset written off"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn write_off_asset(
    admin_user: AdminUser,
    assets_service: web::Data<AssetsService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match assets_service
        .write_off_asset(*path, admin_user.0.tenant_id)
        .await
    {
        Ok(asset) => Ok(HttpResponse::Ok().json(asset)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Start maintenance (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/assets/{id}/maintenance/start", tag = "Assets",
    params(("id" = i64, Path, description = "Asset ID")),
    responses((status = 200, description = "Maintenance started"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn start_maintenance(
    admin_user: AdminUser,
    assets_service: web::Data<AssetsService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match assets_service
        .start_maintenance(*path, admin_user.0.tenant_id)
        .await
    {
        Ok(asset) => Ok(HttpResponse::Ok().json(asset)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// End maintenance (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/assets/{id}/maintenance/end", tag = "Assets",
    params(("id" = i64, Path, description = "Asset ID")),
    request_body = EndMaintenanceRequest,
    responses((status = 200, description = "Maintenance ended"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn end_maintenance(
    admin_user: AdminUser,
    assets_service: web::Data<AssetsService>,
    path: web::Path<i64>,
    payload: web::Json<EndMaintenanceRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match assets_service
        .end_maintenance(*path, admin_user.0.tenant_id, payload.new_status)
        .await
    {
        Ok(asset) => Ok(HttpResponse::Ok().json(asset)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Soft delete an asset (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/assets/{id}", tag = "Assets",
    params(("id" = i64, Path, description = "Asset ID")),
    responses((status = 200, description = "Asset soft deleted", body = MessageResponse), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn soft_delete_asset(
    admin_user: AdminUser,
    assets_service: web::Data<AssetsService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let user_id: i64 = admin_user.0.sub.parse().unwrap_or(0);
    match assets_service
        .soft_delete_asset(*path, admin_user.0.tenant_id, user_id)
        .await
    {
        Ok(()) => {
            let msg = i18n.t(locale.as_str(), "asset.deleted");
            Ok(HttpResponse::Ok().json(MessageResponse { message: msg }))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Restore a soft-deleted asset (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/assets/{id}/restore", tag = "Assets",
    params(("id" = i64, Path, description = "Asset ID")),
    responses((status = 200, description = "Asset restored"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn restore_asset(
    admin_user: AdminUser,
    assets_service: web::Data<AssetsService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match assets_service
        .restore_asset(*path, admin_user.0.tenant_id)
        .await
    {
        Ok(asset) => Ok(HttpResponse::Ok().json(asset)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// List soft-deleted assets (requires admin role)
#[utoipa::path(
    get, path = "/api/v1/assets/deleted", tag = "Assets",
    responses((status = 200, description = "List of deleted assets")),
    security(("bearer_auth" = []))
)]
pub async fn list_deleted_assets(
    admin_user: AdminUser,
    assets_service: web::Data<AssetsService>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match assets_service
        .list_deleted_assets(admin_user.0.tenant_id)
        .await
    {
        Ok(assets) => Ok(HttpResponse::Ok().json(assets)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Hard delete (destroy) an asset (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/assets/{id}/destroy", tag = "Assets",
    params(("id" = i64, Path, description = "Asset ID")),
    responses((status = 204, description = "Asset permanently deleted"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn destroy_asset(
    admin_user: AdminUser,
    assets_service: web::Data<AssetsService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    assets_service
        .destroy_asset(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

/// Create maintenance record (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/assets/maintenance-records", tag = "Assets",
    request_body = CreateMaintenanceRecord,
    responses((status = 201, description = "Maintenance record created"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_maintenance_record(
    _admin_user: AdminUser,
    assets_service: web::Data<AssetsService>,
    payload: web::Json<CreateMaintenanceRecord>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match assets_service
        .create_maintenance_record(payload.into_inner())
        .await
    {
        Ok(record) => Ok(HttpResponse::Created().json(record)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get maintenance records for asset
#[utoipa::path(
    get, path = "/api/v1/assets/{id}/maintenance-records", tag = "Assets",
    params(("id" = i64, Path, description = "Asset ID")),
    responses((status = 200, description = "Maintenance records")),
    security(("bearer_auth" = []))
)]
pub async fn get_maintenance_records(
    _auth_user: AuthUser,
    assets_service: web::Data<AssetsService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match assets_service.get_maintenance_records(*path).await {
        Ok(records) => Ok(HttpResponse::Ok().json(records)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct UpdateAssetStatusRequest {
    pub status: AssetStatus,
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct RecordDepreciationRequest {
    pub amount: rust_decimal::Decimal,
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct EndMaintenanceRequest {
    pub new_status: AssetStatus,
}

/// Configure assets routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/v1/assets")
            .route(web::get().to(get_assets))
            .route(web::post().to(create_asset)),
    )
    .service(web::resource("/v1/assets/deleted").route(web::get().to(list_deleted_assets)))
    .service(web::resource("/v1/assets/status/{status}").route(web::get().to(get_assets_by_status)))
    // MUST register /maintenance-records BEFORE /{id} to avoid route shadowing
    .service(
        web::resource("/v1/assets/maintenance-records")
            .route(web::post().to(create_maintenance_record)),
    )
    .service(
        web::resource("/v1/assets/{id}")
            .route(web::get().to(get_asset))
            .route(web::put().to(update_asset))
            .route(web::delete().to(soft_delete_asset)),
    )
    .service(web::resource("/v1/assets/{id}/restore").route(web::put().to(restore_asset)))
    .service(web::resource("/v1/assets/{id}/destroy").route(web::delete().to(destroy_asset)))
    .service(web::resource("/v1/assets/{id}/status").route(web::put().to(update_asset_status)))
    .service(
        web::resource("/v1/assets/{id}/depreciation").route(web::post().to(calculate_depreciation)),
    )
    .service(
        web::resource("/v1/assets/{id}/depreciation/record")
            .route(web::post().to(record_depreciation)),
    )
    .service(web::resource("/v1/assets/{id}/dispose").route(web::post().to(dispose_asset)))
    .service(web::resource("/v1/assets/{id}/write-off").route(web::post().to(write_off_asset)))
    .service(
        web::resource("/v1/assets/{id}/maintenance/start").route(web::post().to(start_maintenance)),
    )
    .service(
        web::resource("/v1/assets/{id}/maintenance/end").route(web::post().to(end_maintenance)),
    )
    .service(
        web::resource("/v1/assets/{id}/maintenance-records")
            .route(web::get().to(get_maintenance_records)),
    );
}
