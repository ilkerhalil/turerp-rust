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
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
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

/// Update file metadata request
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct UpdateFileRecord {
    pub filename: Option<String>,
    pub original_filename: Option<String>,
    pub content_type: Option<String>,
    pub entity_type: Option<String>,
    pub entity_id: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_file_record_creation() {
        let now = Utc::now();
        let file = FileRecord {
            id: 1,
            tenant_id: 10,
            filename: "doc_abc.pdf".to_string(),
            original_filename: "contract.pdf".to_string(),
            content_type: "application/pdf".to_string(),
            size_bytes: 1024,
            storage_path: "/storage/doc_abc.pdf".to_string(),
            storage_backend: "local".to_string(),
            checksum: "sha256:abc123".to_string(),
            uploaded_by: Some(5),
            entity_type: Some("invoice".to_string()),
            entity_id: Some(42),
            created_at: now,
            deleted_at: None,
            deleted_by: None,
        };

        assert_eq!(file.id, 1);
        assert_eq!(file.tenant_id, 10);
        assert_eq!(file.filename, "doc_abc.pdf");
        assert_eq!(file.original_filename, "contract.pdf");
        assert_eq!(file.content_type, "application/pdf");
        assert_eq!(file.size_bytes, 1024);
        assert_eq!(file.storage_path, "/storage/doc_abc.pdf");
        assert_eq!(file.storage_backend, "local");
        assert_eq!(file.checksum, "sha256:abc123");
        assert_eq!(file.uploaded_by, Some(5));
        assert_eq!(file.entity_type, Some("invoice".to_string()));
        assert_eq!(file.entity_id, Some(42));
        assert!(file.deleted_at.is_none());
        assert!(file.deleted_by.is_none());
    }

    #[test]
    fn test_create_file_record_creation() {
        let create = CreateFileRecord {
            tenant_id: 10,
            filename: "img_001.jpg".to_string(),
            original_filename: "photo.jpg".to_string(),
            content_type: "image/jpeg".to_string(),
            size_bytes: 2048,
            storage_path: "/storage/img_001.jpg".to_string(),
            storage_backend: "s3".to_string(),
            checksum: "sha256:def456".to_string(),
            uploaded_by: Some(3),
            entity_type: Some("product".to_string()),
            entity_id: Some(7),
        };

        assert_eq!(create.tenant_id, 10);
        assert_eq!(create.filename, "img_001.jpg");
        assert_eq!(create.original_filename, "photo.jpg");
        assert_eq!(create.content_type, "image/jpeg");
        assert_eq!(create.size_bytes, 2048);
        assert_eq!(create.storage_path, "/storage/img_001.jpg");
        assert_eq!(create.storage_backend, "s3");
        assert_eq!(create.checksum, "sha256:def456");
        assert_eq!(create.uploaded_by, Some(3));
        assert_eq!(create.entity_type, Some("product".to_string()));
        assert_eq!(create.entity_id, Some(7));
    }

    #[test]
    fn test_file_record_serialization() {
        let now = Utc::now();
        let file = FileRecord {
            id: 1,
            tenant_id: 10,
            filename: "report.pdf".to_string(),
            original_filename: "monthly_report.pdf".to_string(),
            content_type: "application/pdf".to_string(),
            size_bytes: 4096,
            storage_path: "/storage/report.pdf".to_string(),
            storage_backend: "local".to_string(),
            checksum: "sha256:hash99".to_string(),
            uploaded_by: None,
            entity_type: None,
            entity_id: None,
            created_at: now,
            deleted_at: None,
            deleted_by: None,
        };

        let json = serde_json::to_string(&file).unwrap();
        assert!(json.contains("report.pdf"));
        assert!(json.contains("monthly_report.pdf"));

        let deserialized: FileRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, file.id);
        assert_eq!(deserialized.filename, file.filename);
        assert_eq!(deserialized.content_type, file.content_type);
    }

    #[test]
    fn test_create_file_record_deserialization() {
        let json = r#"{"tenant_id":10,"filename":"notes.txt","original_filename":"my_notes.txt","content_type":"text/plain","size_bytes":128,"storage_path":"/storage/notes.txt","storage_backend":"local","checksum":"sha256:hash00","uploaded_by":null,"entity_type":null,"entity_id":null}"#;

        let deserialized: CreateFileRecord = serde_json::from_str(json).unwrap();
        assert_eq!(deserialized.tenant_id, 10);
        assert_eq!(deserialized.filename, "notes.txt");
        assert_eq!(deserialized.original_filename, "my_notes.txt");
        assert_eq!(deserialized.content_type, "text/plain");
        assert_eq!(deserialized.size_bytes, 128);
        assert_eq!(deserialized.storage_path, "/storage/notes.txt");
        assert_eq!(deserialized.storage_backend, "local");
        assert_eq!(deserialized.checksum, "sha256:hash00");
        assert!(deserialized.uploaded_by.is_none());
        assert!(deserialized.entity_type.is_none());
        assert!(deserialized.entity_id.is_none());
    }

    #[test]
    fn test_file_record_to_response() {
        let now = Utc::now();
        let file = FileRecord {
            id: 7,
            tenant_id: 10,
            filename: "doc.pdf".to_string(),
            original_filename: "original.pdf".to_string(),
            content_type: "application/pdf".to_string(),
            size_bytes: 512,
            storage_path: "/doc.pdf".to_string(),
            storage_backend: "s3".to_string(),
            checksum: "h1".to_string(),
            uploaded_by: Some(2),
            entity_type: Some("invoice".to_string()),
            entity_id: Some(99),
            created_at: now,
            deleted_at: None,
            deleted_by: None,
        };

        let resp: FileResponse = file.into();
        assert_eq!(resp.id, 7);
        assert_eq!(resp.filename, "doc.pdf");
        assert_eq!(resp.original_filename, "original.pdf");
        assert_eq!(resp.content_type, "application/pdf");
        assert_eq!(resp.size_bytes, 512);
        assert_eq!(resp.storage_backend, "s3");
        assert_eq!(resp.checksum, "h1");
        assert_eq!(resp.uploaded_by, Some(2));
        assert_eq!(resp.entity_type, Some("invoice".to_string()));
        assert_eq!(resp.entity_id, Some(99));
    }
}
