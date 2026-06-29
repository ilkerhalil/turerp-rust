//! Quality control repository

use async_trait::async_trait;
use chrono::Utc;
use parking_lot::Mutex;
use std::sync::Arc;

use crate::common::SoftDeletable;
use crate::domain::quality_control::model::{
    CreateInspection, CreateNonConformanceReport, Inspection, InspectionStatus, NcrStatus,
    NonConformanceReport, UpdateInspection, UpdateNonConformanceReport,
};
use crate::error::ApiError;

#[async_trait]
pub trait InspectionRepository: Send + Sync {
    async fn create(&self, inspection: CreateInspection) -> Result<Inspection, ApiError>;
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Inspection>, ApiError>;
    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Inspection>, ApiError>;
    async fn find_by_work_order(&self, work_order_id: i64) -> Result<Vec<Inspection>, ApiError>;
    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateInspection,
    ) -> Result<Inspection, ApiError>;
    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError>;
    async fn restore(&self, id: i64, tenant_id: i64) -> Result<Inspection, ApiError>;
    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Inspection>, ApiError>;
    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
}

#[async_trait]
pub trait NcrRepository: Send + Sync {
    async fn create(
        &self,
        ncr: CreateNonConformanceReport,
    ) -> Result<NonConformanceReport, ApiError>;
    async fn find_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<NonConformanceReport>, ApiError>;
    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<NonConformanceReport>, ApiError>;
    async fn find_by_inspection(
        &self,
        inspection_id: i64,
    ) -> Result<Vec<NonConformanceReport>, ApiError>;
    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateNonConformanceReport,
    ) -> Result<NonConformanceReport, ApiError>;
    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError>;
    async fn restore(&self, id: i64, tenant_id: i64) -> Result<NonConformanceReport, ApiError>;
    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<NonConformanceReport>, ApiError>;
    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
}

pub type BoxInspectionRepository = Arc<dyn InspectionRepository>;
pub type BoxNcrRepository = Arc<dyn NcrRepository>;

struct InMemoryInspectionInner {
    inspections: std::collections::HashMap<i64, Inspection>,
    next_id: i64,
}

pub struct InMemoryInspectionRepository {
    inner: Mutex<InMemoryInspectionInner>,
}

impl InMemoryInspectionRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryInspectionInner {
                inspections: std::collections::HashMap::new(),
                next_id: 1,
            }),
        }
    }
}

impl Default for InMemoryInspectionRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InspectionRepository for InMemoryInspectionRepository {
    async fn create(&self, create: CreateInspection) -> Result<Inspection, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;
        let mut inner = self.inner.lock();
        let id = inner.next_id;
        inner.next_id += 1;
        let now = Utc::now();
        let status = create.status.clone();
        let inspection = Inspection {
            id,
            tenant_id: create.tenant_id,
            work_order_id: create.work_order_id,
            product_id: create.product_id,
            inspection_type: create.inspection_type,
            quantity_inspected: create.quantity_inspected,
            quantity_passed: create.quantity_passed,
            quantity_failed: create.quantity_failed,
            status: create.status,
            inspector_id: create.inspector_id,
            inspected_at: if status == InspectionStatus::Passed
                || status == InspectionStatus::Failed
            {
                Some(now)
            } else {
                None
            },
            notes: create.notes,
            created_at: now,
            deleted_at: None,
            deleted_by: None,
        };
        inner.inspections.insert(id, inspection.clone());
        Ok(inspection)
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Inspection>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .inspections
            .get(&id)
            .filter(|x| x.tenant_id == tenant_id && !x.is_deleted())
            .cloned())
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Inspection>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .inspections
            .values()
            .filter(|x| x.tenant_id == tenant_id && !x.is_deleted())
            .cloned()
            .collect())
    }

    async fn find_by_work_order(&self, work_order_id: i64) -> Result<Vec<Inspection>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .inspections
            .values()
            .filter(|x| x.work_order_id == Some(work_order_id) && !x.is_deleted())
            .cloned()
            .collect())
    }

    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateInspection,
    ) -> Result<Inspection, ApiError> {
        let mut inner = self.inner.lock();
        let inspection = inner
            .inspections
            .get_mut(&id)
            .filter(|x| x.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound("Inspection not found".to_string()))?;
        if let Some(status) = update.status {
            inspection.status = status;
            if inspection.status == InspectionStatus::Passed
                || inspection.status == InspectionStatus::Failed
            {
                inspection.inspected_at = Some(Utc::now());
            }
        }
        if let Some(qp) = update.quantity_passed {
            inspection.quantity_passed = qp;
        }
        if let Some(qf) = update.quantity_failed {
            inspection.quantity_failed = qf;
        }
        if let Some(inspector_id) = update.inspector_id {
            inspection.inspector_id = Some(inspector_id);
        }
        if let Some(notes) = update.notes {
            inspection.notes = Some(notes);
        }
        Ok(inspection.clone())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let inspection = inner
            .inspections
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound("Inspection not found".to_string()))?;
        if inspection.tenant_id != tenant_id {
            return Err(ApiError::NotFound("Inspection not found".to_string()));
        }
        inspection.mark_deleted(deleted_by);
        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<Inspection, ApiError> {
        let mut inner = self.inner.lock();
        let inspection = inner
            .inspections
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound("Inspection not found".to_string()))?;
        if inspection.tenant_id != tenant_id {
            return Err(ApiError::NotFound("Inspection not found".to_string()));
        }
        inspection.restore();
        Ok(inspection.clone())
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Inspection>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .inspections
            .values()
            .filter(|x| x.tenant_id == tenant_id && x.is_deleted())
            .cloned()
            .collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let inspection = inner
            .inspections
            .get(&id)
            .ok_or_else(|| ApiError::NotFound("Inspection not found".to_string()))?;
        if inspection.tenant_id != tenant_id {
            return Err(ApiError::NotFound("Inspection not found".to_string()));
        }
        inner.inspections.remove(&id);
        Ok(())
    }
}

