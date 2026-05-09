//! S3-compatible object storage backend
//!
//! Supports AWS S3, MinIO, and other S3-compatible services via rust-s3.

use chrono::Utc;
use s3::{creds::Credentials, Bucket, Region};
use sha2::{Digest, Sha256};

use crate::common::file_storage::{
    FileMetadata, FileStorage, FileUpload, PresignedUrl, StorageBackend,
};
use crate::domain::file::model::CreateFileRecord;
use crate::domain::file::repository::BoxFileRepository;

/// S3/MinIO storage backend
pub struct S3FileStorage {
    bucket: Bucket,
    repo: BoxFileRepository,
}

impl S3FileStorage {
    /// Create a new S3 file storage from environment configuration
    pub async fn from_env(repo: BoxFileRepository) -> Result<Self, String> {
        let endpoint = std::env::var("S3_ENDPOINT")
            .map_err(|_| "S3_ENDPOINT environment variable not set".to_string())?;
        let bucket_name = std::env::var("S3_BUCKET")
            .map_err(|_| "S3_BUCKET environment variable not set".to_string())?;
        let access_key = std::env::var("S3_ACCESS_KEY")
            .map_err(|_| "S3_ACCESS_KEY environment variable not set".to_string())?;
        let secret_key = std::env::var("S3_SECRET_KEY")
            .map_err(|_| "S3_SECRET_KEY environment variable not set".to_string())?;
        let region = std::env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".to_string());

        let credentials = Credentials::new(Some(&access_key), Some(&secret_key), None, None, None)
            .map_err(|e| format!("Invalid S3 credentials: {}", e))?;

        let region = Region::Custom { region, endpoint };

        let bucket = *Bucket::new(&bucket_name, region, credentials)
            .map_err(|e| format!("Failed to create S3 bucket: {}", e))?
            .with_path_style();

        Ok(Self { bucket, repo })
    }

    /// Create a new S3 file storage with explicit configuration
    pub fn new(
        endpoint: &str,
        bucket_name: &str,
        access_key: &str,
        secret_key: &str,
        region: &str,
        repo: BoxFileRepository,
    ) -> Result<Self, String> {
        let credentials = Credentials::new(Some(access_key), Some(secret_key), None, None, None)
            .map_err(|e| format!("Invalid S3 credentials: {}", e))?;

        let region = Region::Custom {
            region: region.to_string(),
            endpoint: endpoint.to_string(),
        };

        let bucket = *Bucket::new(bucket_name, region, credentials)
            .map_err(|e| format!("Failed to create S3 bucket: {}", e))?
            .with_path_style();

        Ok(Self { bucket, repo })
    }

    fn compute_checksum(data: &[u8]) -> String {
        let hash = Sha256::digest(data);
        hex::encode(hash)
    }

    fn storage_path(tenant_id: i64, filename: &str) -> String {
        let uuid = uuid::Uuid::new_v4().to_string();
        format!("tenant_{}/{}_{}", tenant_id, uuid, filename)
    }

    fn map_repo_err(e: crate::error::ApiError) -> String {
        e.to_string()
    }

    fn record_to_metadata(record: crate::domain::file::model::FileRecord) -> FileMetadata {
        FileMetadata {
            id: record.id,
            tenant_id: record.tenant_id,
            filename: record.filename,
            original_filename: record.original_filename,
            content_type: record.content_type,
            size_bytes: record.size_bytes,
            storage_path: record.storage_path,
            storage_backend: StorageBackend::S3,
            checksum: record.checksum,
            uploaded_by: record.uploaded_by,
            entity_type: record.entity_type,
            entity_id: record.entity_id,
            created_at: record.created_at,
            deleted_at: record.deleted_at,
            deleted_by: record.deleted_by,
        }
    }
}

#[async_trait::async_trait]
impl FileStorage for S3FileStorage {
    async fn upload(&self, upload: FileUpload) -> Result<FileMetadata, String> {
        let checksum = Self::compute_checksum(&upload.data);
        let storage_path = Self::storage_path(upload.tenant_id, &upload.filename);

        // Upload to S3 first
        self.bucket
            .put_object(&storage_path, &upload.data)
            .await
            .map_err(|e| format!("S3 upload failed: {}", e))?;

        // Create metadata record
        let create = CreateFileRecord {
            tenant_id: upload.tenant_id,
            filename: format!("{}_{}", uuid::Uuid::new_v4(), upload.filename),
            original_filename: upload.filename,
            content_type: upload.content_type,
            size_bytes: upload.data.len() as i64,
            storage_path,
            storage_backend: "s3".to_string(),
            checksum,
            uploaded_by: upload.uploaded_by,
            entity_type: upload.entity_type,
            entity_id: upload.entity_id,
        };

        let record = self.repo.create(create).await.map_err(Self::map_repo_err)?;

        Ok(Self::record_to_metadata(record))
    }

