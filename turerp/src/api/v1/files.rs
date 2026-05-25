//! File Storage API endpoints (v1)

use actix_multipart::Multipart;
use actix_web::{web, HttpResponse};
use futures::StreamExt;
use serde::Deserialize;

use crate::common::file_storage::FileUpload;
use crate::common::pagination::{default_page, default_per_page};
use crate::error::{ApiError, ApiResult};
use crate::middleware::AuthUser;

const MAX_FILE_SIZE: usize = 50 * 1024 * 1024; // 50 MB

const ALLOWED_EXTENSIONS: &[&str] = &["pdf", "jpg", "jpeg", "png", "csv", "xlsx", "txt"];
const ALLOWED_CONTENT_TYPES: &[&str] = &[
    "application/pdf",
    "image/jpeg",
    "image/png",
    "text/csv",
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    "text/plain",
];

/// Sanitize a filename to prevent path traversal and injection attacks.
fn sanitize_filename(filename: &str) -> String {
    let mut result = filename.replace(['\x00', '\r', '\n', '/', '\\'], "_");

    // Prevent path traversal sequences
    result = result.replace("..", "__");

    // Replace all remaining non-alphanumeric except . _ -
    result
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '.' || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Extract the lowercase extension from a filename.
fn get_extension(filename: &str) -> Option<String> {
    filename.rsplit('.').next().map(|ext| ext.to_lowercase())
}

/// Check if the filename has an allowed extension.
fn is_allowed_extension(filename: &str) -> bool {
    get_extension(filename)
        .map(|ext| ALLOWED_EXTENSIONS.contains(&ext.as_str()))
        .unwrap_or(false)
}

/// Validate that the first bytes of the file match the declared content type.
fn validate_magic_bytes(content_type: &str, data: &[u8]) -> bool {
    if data.is_empty() {
        return false;
    }

    match content_type {
        "application/pdf" => data.starts_with(b"%PDF"),
        "image/jpeg" => data.len() >= 3 && data[0] == 0xFF && data[1] == 0xD8 && data[2] == 0xFF,
        "image/png" => data.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]),
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => {
            data.starts_with(&[0x50, 0x4B, 0x03, 0x04])
        }
        "text/plain" | "text/csv" => {
            let check_len = data.len().min(1024);
            !data[..check_len].contains(&0x00)
        }
        _ => false,
    }
}

/// Query parameters for listing files
#[derive(Debug, Deserialize)]
pub struct ListFilesQuery {
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_per_page")]
    pub per_page: u32,
    pub entity_type: Option<String>,
    pub entity_id: Option<i64>,
}

