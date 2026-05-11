//! Import / Export API endpoints (v1)
//!
//! Provides endpoints for bulk importing Products, Cari accounts,
//! Chart of Accounts, and Stock Movements via CSV/JSON,
//! plus export with optional format selection.

use actix_multipart::Multipart;
use actix_web::{web, HttpResponse};
use futures::StreamExt;
use serde::Deserialize;

use crate::common::import::model::{EntityType, ImportFormat};
use crate::common::import::ImportService;
use crate::error::{ApiError, ApiResult};
use crate::middleware::AdminUser;

/// Upload and import a file
#[utoipa::path(
    post,
    path = "/api/v1/import/{entity}",
    tag = "Import",
    params(("entity" = String, Path, description = "Entity type: product, cari, chart_of_accounts, stock_movement")),
    responses(
        (status = 201, description = "Import completed", body = ImportResult),
        (status = 400, description = "Invalid file or entity type"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Admin role required")
    ),
    security(("bearer_auth" = []))
)]
pub async fn import_file(
    admin_user: AdminUser,
    path: web::Path<String>,
    mut payload: Multipart,
    import_service: web::Data<dyn ImportService>,
) -> ApiResult<HttpResponse> {
    let entity_type = path
        .into_inner()
        .parse::<EntityType>()
        .map_err(ApiError::Validation)?;

    let mut file_data = Vec::new();
    let mut format = ImportFormat::Csv;

    while let Some(item) = payload.next().await {
        let mut field =
            item.map_err(|e| ApiError::BadRequest(format!("Multipart error: {}", e)))?;
        let name = field.name().unwrap_or("").to_string();

        if name == "file" {
            while let Some(chunk) = field.next().await {
                let data = chunk.map_err(|e| ApiError::BadRequest(format!("Read error: {}", e)))?;
                file_data.extend_from_slice(&data);
            }
        } else if name == "format" {
            let mut buf = Vec::new();
            while let Some(chunk) = field.next().await {
                let data = chunk.map_err(|e| ApiError::BadRequest(format!("Read error: {}", e)))?;
                buf.extend_from_slice(&data);
            }
            if let Ok(s) = String::from_utf8(buf) {
                if let Ok(f) = s.trim().parse::<ImportFormat>() {
                    format = f;
                }
            }
        }
    }

    if file_data.is_empty() {
        return Err(ApiError::BadRequest("No file provided".to_string()));
    }

    let result = import_service
        .import(
            admin_user.0.tenant_id,
            entity_type,
            format,
            file_data,
            admin_user.0.user_id()?,
        )
        .await?;

    Ok(HttpResponse::Created().json(result))
}

