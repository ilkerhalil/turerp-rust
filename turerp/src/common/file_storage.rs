//! File upload and document management service
//!
//! Provides a `FileStorage` trait with local filesystem and S3-compatible
//! backends. Supports presigned URL generation, file metadata tracking,
//! and tenant-isolated storage.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

/// File metadata record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub id: i64,
    pub tenant_id: i64,
    pub filename: String,
    pub original_filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub storage_path: String,
    pub storage_backend: StorageBackend,
    pub checksum: String,
    pub uploaded_by: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

/// Storage backend type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageBackend {
    Local,
    S3,
}

/// File upload request
#[derive(Debug, Clone)]
pub struct FileUpload {
    pub tenant_id: i64,
    pub filename: String,
    pub content_type: String,
    pub data: Vec<u8>,
    pub uploaded_by: Option<i64>,
}

/// Presigned URL result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresignedUrl {
    pub url: String,
    pub expires_at: DateTime<Utc>,
}

/// File storage trait
#[async_trait::async_trait]
pub trait FileStorage: Send + Sync {
    /// Upload a file
    async fn upload(&self, upload: FileUpload) -> Result<FileMetadata, String>;

    /// Download a file by ID
    async fn download(&self, tenant_id: i64, file_id: i64) -> Result<Vec<u8>, String>;

    /// Get file metadata
    async fn get_metadata(
        &self,
        tenant_id: i64,
        file_id: i64,
    ) -> Result<Option<FileMetadata>, String>;

    /// Delete a file (soft delete)
    async fn delete(&self, tenant_id: i64, file_id: i64) -> Result<(), String>;

    /// List files for a tenant
    async fn list_files(
        &self,
        tenant_id: i64,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<FileMetadata>, String>;

    /// Generate a presigned URL for download (S3 backends only, returns error for local)
    async fn presigned_url(
        &self,
        tenant_id: i64,
        file_id: i64,
        expires_in_secs: u32,
    ) -> Result<PresignedUrl, String>;

    /// Get total storage used by a tenant
    async fn storage_used(&self, tenant_id: i64) -> Result<i64, String>;
}

/// Type alias for boxed file storage
pub type BoxFileStorage = Arc<dyn FileStorage>;

/// Local filesystem storage backend
pub struct LocalFileStorage {
    base_path: PathBuf,
    metadata: parking_lot::RwLock<Vec<FileMetadata>>,
    next_id: parking_lot::RwLock<i64>,
}

impl LocalFileStorage {
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        let base = base_path.into();
        std::fs::create_dir_all(&base).ok();
        Self {
            base_path: base,
            metadata: parking_lot::RwLock::new(Vec::new()),
            next_id: parking_lot::RwLock::new(1),
        }
    }

    fn allocate_id(&self) -> i64 {
        let mut id = self.next_id.write();
        let file_id = *id;
        *id += 1;
        file_id
    }

    fn tenant_path(&self, tenant_id: i64) -> PathBuf {
        self.base_path.join(format!("tenant_{}", tenant_id))
    }

    fn compute_checksum(data: &[u8]) -> String {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        use std::hash::{Hash, Hasher};
        data.hash(&mut hasher);
        let hash = hasher.finish();
        format!("{:016x}", hash)
    }
}

