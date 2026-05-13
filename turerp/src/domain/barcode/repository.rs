//! Barcode repository

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use parking_lot::Mutex;

use crate::common::pagination::PaginatedResult;
use crate::domain::barcode::model::{BarcodeConfig, CreateBarcode};
use crate::error::ApiError;

/// Repository trait for barcode operations
#[async_trait]
pub trait BarcodeRepository: Send + Sync {
    /// Find barcode by entity type and entity id for a tenant
    async fn find_by_entity(
        &self,
        tenant_id: i64,
        entity_type: &str,
        entity_id: i64,
    ) -> Result<Option<BarcodeConfig>, ApiError>;

    /// Find all barcodes for a tenant with pagination
    async fn find_by_tenant(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<BarcodeConfig>, ApiError>;

    /// Create a new barcode record
    async fn create(
        &self,
        tenant_id: i64,
        create: CreateBarcode,
    ) -> Result<BarcodeConfig, ApiError>;

    /// Delete a barcode by id for a tenant
    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;

    /// Find barcode by id for a tenant
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<BarcodeConfig>, ApiError>;
}

/// Type alias for boxed barcode repository
pub type BoxBarcodeRepository = Arc<dyn BarcodeRepository>;

/// Inner state for InMemoryBarcodeRepository
struct InMemoryBarcodeInner {
    barcodes: HashMap<i64, BarcodeConfig>,
    next_id: i64,
    tenant_barcodes: HashMap<i64, Vec<i64>>,
}

/// In-memory barcode repository for development/testing
pub struct InMemoryBarcodeRepository {
    inner: Mutex<InMemoryBarcodeInner>,
}

impl InMemoryBarcodeRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryBarcodeInner {
                barcodes: HashMap::new(),
                next_id: 1,
                tenant_barcodes: HashMap::new(),
            }),
        }
    }
}

impl Default for InMemoryBarcodeRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BarcodeRepository for InMemoryBarcodeRepository {
    async fn find_by_entity(
        &self,
        tenant_id: i64,
        entity_type: &str,
        entity_id: i64,
    ) -> Result<Option<BarcodeConfig>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .barcodes
            .values()
            .find(|b| {
                b.tenant_id == tenant_id && b.entity_type == entity_type && b.entity_id == entity_id
            })
            .cloned())
    }

    async fn find_by_tenant(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<BarcodeConfig>, ApiError> {
        let inner = self.inner.lock();
        let all: Vec<_> = inner
            .barcodes
            .values()
            .filter(|b| b.tenant_id == tenant_id)
            .cloned()
            .collect();
        let total = all.len() as u64;
        let items: Vec<_> = all
            .into_iter()
            .skip(((page.saturating_sub(1)) * per_page) as usize)
            .take(per_page as usize)
            .collect();
        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn create(
        &self,
        tenant_id: i64,
        create: CreateBarcode,
    ) -> Result<BarcodeConfig, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_id;
        inner.next_id += 1;

        let barcode = BarcodeConfig {
            id,
            tenant_id,
            entity_type: create.entity_type,
            entity_id: create.entity_id,
            barcode_type: create.barcode_type,
            code: create.code,
            image_data: None,
            created_at: Utc::now(),
        };

        inner.barcodes.insert(id, barcode.clone());
        inner.tenant_barcodes.entry(tenant_id).or_default().push(id);

        Ok(barcode)
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let removed = inner
            .barcodes
            .remove(&id)
            .filter(|b| b.tenant_id == tenant_id);
        if removed.is_none() {
            return Err(ApiError::NotFound(format!("Barcode {} not found", id)));
        }
        if let Some(ids) = inner.tenant_barcodes.get_mut(&tenant_id) {
            ids.retain(|&bid| bid != id);
        }
        Ok(())
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<BarcodeConfig>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .barcodes
            .get(&id)
            .filter(|b| b.tenant_id == tenant_id)
            .cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::barcode::model::BarcodeType;

    #[tokio::test]
    async fn test_create_and_find_by_id() {
        let repo = InMemoryBarcodeRepository::new();
        let create = CreateBarcode {
            entity_type: "product".to_string(),
            entity_id: 1,
            barcode_type: BarcodeType::Ean13,
            code: "5901234123457".to_string(),
        };
        let barcode = repo.create(1, create).await.unwrap();
        assert_eq!(barcode.id, 1);
        assert_eq!(barcode.tenant_id, 1);

        let found = repo.find_by_id(1, 1).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().code, "5901234123457");
    }

    #[tokio::test]
    async fn test_find_by_entity() {
        let repo = InMemoryBarcodeRepository::new();
        let create = CreateBarcode {
            entity_type: "invoice".to_string(),
            entity_id: 42,
            barcode_type: BarcodeType::QrCode,
            code: "INV-001".to_string(),
        };
        repo.create(1, create).await.unwrap();

        let found = repo.find_by_entity(1, "invoice", 42).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().barcode_type, BarcodeType::QrCode);
    }

    #[tokio::test]
    async fn test_find_by_tenant_paginated() {
        let repo = InMemoryBarcodeRepository::new();
        for i in 1..=5 {
            let create = CreateBarcode {
                entity_type: "product".to_string(),
                entity_id: i,
                barcode_type: BarcodeType::Code128,
                code: format!("CODE{}", i),
            };
            repo.create(1, create).await.unwrap();
        }

        let page1 = repo.find_by_tenant(1, 1, 2).await.unwrap();
        assert_eq!(page1.items.len(), 2);
        assert_eq!(page1.total, 5);

        let page2 = repo.find_by_tenant(1, 2, 2).await.unwrap();
        assert_eq!(page2.items.len(), 2);

        let page3 = repo.find_by_tenant(1, 3, 2).await.unwrap();
        assert_eq!(page3.items.len(), 1);
    }

    #[tokio::test]
    async fn test_delete() {
        let repo = InMemoryBarcodeRepository::new();
        let create = CreateBarcode {
            entity_type: "product".to_string(),
            entity_id: 1,
            barcode_type: BarcodeType::Ean13,
            code: "5901234123457".to_string(),
        };
        repo.create(1, create).await.unwrap();

        repo.delete(1, 1).await.unwrap();
        let found = repo.find_by_id(1, 1).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_delete_not_found() {
        let repo = InMemoryBarcodeRepository::new();
        let result = repo.delete(999, 1).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ApiError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_tenant_isolation() {
        let repo = InMemoryBarcodeRepository::new();
        let create = CreateBarcode {
            entity_type: "product".to_string(),
            entity_id: 1,
            barcode_type: BarcodeType::Ean13,
            code: "5901234123457".to_string(),
        };
        repo.create(1, create).await.unwrap();

        let found_tenant2 = repo.find_by_id(1, 2).await.unwrap();
        assert!(found_tenant2.is_none());
    }
}
