//! Barcode API endpoints

use actix_web::{delete, get, post, web, HttpResponse};
use serde::Deserialize;

use crate::app::AppState;
use crate::domain::barcode::model::{BarcodeResponse, GenerateBarcodeRequest};
use crate::error::ApiError;
use crate::middleware::auth::AdminUser;

/// Query params for listing barcodes
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ListBarcodesQuery {
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub per_page: Option<u32>,
}

/// Generate a barcode for an entity (admin only)
#[utoipa::path(
    post,
    path = "/api/v1/barcodes/generate",
    request_body = GenerateBarcodeRequest,
    responses(
        (status = 201, description = "Barcode generated", body = BarcodeResponse),
        (status = 400, description = "Validation failed"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin required"),
    ),
    tag = "barcodes"
)]
#[post("/barcodes/generate")]
pub async fn generate_barcode(
    _admin: AdminUser,
    state: web::Data<AppState>,
    body: web::Json<GenerateBarcodeRequest>,
) -> Result<HttpResponse, ApiError> {
    let tenant_id = _admin.0.tenant_id;
    let request = body.into_inner();

    let barcode = state
        .commerce
        .barcode_service
        .generate_barcode(tenant_id, request)
        .await?;

    Ok(HttpResponse::Created().json(BarcodeResponse::from(barcode)))
}

/// Get barcode for a specific entity
#[utoipa::path(
    get,
    path = "/api/v1/barcodes/{entity_type}/{entity_id}",
    params(
        ("entity_type" = String, Path, description = "Entity type (e.g. product, invoice)"),
        ("entity_id" = i64, Path, description = "Entity ID"),
    ),
    responses(
        (status = 200, description = "Barcode found", body = BarcodeResponse),
        (status = 404, description = "Barcode not found"),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "barcodes"
)]
#[get("/barcodes/{entity_type}/{entity_id}")]
pub async fn get_barcode_for_entity(
    state: web::Data<AppState>,
    path: web::Path<(String, i64)>,
    user: crate::middleware::auth::AuthUser,
) -> Result<HttpResponse, ApiError> {
    let (entity_type, entity_id) = path.into_inner();
    let tenant_id = user.0.tenant_id;

    let barcode = state
        .commerce
        .barcode_service
        .get_barcode(tenant_id, &entity_type, entity_id)
        .await?
        .ok_or_else(|| {
            ApiError::NotFound(format!(
                "Barcode for {} {} not found",
                entity_type, entity_id
            ))
        })?;

    Ok(HttpResponse::Ok().json(BarcodeResponse::from(barcode)))
}

/// Delete a barcode by ID (admin only)
#[utoipa::path(
    delete,
    path = "/api/v1/barcodes/{id}",
    params(("id" = i64, Path, description = "Barcode ID")),
    responses(
        (status = 204, description = "Barcode deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin required"),
        (status = 404, description = "Barcode not found"),
    ),
    tag = "barcodes"
)]
#[delete("/barcodes/{id}")]
pub async fn delete_barcode(
    _admin: AdminUser,
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    let id = path.into_inner();
    let tenant_id = _admin.0.tenant_id;

    state
        .commerce
        .barcode_service
        .delete_barcode(tenant_id, id)
        .await?;

    Ok(HttpResponse::NoContent().finish())
}

/// List all barcodes for the current tenant with pagination
#[utoipa::path(
    get,
    path = "/api/v1/barcodes",
    params(
        ("page" = Option<u32>, Query, description = "Page number"),
        ("per_page" = Option<u32>, Query, description = "Items per page"),
    ),
    responses(
        (status = 200, description = "List of barcodes"),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "barcodes"
)]
#[get("/barcodes")]
pub async fn list_barcodes(
    state: web::Data<AppState>,
    query: web::Query<ListBarcodesQuery>,
    user: crate::middleware::auth::AuthUser,
) -> Result<HttpResponse, ApiError> {
    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(50).min(200);
    let tenant_id = user.0.tenant_id;

    let result = state
        .commerce
        .barcode_service
        .list_barcodes(tenant_id, page, per_page)
        .await?;

    let responses: Vec<BarcodeResponse> = result
        .items
        .into_iter()
        .map(BarcodeResponse::from)
        .collect();

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "data": responses,
        "page": result.page,
        "per_page": result.per_page,
        "total": result.total,
        "total_pages": result.total_pages,
    })))
}

/// Configure routes for barcode API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(generate_barcode)
        .service(get_barcode_for_entity)
        .service(delete_barcode)
        .service(list_barcodes);
}