#[async_trait::async_trait]
impl FileStorage for LocalFileStorage {
    async fn upload(&self, upload: FileUpload) -> Result<FileMetadata, String> {
        let id = self.allocate_id();
        let checksum = Self::compute_checksum(&upload.data);
        let storage_path = format!("tenant_{}/{}/{}", upload.tenant_id, id, upload.filename);

        // Create tenant directory
        let tenant_dir = self.tenant_path(upload.tenant_id);
        std::fs::create_dir_all(&tenant_dir)
            .map_err(|e| format!("Failed to create directory: {}", e))?;

        // Write file
        let file_path = tenant_dir.join(format!("{}_{}", id, upload.filename));
        std::fs::write(&file_path, &upload.data)
            .map_err(|e| format!("Failed to write file: {}", e))?;

        let metadata = FileMetadata {
            id,
            tenant_id: upload.tenant_id,
            filename: format!("{}_{}", id, upload.filename),
            original_filename: upload.filename,
            content_type: upload.content_type,
            size_bytes: upload.data.len() as i64,
            storage_path,
            storage_backend: StorageBackend::Local,
            checksum,
            uploaded_by: upload.uploaded_by,
            created_at: Utc::now(),
            deleted_at: None,
        };

        self.metadata.write().push(metadata.clone());
        Ok(metadata)
    }

    async fn download(&self, tenant_id: i64, file_id: i64) -> Result<Vec<u8>, String> {
        let meta = self
            .metadata
            .read()
            .iter()
            .find(|m| m.id == file_id && m.tenant_id == tenant_id && m.deleted_at.is_none())
            .cloned()
            .ok_or_else(|| format!("File {} not found", file_id))?;

        let file_path = self.tenant_path(tenant_id).join(&meta.filename);
        std::fs::read(&file_path).map_err(|e| format!("Failed to read file: {}", e))
    }

    async fn get_metadata(
        &self,
        tenant_id: i64,
        file_id: i64,
    ) -> Result<Option<FileMetadata>, String> {
        Ok(self
            .metadata
            .read()
            .iter()
            .find(|m| m.id == file_id && m.tenant_id == tenant_id && m.deleted_at.is_none())
            .cloned())
    }

    async fn delete(&self, tenant_id: i64, file_id: i64) -> Result<(), String> {
        let mut metadata = self.metadata.write();
        let file = metadata
            .iter_mut()
            .find(|m| m.id == file_id && m.tenant_id == tenant_id && m.deleted_at.is_none())
            .ok_or_else(|| format!("File {} not found", file_id))?;

        // Soft delete — keep metadata, mark as deleted
        file.deleted_at = Some(Utc::now());

        // Optionally remove physical file
        let file_path = self.tenant_path(tenant_id).join(&file.filename);
        std::fs::remove_file(&file_path).ok();

        Ok(())
    }