/// Get import status and results by job ID
#[utoipa::path(
    get,
    path = "/api/v1/import/{job_id}/status",
    tag = "Import",
    params(("job_id" = i64, Path, description = "Import job ID")),
    responses(
        (status = 200, description = "Import status", body = ImportResult),
        (status = 404, description = "Job not found"),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn import_status(
    _admin_user: AdminUser,
    path: web::Path<i64>,
    import_service: web::Data<dyn ImportService>,
) -> ApiResult<HttpResponse> {
    let job_id = path.into_inner();
    match import_service.get_result(job_id) {
        Some(result) => Ok(HttpResponse::Ok().json(result)),
        None => Err(ApiError::NotFound(format!(
            "Import job {} not found",
            job_id
        ))),
    }
}

/// Get validation errors for an import job
#[utoipa::path(
    get,
    path = "/api/v1/import/{job_id}/errors",
    tag = "Import",
    params(("job_id" = i64, Path, description = "Import job ID")),
    responses(
        (status = 200, description = "Validation errors", body = Vec<crate::common::import::model::ImportError>),
        (status = 404, description = "Job not found"),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn import_errors(
    _admin_user: AdminUser,
    path: web::Path<i64>,
    import_service: web::Data<dyn ImportService>,
) -> ApiResult<HttpResponse> {
    let job_id = path.into_inner();
    match import_service.get_result(job_id) {
        Some(result) => Ok(HttpResponse::Ok().json(result.errors)),
        None => Err(ApiError::NotFound(format!(
            "Import job {} not found",
            job_id
        ))),
    }
}

/// Download an empty CSV/JSON template for an entity type
#[utoipa::path(
    get,
    path = "/api/v1/import/templates/{entity}",
    tag = "Import",
    params(
        ("entity" = String, Path, description = "Entity type"),
        ("format" = String, Query, description = "Template format: csv or json (default csv)")
    ),
    responses(
        (status = 200, description = "Template file"),
        (status = 400, description = "Invalid entity type or format"),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn download_template(
    _admin_user: AdminUser,
    path: web::Path<String>,
    query: web::Query<TemplateQuery>,
    import_service: web::Data<dyn ImportService>,
) -> ApiResult<HttpResponse> {
    let entity_type = path
        .into_inner()
        .parse::<EntityType>()
        .map_err(ApiError::Validation)?;
    let format = query
        .format
        .as_deref()
        .unwrap_or("csv")
        .parse::<ImportFormat>()
        .map_err(ApiError::Validation)?;

    let data = import_service.generate_template(entity_type, format)?;
    let content_type = match format {
        ImportFormat::Csv => "text/csv",
        ImportFormat::Json => "application/json",
    };
    let filename = format!("{}_template.{}", entity_type, format);

    Ok(HttpResponse::Ok()
        .content_type(content_type)
        .insert_header((
            actix_web::http::header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        ))
        .body(data))
}

/// Export entity data to CSV or JSON
#[utoipa::path(
    get,
    path = "/api/v1/export/{entity}",
    tag = "Export",
    params(
        ("entity" = String, Path, description = "Entity type"),
        ("format" = String, Query, description = "Export format: csv or json (default csv)"),
        ("from" = Option<String>, Query, description = "Optional date range start (ISO 8601)"),
        ("to" = Option<String>, Query, description = "Optional date range end (ISO 8601)")
    ),
    responses(
        (status = 200, description = "Exported data"),
        (status = 400, description = "Invalid entity type or format"),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn export_entity(
    admin_user: AdminUser,
    path: web::Path<String>,
    query: web::Query<ExportQuery>,
    import_service: web::Data<dyn ImportService>,
) -> ApiResult<HttpResponse> {
    let entity_type = path
        .into_inner()
        .parse::<EntityType>()
        .map_err(ApiError::Validation)?;
    let format = query
        .format
        .as_deref()
        .unwrap_or("csv")
        .parse::<ImportFormat>()
        .map_err(ApiError::Validation)?;

    let data = import_service
        .export(
            admin_user.0.tenant_id,
            entity_type,
            format,
            query.from.clone(),
            query.to.clone(),
        )
        .await?;

    let content_type = match format {
        ImportFormat::Csv => "text/csv",
        ImportFormat::Json => "application/json",
    };
    let filename = format!("{}_export.{}", entity_type, format);

    Ok(HttpResponse::Ok()
        .content_type(content_type)
        .insert_header((
            actix_web::http::header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        ))
        .body(data))
}

/// Query parameters for template download
#[derive(Debug, Deserialize)]
pub struct TemplateQuery {
    pub format: Option<String>,
}

/// Query parameters for export
#[derive(Debug, Deserialize)]
pub struct ExportQuery {
    pub format: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}

/// Configure import/export routes
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1/import")
            .route("/{entity}", web::post().to(import_file))
            .route("/{job_id}/status", web::get().to(import_status))
            .route("/{job_id}/errors", web::get().to(import_errors))
            .route("/templates/{entity}", web::get().to(download_template)),
    )
    .service(web::scope("/v1/export").route("/{entity}", web::get().to(export_entity)));
}