/// Upload a file (multipart form data)
#[utoipa::path(
    post,
    path = "/api/v1/files",
    tag = "Files",
    responses(
        (status = 201, description = "File uploaded successfully"),
        (status = 400, description = "Invalid file upload"),
        (status = 413, description = "File too large"),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn upload_file(
    auth_user: AuthUser,
    mut payload: Multipart,
    storage: web::Data<dyn crate::common::file_storage::FileStorage>,
) -> ApiResult<HttpResponse> {
    let tenant_id = auth_user.0.tenant_id;
    let uploaded_by = auth_user.0.sub.parse::<i64>().ok();

    let mut file_data = Vec::new();
    let mut filename = String::new();
    let mut content_type = String::from("application/octet-stream");
    let mut entity_type = None;
    let mut entity_id = None;

    while let Some(item) = payload.next().await {
        let mut field =
            item.map_err(|e| ApiError::BadRequest(format!("Multipart error: {}", e)))?;
        let name = field.name().unwrap_or("").to_string();

        if name == "file" {
            filename = field
                .content_disposition()
                .and_then(|cd| cd.get_filename())
                .unwrap_or("unknown")
                .to_string();
            if let Some(ct) = field.content_type() {
                content_type = ct.essence_str().to_string();
            }
            while let Some(chunk) = field.next().await {
                let data = chunk.map_err(|e| ApiError::BadRequest(format!("Read error: {}", e)))?;
                file_data.extend_from_slice(&data);
                if file_data.len() > MAX_FILE_SIZE {
                    return Err(ApiError::PayloadTooLarge(
                        "File size exceeds 50MB limit".to_string(),
                    ));
                }
            }
        } else if name == "entity_type" {
            let mut buf = Vec::new();
            while let Some(chunk) = field.next().await {
                let data = chunk.map_err(|e| ApiError::BadRequest(format!("Read error: {}", e)))?;
                buf.extend_from_slice(&data);
            }
            entity_type = String::from_utf8(buf).ok().map(|s| s.trim().to_string());
        } else if name == "entity_id" {
            let mut buf = Vec::new();
            while let Some(chunk) = field.next().await {
                let data = chunk.map_err(|e| ApiError::BadRequest(format!("Read error: {}", e)))?;
                buf.extend_from_slice(&data);
            }
            if let Ok(s) = String::from_utf8(buf) {
                entity_id = s.trim().parse().ok();
            }
        }
    }

    if filename.is_empty() || file_data.is_empty() {
        return Err(ApiError::BadRequest("No file provided".to_string()));
    }

    // Validate extension
    if !is_allowed_extension(&filename) {
        return Err(ApiError::Validation(format!(
            "File extension not allowed. Allowed: {}",
            ALLOWED_EXTENSIONS.join(", ")
        )));
    }

    // Validate content type
    if !ALLOWED_CONTENT_TYPES.contains(&content_type.as_str()) {
        return Err(ApiError::Validation(format!(
            "Content type not allowed. Allowed: {}",
            ALLOWED_CONTENT_TYPES.join(", ")
        )));
    }

    // Validate magic bytes
    if !validate_magic_bytes(&content_type, &file_data) {
        return Err(ApiError::Validation(
            "File content does not match declared type".to_string(),
        ));
    }

    // Sanitize filename before storage
    let filename = sanitize_filename(&filename);

    let upload = FileUpload {
        tenant_id,
        filename,
        content_type,
        data: file_data,
        uploaded_by,
        entity_type,
        entity_id,
    };

    match storage.upload(upload).await {
        Ok(meta) => Ok(HttpResponse::Created().json(meta)),
        Err(e) => Err(ApiError::Internal(e)),
    }
}

/// Get file metadata by ID
#[utoipa::path(
    get,
    path = "/api/v1/files/{id}",
    tag = "Files",
    params(("id" = i64, Path, description = "File ID")),
    responses(
        (status = 200, description = "File metadata found"),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "File not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_file(
    auth_user: AuthUser,
    storage: web::Data<dyn crate::common::file_storage::FileStorage>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let tenant_id = auth_user.0.tenant_id;
    match storage.get_metadata(tenant_id, *path).await {
        Ok(Some(meta)) => Ok(HttpResponse::Ok().json(meta)),
        Ok(None) => Err(ApiError::NotFound(format!("File {} not found", path))),
        Err(e) => Err(ApiError::Internal(e)),
    }
}

/// Download a file by ID
#[utoipa::path(
    get,
    path = "/api/v1/files/{id}/download",
    tag = "Files",
    params(("id" = i64, Path, description = "File ID")),
    responses(
        (status = 200, description = "File content"),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "File not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn download_file(
    auth_user: AuthUser,
    storage: web::Data<dyn crate::common::file_storage::FileStorage>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let tenant_id = auth_user.0.tenant_id;
    let file_id = *path;

    let meta = storage
        .get_metadata(tenant_id, file_id)
        .await
        .map_err(ApiError::Internal)?
        .ok_or_else(|| ApiError::NotFound(format!("File {} not found", file_id)))?;

    let data = storage
        .download(tenant_id, file_id)
        .await
        .map_err(ApiError::Internal)?;

    let safe_filename = sanitize_filename(&meta.original_filename);
    Ok(HttpResponse::Ok()
        .content_type(meta.content_type)
        .insert_header((
            actix_web::http::header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", safe_filename),
        ))
        .body(data))
}

/// Get a presigned URL for downloading a file (S3 only)
#[utoipa::path(
    get,
    path = "/api/v1/files/{id}/presigned",
    tag = "Files",
    params(("id" = i64, Path, description = "File ID")),
    responses(
        (status = 200, description = "Presigned URL generated"),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "File not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn presigned_url(
    auth_user: AuthUser,
    storage: web::Data<dyn crate::common::file_storage::FileStorage>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let tenant_id = auth_user.0.tenant_id;
    match storage.presigned_url(tenant_id, *path, 3600).await {
        Ok(url) => Ok(HttpResponse::Ok().json(url)),
        Err(e) if e.contains("not supported") => Err(ApiError::BadRequest(
            "Presigned URLs are only available for S3 storage".to_string(),
        )),
        Err(e) => Err(ApiError::Internal(e)),
    }
}

/// Soft delete a file
#[utoipa::path(
    delete,
    path = "/api/v1/files/{id}",
    tag = "Files",
    params(("id" = i64, Path, description = "File ID")),
    responses(
        (status = 204, description = "File deleted"),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "File not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_file(
    auth_user: AuthUser,
    storage: web::Data<dyn crate::common::file_storage::FileStorage>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let tenant_id = auth_user.0.tenant_id;
    match storage.delete(tenant_id, *path).await {
        Ok(()) => Ok(HttpResponse::NoContent().finish()),
        Err(e) if e.contains("not found") => {
            Err(ApiError::NotFound(format!("File {} not found", path)))
        }
        Err(e) => Err(ApiError::Internal(e)),
    }
}

/// List files for a tenant
#[utoipa::path(
    get,
    path = "/api/v1/files",
    tag = "Files",
    responses(
        (status = 200, description = "List of files"),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_files(
    auth_user: AuthUser,
    storage: web::Data<dyn crate::common::file_storage::FileStorage>,
    query: web::Query<ListFilesQuery>,
) -> ApiResult<HttpResponse> {
    let tenant_id = auth_user.0.tenant_id;
    let offset = (query.page.saturating_sub(1)) * query.per_page;
    let mut files = storage
        .list_files(tenant_id, query.per_page, offset)
        .await
        .map_err(ApiError::Internal)?;

    // Client-side filter by entity_type / entity_id
    if let (Some(entity_type), Some(entity_id)) = (&query.entity_type, query.entity_id) {
        files.retain(|f| {
            f.entity_type.as_deref() == Some(entity_type) && f.entity_id == Some(entity_id)
        });
    }

    Ok(HttpResponse::Ok().json(files))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename_basic() {
        assert_eq!(sanitize_filename("report.pdf"), "report.pdf");
    }

    #[test]
    fn test_sanitize_filename_null_bytes() {
        assert_eq!(sanitize_filename("file\x00name.txt"), "file_name.txt");
    }

    #[test]
    fn test_sanitize_filename_path_traversal() {
        assert_eq!(
            sanitize_filename("../../../etc/passwd"),
            "_________etc_passwd"
        );
    }

    #[test]
    fn test_sanitize_filename_backslash_traversal() {
        assert_eq!(
            sanitize_filename("..\\..\\windows\\system32"),
            "______windows_system32"
        );
    }

    #[test]
    fn test_sanitize_filename_special_chars() {
        assert_eq!(sanitize_filename("file<name>.txt"), "file_name_.txt");
    }

    #[test]
    fn test_sanitize_filename_newlines() {
        assert_eq!(sanitize_filename("file\nname\r.txt"), "file_name_.txt");
    }

    #[test]
    fn test_sanitize_filename_spaces() {
        assert_eq!(sanitize_filename("my document.pdf"), "my_document.pdf");
    }

    #[test]
    fn test_is_allowed_extension() {
        assert!(is_allowed_extension("document.pdf"));
        assert!(is_allowed_extension("photo.JPG"));
        assert!(is_allowed_extension("data.CSV"));
        assert!(!is_allowed_extension("script.exe"));
        assert!(!is_allowed_extension("page.html"));
        assert!(!is_allowed_extension("archive.tar.gz"));
    }

    #[test]
    fn test_validate_magic_bytes_pdf() {
        let data = b"%PDF-1.4\n1 0 obj\n<<\n/Type /Catalog\n>>\nendobj\n";
        assert!(validate_magic_bytes("application/pdf", data));
    }

    #[test]
    fn test_validate_magic_bytes_jpeg() {
        let data = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
        assert!(validate_magic_bytes("image/jpeg", &data));
    }

    #[test]
    fn test_validate_magic_bytes_png() {
        let data = vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
        ];
        assert!(validate_magic_bytes("image/png", &data));
    }

    #[test]
    fn test_validate_magic_bytes_xlsx() {
        let data = vec![0x50, 0x4B, 0x03, 0x04, 0x14, 0x00, 0x06, 0x00];
        assert!(validate_magic_bytes(
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            &data
        ));
    }

    #[test]
    fn test_validate_magic_bytes_txt() {
        let data = b"Hello, world!\n";
        assert!(validate_magic_bytes("text/plain", data));
    }

    #[test]
    fn test_validate_magic_bytes_csv() {
        let data = b"name,age\nAlice,30\n";
        assert!(validate_magic_bytes("text/csv", data));
    }

    #[test]
    fn test_validate_magic_bytes_rejects_binary_as_text() {
        let data = vec![0x00, 0x01, 0x02, 0x03];
        assert!(!validate_magic_bytes("text/plain", &data));
        assert!(!validate_magic_bytes("text/csv", &data));
    }

    #[test]
    fn test_validate_magic_bytes_rejects_mismatched_type() {
        let data = b"%PDF-1.4";
        assert!(!validate_magic_bytes("image/png", data));
        assert!(!validate_magic_bytes("image/jpeg", data));
        assert!(!validate_magic_bytes(
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            data,
        ));
    }

    #[test]
    fn test_validate_magic_bytes_empty() {
        assert!(!validate_magic_bytes("text/plain", b""));
    }

    #[test]
    fn test_validate_magic_bytes_rejects_exe_as_non_text() {
        let data = vec![0x4D, 0x5A]; // Windows EXE magic bytes
        assert!(!validate_magic_bytes("application/pdf", &data));
        assert!(!validate_magic_bytes("image/jpeg", &data));
        assert!(!validate_magic_bytes("image/png", &data));
    }
}

/// Configure file storage routes
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1/files")
            .route("", web::post().to(upload_file))
            .route("", web::get().to(list_files))
            .route("/{id}", web::get().to(get_file))
            .route("/{id}/download", web::get().to(download_file))
            .route("/{id}/presigned", web::get().to(presigned_url))
            .route("/{id}", web::delete().to(delete_file)),
    );
}
