//! File metadata repository

use async_trait::async_trait;
use chrono::Utc;
use parking_lot::Mutex;
use std::sync::Arc;

use crate::domain::file::model::{CreateFileRecord, FileRecord};
use crate::error::ApiError;

/// Repository trait for file metadata operations
#[async_trait]
pub trait FileRepository: Send + Sync {
    /// Create a new file metadata record
    async fn create(&self, file: CreateFileRecord) -> Result<FileRecord, ApiError>;

    /// Find file by ID (tenant-scoped)
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<FileRecord>, ApiError>;

    /// Find all files for a tenant
    async fn find_all(&self, tenant_id: i64) -> Result<Vec<FileRecord>, ApiError>;

    /// Find files by entity type and entity ID
    async fn find_by_entity(
        &self,
        tenant_id: i64,
        entity_type: &str,
        entity_id: i64,
    ) -> Result<Vec<FileRecord>, ApiError>;

    /// Soft delete a file
    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError>;

    /// Restore a soft-deleted file
    async fn restore(&self, id: i64, tenant_id: i64) -> Result<FileRecord, ApiError>;

    /// Find deleted files for a tenant
    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<FileRecord>, ApiError>;

    /// Hard delete a file (permanent)
    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;

    /// Get total storage used by a tenant
    async fn storage_used(&self, tenant_id: i64) -> Result<i64, ApiError>;
}

/// Type alias for boxed file repository
pub type BoxFileRepository = Arc<dyn FileRepository>;

/// In-memory file repository for development/testing
pub struct InMemoryFileRepository {
    files: Mutex<Vec<FileRecord>>,
    next_id: Mutex<i64>,
}

impl InMemoryFileRepository {
    pub fn new() -> Self {
        Self {
            files: Mutex::new(Vec::new()),
            next_id: Mutex::new(1),
        }
    }

    fn allocate_id(&self) -> i64 {
        let mut id = self.next_id.lock();
        let file_id = *id;
        *id += 1;
        file_id
    }
}

impl Default for InMemoryFileRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FileRepository for InMemoryFileRepository {
    async fn create(&self, file: CreateFileRecord) -> Result<FileRecord, ApiError> {
        let record = FileRecord {
            id: self.allocate_id(),
            tenant_id: file.tenant_id,
            filename: file.filename,
            original_filename: file.original_filename,
            content_type: file.content_type,
            size_bytes: file.size_bytes,
            storage_path: file.storage_path,
            storage_backend: file.storage_backend,
            checksum: file.checksum,
            uploaded_by: file.uploaded_by,
            entity_type: file.entity_type,
            entity_id: file.entity_id,
            created_at: Utc::now(),
            deleted_at: None,
            deleted_by: None,
        };
        self.files.lock().push(record.clone());
        Ok(record)
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<FileRecord>, ApiError> {
        Ok(self
            .files
            .lock()
            .iter()
            .find(|f| f.id == id && f.tenant_id == tenant_id && f.deleted_at.is_none())
            .cloned())
    }

    async fn find_all(&self, tenant_id: i64) -> Result<Vec<FileRecord>, ApiError> {
        Ok(self
            .files
            .lock()
            .iter()
            .filter(|f| f.tenant_id == tenant_id && f.deleted_at.is_none())
            .cloned()
            .collect())
    }

    async fn find_by_entity(
        &self,
        tenant_id: i64,
        entity_type: &str,
        entity_id: i64,
    ) -> Result<Vec<FileRecord>, ApiError> {
        Ok(self
            .files
            .lock()
            .iter()
            .filter(|f| {
                f.tenant_id == tenant_id
                    && f.deleted_at.is_none()
                    && f.entity_type.as_deref() == Some(entity_type)
                    && f.entity_id == Some(entity_id)
            })
            .cloned()
            .collect())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let mut files = self.files.lock();
        let file = files
            .iter_mut()
            .find(|f| f.id == id && f.tenant_id == tenant_id && f.deleted_at.is_none())
            .ok_or_else(|| ApiError::NotFound(format!("File {} not found", id)))?;
        file.deleted_at = Some(Utc::now());
        file.deleted_by = Some(deleted_by);
        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<FileRecord, ApiError> {
        let mut files = self.files.lock();
        let file = files
            .iter_mut()
            .find(|f| f.id == id && f.tenant_id == tenant_id && f.deleted_at.is_some())
            .ok_or_else(|| ApiError::NotFound(format!("File {} not found", id)))?;
        file.deleted_at = None;
        file.deleted_by = None;
        Ok(file.clone())
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<FileRecord>, ApiError> {
        Ok(self
            .files
            .lock()
            .iter()
            .filter(|f| f.tenant_id == tenant_id && f.deleted_at.is_some())
            .cloned()
            .collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut files = self.files.lock();
        let pos = files
            .iter()
            .position(|f| f.id == id && f.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("File {} not found", id)))?;
        files.remove(pos);
        Ok(())
    }

    async fn storage_used(&self, tenant_id: i64) -> Result<i64, ApiError> {
        Ok(self
            .files
            .lock()
            .iter()
            .filter(|f| f.tenant_id == tenant_id && f.deleted_at.is_none())
            .map(|f| f.size_bytes)
            .sum())
    }
}
