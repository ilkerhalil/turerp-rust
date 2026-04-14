//! Assets API endpoints (v1)

use actix_web::{web, HttpResponse};

use crate::common::pagination::PaginationParams;
use crate::domain::assets::model::{
    AssetStatus, CreateAsset, CreateMaintenanceRecord, UpdateAsset,
};
use crate::domain::assets::service::AssetsService;
use crate::error::ApiResult;
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
) -> ApiResult<HttpResponse> {
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    let asset = assets_service.create_asset(create).await?;
    Ok(HttpResponse::Created().json(asset))
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
) -> ApiResult<HttpResponse> {
    let result = assets_service
        .get_assets_by_tenant_paginated(auth_user.0.tenant_id, query.page, query.per_page)
        .await?;
    Ok(HttpResponse::Ok().json(result))
}

/// Get asset by ID
#[utoipa::path(
    get, path = "/api/v1/assets/{id}", tag = "Assets",
    params(("id" = i64, Path, description = "Asset ID")),
    responses((status = 200, description = "Asset found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_asset(
    _auth_user: AuthUser,
    assets_service: web::Data<AssetsService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let asset = assets_service.get_asset(*path).await?;
    Ok(HttpResponse::Ok().json(asset))
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
) -> ApiResult<HttpResponse> {
    let assets = assets_service
        .get_assets_by_status(auth_user.0.tenant_id, path.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(assets))
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
    _admin_user: AdminUser,
    assets_service: web::Data<AssetsService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateAsset>,
) -> ApiResult<HttpResponse> {
    let asset = assets_service
        .update_asset(*path, payload.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(asset))
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
    _admin_user: AdminUser,
    assets_service: web::Data<AssetsService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateAssetStatusRequest>,
) -> ApiResult<HttpResponse> {
    let asset = assets_service
        .update_asset_status(*path, payload.into_inner().status)
        .await?;
    Ok(HttpResponse::Ok().json(asset))
}

/// Calculate depreciation
#[utoipa::path(
    post, path = "/api/v1/assets/{id}/depreciation", tag = "Assets",
    params(("id" = i64, Path, description = "Asset ID")),
    responses((status = 200, description = "Depreciation calculated")),
    security(("bearer_auth" = []))
)]
pub async fn calculate_depreciation(
    _admin_user: AdminUser,
    assets_service: web::Data<AssetsService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let asset = assets_service.calculate_depreciation(*path).await?;
    Ok(HttpResponse::Ok().json(asset))
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
    _admin_user: AdminUser,
    assets_service: web::Data<AssetsService>,
    path: web::Path<i64>,
    payload: web::Json<RecordDepreciationRequest>,
) -> ApiResult<HttpResponse> {
    let asset = assets_service
        .record_depreciation(*path, payload.amount)
        .await?;
    Ok(HttpResponse::Ok().json(asset))
}

/// Dispose asset (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/assets/{id}/dispose", tag = "Assets",
    params(("id" = i64, Path, description = "Asset ID")),
    responses((status = 200, description = "Asset disposed"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn dispose_asset(
    _admin_user: AdminUser,
    assets_service: web::Data<AssetsService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let asset = assets_service.dispose_asset(*path).await?;
    Ok(HttpResponse::Ok().json(asset))
}

/// Write off asset (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/assets/{id}/write-off", tag = "Assets",
    params(("id" = i64, Path, description = "Asset ID")),
    responses((status = 200, description = "Asset written off"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn write_off_asset(
    _admin_user: AdminUser,
    assets_service: web::Data<AssetsService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let asset = assets_service.write_off_asset(*path).await?;
    Ok(HttpResponse::Ok().json(asset))
}

/// Start maintenance (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/assets/{id}/maintenance/start", tag = "Assets",
    params(("id" = i64, Path, description = "Asset ID")),
    responses((status = 200, description = "Maintenance started"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn start_maintenance(
    _admin_user: AdminUser,
    assets_service: web::Data<AssetsService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let asset = assets_service.start_maintenance(*path).await?;
    Ok(HttpResponse::Ok().json(asset))
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
    _admin_user: AdminUser,
    assets_service: web::Data<AssetsService>,
    path: web::Path<i64>,
    payload: web::Json<EndMaintenanceRequest>,
) -> ApiResult<HttpResponse> {
    let asset = assets_service
        .end_maintenance(*path, payload.new_status)
        .await?;
    Ok(HttpResponse::Ok().json(asset))
}

/// Delete asset (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/assets/{id}", tag = "Assets",
    params(("id" = i64, Path, description = "Asset ID")),
    responses((status = 204, description = "Asset deleted"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn delete_asset(
    _admin_user: AdminUser,
    assets_service: web::Data<AssetsService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    assets_service.delete_asset(*path).await?;
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
) -> ApiResult<HttpResponse> {
    let record = assets_service
        .create_maintenance_record(payload.into_inner())
        .await?;
    Ok(HttpResponse::Created().json(record))
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
) -> ApiResult<HttpResponse> {
    let records = assets_service.get_maintenance_records(*path).await?;
    Ok(HttpResponse::Ok().json(records))
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
    .service(web::resource("/v1/assets/status/{status}").route(web::get().to(get_assets_by_status)))
    .service(
        web::resource("/v1/assets/{id}")
            .route(web::get().to(get_asset))
            .route(web::put().to(update_asset))
            .route(web::delete().to(delete_asset)),
    )
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
    )
    .service(
        web::resource("/v1/assets/maintenance-records")
            .route(web::post().to(create_maintenance_record)),
    );
}