struct InMemoryNcrInner {
    ncrs: std::collections::HashMap<i64, NonConformanceReport>,
    next_id: i64,
}

pub struct InMemoryNcrRepository {
    inner: Mutex<InMemoryNcrInner>,
}

impl InMemoryNcrRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryNcrInner {
                ncrs: std::collections::HashMap::new(),
                next_id: 1,
            }),
        }
    }
}

impl Default for InMemoryNcrRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NcrRepository for InMemoryNcrRepository {
    async fn create(
        &self,
        create: CreateNonConformanceReport,
    ) -> Result<NonConformanceReport, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;
        let mut inner = self.inner.lock();
        let id = inner.next_id;
        inner.next_id += 1;
        let now = Utc::now();
        let ncr = NonConformanceReport {
            id,
            tenant_id: create.tenant_id,
            inspection_id: create.inspection_id,
            product_id: create.product_id,
            ncr_type: create.ncr_type,
            description: create.description,
            root_cause: create.root_cause,
            corrective_action: create.corrective_action,
            status: NcrStatus::Open,
            raised_by: create.raised_by,
            raised_at: now,
            closed_at: None,
            deleted_at: None,
            deleted_by: None,
        };
        inner.ncrs.insert(id, ncr.clone());
        Ok(ncr)
    }

    async fn find_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<NonConformanceReport>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .ncrs
            .get(&id)
            .filter(|x| x.tenant_id == tenant_id && !x.is_deleted())
            .cloned())
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<NonConformanceReport>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .ncrs
            .values()
            .filter(|x| x.tenant_id == tenant_id && !x.is_deleted())
            .cloned()
            .collect())
    }

    async fn find_by_inspection(
        &self,
        inspection_id: i64,
    ) -> Result<Vec<NonConformanceReport>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .ncrs
            .values()
            .filter(|x| x.inspection_id == Some(inspection_id) && !x.is_deleted())
            .cloned()
            .collect())
    }

    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateNonConformanceReport,
    ) -> Result<NonConformanceReport, ApiError> {
        let mut inner = self.inner.lock();
        let ncr = inner
            .ncrs
            .get_mut(&id)
            .filter(|x| x.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound("NCR not found".to_string()))?;
        if let Some(ncr_type) = update.ncr_type {
            ncr.ncr_type = ncr_type;
        }
        if let Some(description) = update.description {
            ncr.description = description;
        }
        if let Some(root_cause) = update.root_cause {
            ncr.root_cause = Some(root_cause);
        }
        if let Some(corrective_action) = update.corrective_action {
            ncr.corrective_action = Some(corrective_action);
        }
        if let Some(status) = update.status {
            ncr.status = status;
            if ncr.status == NcrStatus::Closed {
                ncr.closed_at = Some(Utc::now());
            }
        }
        Ok(ncr.clone())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let ncr = inner
            .ncrs
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound("NCR not found".to_string()))?;
        if ncr.tenant_id != tenant_id {
            return Err(ApiError::NotFound("NCR not found".to_string()));
        }
        ncr.mark_deleted(deleted_by);
        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<NonConformanceReport, ApiError> {
        let mut inner = self.inner.lock();
        let ncr = inner
            .ncrs
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound("NCR not found".to_string()))?;
        if ncr.tenant_id != tenant_id {
            return Err(ApiError::NotFound("NCR not found".to_string()));
        }
        ncr.restore();
        Ok(ncr.clone())
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<NonConformanceReport>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .ncrs
            .values()
            .filter(|x| x.tenant_id == tenant_id && x.is_deleted())
            .cloned()
            .collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let ncr = inner
            .ncrs
            .get(&id)
            .ok_or_else(|| ApiError::NotFound("NCR not found".to_string()))?;
        if ncr.tenant_id != tenant_id {
            return Err(ApiError::NotFound("NCR not found".to_string()));
        }
        inner.ncrs.remove(&id);
        Ok(())
    }
}
