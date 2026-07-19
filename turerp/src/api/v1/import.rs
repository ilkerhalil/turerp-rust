//! Import / Export API endpoints (v1)
//!
//! Provides endpoints for bulk importing Products, Cari accounts,
//! Chart of Accounts, and Stock Movements via CSV/JSON,
//! plus export with optional format selection.

use actix_multipart::Multipart;
use actix_web::{web, HttpResponse};
use futures::StreamExt;
use serde::Deserialize;

use crate::common::file_storage::{FileStorage, FileUpload};
use crate::common::import::model::{EntityType, ImportFormat};
use crate::common::import::ImportService;
use crate::error::{ApiError, ApiResult};
use crate::middleware::AdminUser;

/// Maximum allowed import file size (100 MB)
const MAX_IMPORT_FILE_SIZE: usize = 100 * 1024 * 1024;

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
        (status = 403, description = "Admin role required"),
        (status = 413, description = "File exceeds maximum size of 100MB")
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
    let mut company_id: i64 = 1; // Default company for backward compatibility

    while let Some(item) = payload.next().await {
        let mut field =
            item.map_err(|e| ApiError::BadRequest(format!("Multipart error: {}", e)))?;
        let name = field.name().unwrap_or("").to_string();

        if name == "file" {
            while let Some(chunk) = field.next().await {
                let data = chunk.map_err(|e| ApiError::BadRequest(format!("Read error: {}", e)))?;
                if file_data.len() + data.len() > MAX_IMPORT_FILE_SIZE {
                    return Err(ApiError::PayloadTooLarge(format!(
                        "File exceeds maximum size of {}MB",
                        MAX_IMPORT_FILE_SIZE / 1024 / 1024
                    )));
                }
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
        } else if name == "company_id" {
            let mut buf = Vec::new();
            while let Some(chunk) = field.next().await {
                let data = chunk.map_err(|e| ApiError::BadRequest(format!("Read error: {}", e)))?;
                buf.extend_from_slice(&data);
            }
            if let Ok(s) = String::from_utf8(buf) {
                if let Ok(id) = s.trim().parse::<i64>() {
                    company_id = id;
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
            company_id,
            entity_type,
            format,
            file_data,
            admin_user.0.user_id()?,
            None,
        )
        .await?;

    Ok(HttpResponse::Created().json(result))
}

/// Upload a file and schedule an asynchronous import job.
#[utoipa::path(
    post,
    path = "/api/v1/import/{entity}/async",
    tag = "Import",
    params(("entity" = String, Path, description = "Entity type: product, cari, chart_of_accounts, stock_movement")),
    responses(
        (status = 202, description = "Import job scheduled", body = serde_json::Value),
        (status = 400, description = "Invalid file or entity type"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Admin role required"),
        (status = 413, description = "File exceeds maximum size of 100MB")
    ),
    security(("bearer_auth" = []))
)]
pub async fn import_file_async(
    admin_user: AdminUser,
    path: web::Path<String>,
    mut payload: Multipart,
    import_service: web::Data<dyn ImportService>,
    file_storage: web::Data<dyn FileStorage>,
) -> ApiResult<HttpResponse> {
    let entity_type = path
        .into_inner()
        .parse::<EntityType>()
        .map_err(ApiError::Validation)?;

    let mut file_data = Vec::new();
    let mut format = ImportFormat::Csv;
    let mut company_id: i64 = 1;
    let mut filename = format!("import_{}.csv", entity_type);

    while let Some(item) = payload.next().await {
        let mut field =
            item.map_err(|e| ApiError::BadRequest(format!("Multipart error: {}", e)))?;
        let name = field.name().unwrap_or("").to_string();

        if name == "file" {
            if let Some(fname) = field.content_disposition().and_then(|cd| cd.get_filename()) {
                filename = fname.to_string();
            }
            while let Some(chunk) = field.next().await {
                let data = chunk.map_err(|e| ApiError::BadRequest(format!("Read error: {}", e)))?;
                if file_data.len() + data.len() > MAX_IMPORT_FILE_SIZE {
                    return Err(ApiError::PayloadTooLarge(format!(
                        "File exceeds maximum size of {}MB",
                        MAX_IMPORT_FILE_SIZE / 1024 / 1024
                    )));
                }
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
        } else if name == "company_id" {
            let mut buf = Vec::new();
            while let Some(chunk) = field.next().await {
                let data = chunk.map_err(|e| ApiError::BadRequest(format!("Read error: {}", e)))?;
                buf.extend_from_slice(&data);
            }
            if let Ok(s) = String::from_utf8(buf) {
                if let Ok(id) = s.trim().parse::<i64>() {
                    company_id = id;
                }
            }
        }
    }

    if file_data.is_empty() {
        return Err(ApiError::BadRequest("No file provided".to_string()));
    }

    let content_type = match format {
        ImportFormat::Csv => "text/csv",
        ImportFormat::Json => "application/json",
    };

    let upload = FileUpload {
        tenant_id: admin_user.0.tenant_id,
        filename: filename.clone(),
        content_type: content_type.to_string(),
        data: file_data,
        uploaded_by: Some(admin_user.0.user_id()?),
        entity_type: Some(entity_type.to_string()),
        entity_id: None,
    };

    let file_meta = file_storage
        .upload(upload)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to store file: {}", e)))?;

    let job_id = import_service
        .schedule_import(
            admin_user.0.tenant_id,
            company_id,
            entity_type,
            format,
            file_meta.id,
        )
        .await?;

    Ok(HttpResponse::Accepted().json(serde_json::json!({
        "job_id": job_id,
        "file_id": file_meta.id,
        "status": "scheduled"
    })))
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
    admin_user: AdminUser,
    path: web::Path<i64>,
    import_service: web::Data<dyn ImportService>,
) -> ApiResult<HttpResponse> {
    let job_id = path.into_inner();
    match import_service.get_result(job_id, admin_user.0.tenant_id) {
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
    admin_user: AdminUser,
    path: web::Path<i64>,
    import_service: web::Data<dyn ImportService>,
) -> ApiResult<HttpResponse> {
    let job_id = path.into_inner();
    match import_service.get_result(job_id, admin_user.0.tenant_id) {
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
            .route("/{entity}/async", web::post().to(import_file_async))
            .route("/{job_id}/status", web::get().to(import_status))
            .route("/{job_id}/errors", web::get().to(import_errors))
            .route("/templates/{entity}", web::get().to(download_template)),
    )
    .service(web::scope("/v1/export").route("/{entity}", web::get().to(export_entity)));
}
