//! File domain model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// File metadata record for database and API
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FileRecord {
    pub id: i64,
    pub tenant_id: i64,
    pub filename: String,
    pub original_filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub storage_path: String,
    pub storage_backend: String,
    pub checksum: String,
    pub uploaded_by: Option<i64>,
    pub entity_type: Option<String>,
    pub entity_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

/// Response DTO for file metadata
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FileResponse {
    pub id: i64,
    pub filename: String,
    pub original_filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub storage_backend: String,
    pub checksum: String,
    pub uploaded_by: Option<i64>,
    pub entity_type: Option<String>,
    pub entity_id: Option<i64>,
    pub created_at: DateTime<Utc>,
}

impl From<FileRecord> for FileResponse {
    fn from(record: FileRecord) -> Self {
        Self {
            id: record.id,
            filename: record.filename,
            original_filename: record.original_filename,
            content_type: record.content_type,
            size_bytes: record.size_bytes,
            storage_backend: record.storage_backend,
            checksum: record.checksum,
            uploaded_by: record.uploaded_by,
            entity_type: record.entity_type,
            entity_id: record.entity_id,
            created_at: record.created_at,
        }
    }
}

/// Create file metadata request
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateFileRecord {
    pub tenant_id: i64,
    pub filename: String,
    pub original_filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub storage_path: String,
    pub storage_backend: String,
    pub checksum: String,
    pub uploaded_by: Option<i64>,
    pub entity_type: Option<String>,
    pub entity_id: Option<i64>,
}