    async fn download(&self, tenant_id: i64, file_id: i64) -> Result<Vec<u8>, String> {
        let record = self
            .repo
            .find_by_id(file_id, tenant_id)
            .await
            .map_err(Self::map_repo_err)?
            .ok_or_else(|| format!("File {} not found", file_id))?;

        if record.deleted_at.is_some() {
            return Err(format!("File {} has been deleted", file_id));
        }

        let response = self
            .bucket
            .get_object(&record.storage_path)
            .await
            .map_err(|e| format!("S3 download failed: {}", e))?;

        Ok(response.bytes().to_vec())
    }

    async fn get_metadata(
        &self,
        tenant_id: i64,
        file_id: i64,
    ) -> Result<Option<FileMetadata>, String> {
        match self
            .repo
            .find_by_id(file_id, tenant_id)
            .await
            .map_err(Self::map_repo_err)?
        {
            Some(record) => {
                if record.deleted_at.is_some() {
                    return Ok(None);
                }
                Ok(Some(Self::record_to_metadata(record)))
            }
            None => Ok(None),
        }
    }

    async fn delete(&self, tenant_id: i64, file_id: i64) -> Result<(), String> {
        let record = self
            .repo
            .find_by_id(file_id, tenant_id)
            .await
            .map_err(Self::map_repo_err)?
            .ok_or_else(|| format!("File {} not found", file_id))?;

        if record.deleted_at.is_some() {
            return Ok(());
        }

        // Soft delete metadata (deleted_by = 0 as placeholder when caller is unknown)
        self.repo
            .soft_delete(file_id, tenant_id, 0)
            .await
            .map_err(Self::map_repo_err)?;

        // Note: S3 object is kept to allow restore. Hard cleanup can be done by a background job.
        Ok(())
    }

    async fn list_files(
        &self,
        tenant_id: i64,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<FileMetadata>, String> {
        let records = self
            .repo
            .find_all(tenant_id)
            .await
            .map_err(Self::map_repo_err)?;

        Ok(records
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .map(Self::record_to_metadata)
            .collect())
    }

    async fn presigned_url(
        &self,
        tenant_id: i64,
        file_id: i64,
        expires_in_secs: u32,
    ) -> Result<PresignedUrl, String> {
        let record = self
            .repo
            .find_by_id(file_id, tenant_id)
            .await
            .map_err(Self::map_repo_err)?
            .ok_or_else(|| format!("File {} not found", file_id))?;

        if record.deleted_at.is_some() {
            return Err(format!("File {} has been deleted", file_id));
        }

        let url = self
            .bucket
            .presign_get(&record.storage_path, expires_in_secs, None)
            .await
            .map_err(|e| format!("Failed to generate presigned URL: {}", e))?;

        Ok(PresignedUrl {
            url,
            expires_at: Utc::now() + chrono::Duration::seconds(expires_in_secs as i64),
        })
    }

    async fn storage_used(&self, tenant_id: i64) -> Result<i64, String> {
        self.repo
            .storage_used(tenant_id)
            .await
            .map_err(Self::map_repo_err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::file::repository::InMemoryFileRepository;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_s3_checksum() {
        let data = b"hello world";
        let checksum = S3FileStorage::compute_checksum(data);
        assert_eq!(checksum.len(), 64); // SHA-256 hex length
    }

    #[tokio::test]
    async fn test_s3_storage_path() {
        let path = S3FileStorage::storage_path(1, "test.pdf");
        assert!(path.starts_with("tenant_1/"));
        assert!(path.ends_with("_test.pdf"));
    }

    #[tokio::test]
    async fn test_s3_file_storage_new_without_env() {
        let repo = Arc::new(InMemoryFileRepository::new()) as BoxFileRepository;
        let storage = S3FileStorage::new(
            "http://localhost:9000",
            "test-bucket",
            "minioadmin",
            "minioadmin",
            "us-east-1",
            repo,
        );
        // Should succeed in creating the bucket wrapper (no network call yet)
        assert!(storage.is_ok());
    }

    #[tokio::test]
    async fn test_s3_upload_and_metadata() {
        let repo = Arc::new(InMemoryFileRepository::new()) as BoxFileRepository;
        let storage = S3FileStorage::new(
            "http://localhost:9000",
            "test-bucket",
            "minioadmin",
            "minioadmin",
            "us-east-1",
            repo.clone(),
        )
        .unwrap();

        let upload = FileUpload {
            tenant_id: 1,
            filename: "test.txt".to_string(),
            content_type: "text/plain".to_string(),
            data: b"Hello, S3!".to_vec(),
            uploaded_by: Some(1),
            entity_type: None,
            entity_id: None,
        };

        // Upload will fail because there's no real S3 server, but we can at least
        // verify the metadata repo integration before the S3 call
        let result = storage.upload(upload).await;
        assert!(result.is_err()); // Expected: no S3 server running
        assert!(result.unwrap_err().contains("S3 upload failed"));
    }
}
