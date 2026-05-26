//! File metadata service — business logic

use crate::domain::file::model::{CreateFileRecord, FileRecord, UpdateFileRecord};
use crate::domain::file::repository::BoxFileRepository;
use crate::error::ApiError;

/// Service for managing file metadata records
#[derive(Clone)]
pub struct FileService {
    repo: BoxFileRepository,
}

impl FileService {
    pub fn new(repo: BoxFileRepository) -> Self {
        Self { repo }
    }

    /// Create a new file metadata record
    #[tracing::instrument(skip(self))]
    pub async fn create(&self, create: CreateFileRecord) -> Result<FileRecord, ApiError> {
        if create.filename.trim().is_empty() {
            return Err(ApiError::Validation("Filename is required".to_string()));
        }
        if create.original_filename.trim().is_empty() {
            return Err(ApiError::Validation(
                "Original filename is required".to_string(),
            ));
        }
        if create.content_type.trim().is_empty() {
            return Err(ApiError::Validation("Content type is required".to_string()));
        }
        if create.size_bytes <= 0 {
            return Err(ApiError::Validation(
                "File size must be positive".to_string(),
            ));
        }
        if create.storage_path.trim().is_empty() {
            return Err(ApiError::Validation("Storage path is required".to_string()));
        }
        if create.storage_backend.trim().is_empty() {
            return Err(ApiError::Validation(
                "Storage backend is required".to_string(),
            ));
        }
        if create.checksum.trim().is_empty() {
            return Err(ApiError::Validation("Checksum is required".to_string()));
        }
        self.repo.create(create).await
    }