    async fn list_files(
        &self,
        tenant_id: i64,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<FileMetadata>, String> {
        let metadata = self.metadata.read();
        Ok(metadata
            .iter()
            .filter(|m| m.tenant_id == tenant_id && m.deleted_at.is_none())
            .skip(offset as usize)
            .take(limit as usize)
            .cloned()
            .collect())
    }

    async fn presigned_url(
        &self,
        _tenant_id: i64,
        _file_id: i64,
        _expires_in_secs: u32,
    ) -> Result<PresignedUrl, String> {
        Err("Presigned URLs are not supported by local storage backend".to_string())
    }

    async fn storage_used(&self, tenant_id: i64) -> Result<i64, String> {
        let metadata = self.metadata.read();
        Ok(metadata
            .iter()
            .filter(|m| m.tenant_id == tenant_id && m.deleted_at.is_none())
            .map(|m| m.size_bytes)
            .sum())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn test_storage_path() -> PathBuf {
        let path = env::temp_dir().join(format!("turerp_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&path);
        path
    }

    #[tokio::test]
    async fn test_upload_and_download() {
        let storage = LocalFileStorage::new(test_storage_path());

        let upload = FileUpload {
            tenant_id: 1,
            filename: "test.txt".to_string(),
            content_type: "text/plain".to_string(),
            data: b"Hello, World!".to_vec(),
            uploaded_by: Some(1),
        };

        let meta = storage.upload(upload).await.unwrap();
        assert_eq!(meta.tenant_id, 1);
        assert_eq!(meta.original_filename, "test.txt");
        assert_eq!(meta.size_bytes, 13);
        assert_eq!(meta.storage_backend, StorageBackend::Local);

        let data = storage.download(1, meta.id).await.unwrap();
        assert_eq!(data, b"Hello, World!");
    }

    #[tokio::test]
    async fn test_get_metadata() {
        let storage = LocalFileStorage::new(test_storage_path());

        let upload = FileUpload {
            tenant_id: 1,
            filename: "doc.pdf".to_string(),
            content_type: "application/pdf".to_string(),
            data: vec![1, 2, 3, 4],
            uploaded_by: None,
        };

        let meta = storage.upload(upload).await.unwrap();
        let fetched = storage.get_metadata(1, meta.id).await.unwrap().unwrap();
        assert_eq!(fetched.id, meta.id);
        assert_eq!(fetched.content_type, "application/pdf");
    }

    #[tokio::test]
    async fn test_delete_file() {
        let storage = LocalFileStorage::new(test_storage_path());

        let upload = FileUpload {
            tenant_id: 1,
            filename: "delete_me.txt".to_string(),
            content_type: "text/plain".to_string(),
            data: b"delete me".to_vec(),
            uploaded_by: Some(1),
        };

        let meta = storage.upload(upload).await.unwrap();
        storage.delete(1, meta.id).await.unwrap();

        // Should not be findable
        let result = storage.get_metadata(1, meta.id).await.unwrap();
        assert!(result.is_none());

        // Download should fail
        let result = storage.download(1, meta.id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_files() {
        let storage = LocalFileStorage::new(test_storage_path());

        for i in 0..5 {
            storage
                .upload(FileUpload {
                    tenant_id: 1,
                    filename: format!("file_{}.txt", i),
                    content_type: "text/plain".to_string(),
                    data: format!("content {}", i).into_bytes(),
                    uploaded_by: Some(1),
                })
                .await
                .unwrap();
        }

        let files = storage.list_files(1, 10, 0).await.unwrap();
        assert_eq!(files.len(), 5);

        let files = storage.list_files(1, 3, 0).await.unwrap();
        assert_eq!(files.len(), 3);
    }

    #[tokio::test]
    async fn test_tenant_isolation() {
        let storage = LocalFileStorage::new(test_storage_path());

        storage
            .upload(FileUpload {
                tenant_id: 1,
                filename: "tenant1.txt".to_string(),
                content_type: "text/plain".to_string(),
                data: b"tenant 1 data".to_vec(),
                uploaded_by: None,
            })
            .await
            .unwrap();

        storage
            .upload(FileUpload {
                tenant_id: 2,
                filename: "tenant2.txt".to_string(),
                content_type: "text/plain".to_string(),
                data: b"tenant 2 data".to_vec(),
                uploaded_by: None,
            })
            .await
            .unwrap();

        let files1 = storage.list_files(1, 10, 0).await.unwrap();
        let files2 = storage.list_files(2, 10, 0).await.unwrap();
        assert_eq!(files1.len(), 1);
        assert_eq!(files2.len(), 1);
    }

    #[tokio::test]
    async fn test_storage_used() {
        let storage = LocalFileStorage::new(test_storage_path());

        storage
            .upload(FileUpload {
                tenant_id: 1,
                filename: "file1.txt".to_string(),
                content_type: "text/plain".to_string(),
                data: vec![0; 100],
                uploaded_by: None,
            })
            .await
            .unwrap();

        storage
            .upload(FileUpload {
                tenant_id: 1,
                filename: "file2.txt".to_string(),
                content_type: "text/plain".to_string(),
                data: vec![0; 200],
                uploaded_by: None,
            })
            .await
            .unwrap();

        let used = storage.storage_used(1).await.unwrap();
        assert_eq!(used, 300);
    }

    #[tokio::test]
    async fn test_presigned_url_not_supported() {
        let storage = LocalFileStorage::new(test_storage_path());

        let result = storage.presigned_url(1, 1, 3600).await;
        assert!(result.is_err());
    }
}