    /// Get a file metadata record by ID
    #[tracing::instrument(skip(self))]
    pub async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<FileRecord, ApiError> {
        self.repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("File {} not found", id)))
    }

    /// List all file metadata records for a tenant
    #[tracing::instrument(skip(self))]
    pub async fn find_all(&self, tenant_id: i64) -> Result<Vec<FileRecord>, ApiError> {
        self.repo.find_all(tenant_id).await
    }

    /// Update file metadata
    #[tracing::instrument(skip(self))]
    pub async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateFileRecord,
    ) -> Result<FileRecord, ApiError> {
        if let Some(ref filename) = update.filename {
            if filename.trim().is_empty() {
                return Err(ApiError::Validation("Filename cannot be empty".to_string()));
            }
        }
        if let Some(ref original_filename) = update.original_filename {
            if original_filename.trim().is_empty() {
                return Err(ApiError::Validation(
                    "Original filename cannot be empty".to_string(),
                ));
            }
        }
        if let Some(ref content_type) = update.content_type {
            if content_type.trim().is_empty() {
                return Err(ApiError::Validation(
                    "Content type cannot be empty".to_string(),
                ));
            }
        }
        self.repo.update(id, tenant_id, update).await
    }

    /// Soft delete a file
    #[tracing::instrument(skip(self))]
    pub async fn delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        self.repo.soft_delete(id, tenant_id, deleted_by).await
    }

    /// Find files linked to a specific entity
    #[tracing::instrument(skip(self))]
    pub async fn find_by_entity(
        &self,
        tenant_id: i64,
        entity_type: &str,
        entity_id: i64,
    ) -> Result<Vec<FileRecord>, ApiError> {
        self.repo
            .find_by_entity(tenant_id, entity_type, entity_id)
            .await
    }

    /// Restore a soft-deleted file
    #[tracing::instrument(skip(self))]
    pub async fn restore(&self, id: i64, tenant_id: i64) -> Result<FileRecord, ApiError> {
        self.repo.restore(id, tenant_id).await
    }

    /// List soft-deleted files for a tenant
    #[tracing::instrument(skip(self))]
    pub async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<FileRecord>, ApiError> {
        self.repo.find_deleted(tenant_id).await
    }

    /// Permanently destroy a file record
    #[tracing::instrument(skip(self))]
    pub async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.repo.destroy(id, tenant_id).await
    }

    /// Get total storage used by a tenant
    #[tracing::instrument(skip(self))]
    pub async fn storage_used(&self, tenant_id: i64) -> Result<i64, ApiError> {
        self.repo.storage_used(tenant_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::file::repository::InMemoryFileRepository;
    use std::sync::Arc;

    fn make_service() -> FileService {
        let repo = Arc::new(InMemoryFileRepository::new());
        FileService::new(repo)
    }

    fn make_create() -> CreateFileRecord {
        CreateFileRecord {
            tenant_id: 1,
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
        }
    }

    #[tokio::test]
    async fn test_create_and_find_by_id() {
        let svc = make_service();
        let create = make_create();

        let file = svc.create(create).await.unwrap();
        assert_eq!(file.filename, "doc_abc.pdf");

        let found = svc.find_by_id(file.id, 1).await.unwrap();
        assert_eq!(found.id, file.id);
    }

    #[tokio::test]
    async fn test_create_validation() {
        let svc = make_service();
        let mut create = make_create();
        create.filename = "".to_string();
        assert!(svc.create(create).await.is_err());
    }

    #[tokio::test]
    async fn test_find_all() {
        let svc = make_service();
        svc.create(make_create()).await.unwrap();

        let files = svc.find_all(1).await.unwrap();
        assert_eq!(files.len(), 1);

        let empty = svc.find_all(2).await.unwrap();
        assert!(empty.is_empty());
    }

    #[tokio::test]
    async fn test_update() {
        let svc = make_service();
        let file = svc.create(make_create()).await.unwrap();

        let updated = svc
            .update(
                file.id,
                1,
                UpdateFileRecord {
                    filename: Some("updated.pdf".to_string()),
                    original_filename: None,
                    content_type: None,
                    entity_type: Some("order".to_string()),
                    entity_id: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(updated.filename, "updated.pdf");
        assert_eq!(updated.entity_type, Some("order".to_string()));
        assert_eq!(updated.original_filename, "contract.pdf"); // unchanged
    }

    #[tokio::test]
    async fn test_update_validation() {
        let svc = make_service();
        let file = svc.create(make_create()).await.unwrap();

        let result = svc
            .update(
                file.id,
                1,
                UpdateFileRecord {
                    filename: Some("".to_string()),
                    original_filename: None,
                    content_type: None,
                    entity_type: None,
                    entity_id: None,
                },
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete() {
        let svc = make_service();
        let file = svc.create(make_create()).await.unwrap();

        svc.delete(file.id, 1, 5).await.unwrap();
        assert!(svc.find_by_id(file.id, 1).await.is_err());

        let deleted = svc.find_deleted(1).await.unwrap();
        assert_eq!(deleted.len(), 1);
    }

    #[tokio::test]
    async fn test_restore() {
        let svc = make_service();
        let file = svc.create(make_create()).await.unwrap();

        svc.delete(file.id, 1, 5).await.unwrap();
        let restored = svc.restore(file.id, 1).await.unwrap();
        assert_eq!(restored.id, file.id);
        assert!(restored.deleted_at.is_none());

        let found = svc.find_by_id(file.id, 1).await.unwrap();
        assert_eq!(found.id, file.id);
    }

    #[tokio::test]
    async fn test_find_by_entity() {
        let svc = make_service();
        svc.create(make_create()).await.unwrap();

        let files = svc.find_by_entity(1, "invoice", 42).await.unwrap();
        assert_eq!(files.len(), 1);

        let empty = svc.find_by_entity(1, "order", 42).await.unwrap();
        assert!(empty.is_empty());
    }

    #[tokio::test]
    async fn test_destroy() {
        let svc = make_service();
        let file = svc.create(make_create()).await.unwrap();

        svc.destroy(file.id, 1).await.unwrap();
        assert!(svc.find_by_id(file.id, 1).await.is_err());
        assert!(svc.find_deleted(1).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_storage_used() {
        let svc = make_service();
        svc.create(make_create()).await.unwrap();

        let used = svc.storage_used(1).await.unwrap();
        assert_eq!(used, 1024);

        let empty = svc.storage_used(2).await.unwrap();
        assert_eq!(empty, 0);
    }

    #[tokio::test]
    async fn test_tenant_isolation() {
        let svc = make_service();
        let file = svc.create(make_create()).await.unwrap();

        assert!(svc.find_by_id(file.id, 2).await.is_err());
    }
}
