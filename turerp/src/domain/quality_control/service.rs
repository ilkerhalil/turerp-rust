//! Quality control service

use crate::domain::quality_control::model::{
    CreateInspection, CreateNonConformanceReport, Inspection, NonConformanceReport,
    UpdateInspection, UpdateNonConformanceReport,
};
use crate::domain::quality_control::repository::{BoxInspectionRepository, BoxNcrRepository};
use crate::error::ApiError;

#[derive(Clone)]
pub struct QualityControlService {
    inspection_repo: BoxInspectionRepository,
    ncr_repo: BoxNcrRepository,
}

impl QualityControlService {
    pub fn new(inspection_repo: BoxInspectionRepository, ncr_repo: BoxNcrRepository) -> Self {
        Self {
            inspection_repo,
            ncr_repo,
        }
    }

    // Inspection methods
    #[tracing::instrument(skip(self))]
    pub async fn create_inspection(
        &self,
        create: CreateInspection,
    ) -> Result<Inspection, ApiError> {
        self.inspection_repo.create(create).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_inspection(&self, id: i64, tenant_id: i64) -> Result<Inspection, ApiError> {
        self.inspection_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Inspection not found".to_string()))
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_inspections_by_tenant(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<Inspection>, ApiError> {
        self.inspection_repo.find_by_tenant(tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_inspections_by_work_order(
        &self,
        work_order_id: i64,
    ) -> Result<Vec<Inspection>, ApiError> {
        self.inspection_repo.find_by_work_order(work_order_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn update_inspection(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateInspection,
    ) -> Result<Inspection, ApiError> {
        self.inspection_repo.update(id, tenant_id, update).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn delete_inspection(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.inspection_repo
            .soft_delete(id, tenant_id, deleted_by)
            .await
    }

    // NCR methods
    #[tracing::instrument(skip(self))]
    pub async fn create_ncr(
        &self,
        create: CreateNonConformanceReport,
    ) -> Result<NonConformanceReport, ApiError> {
        self.ncr_repo.create(create).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_ncr(&self, id: i64, tenant_id: i64) -> Result<NonConformanceReport, ApiError> {
        self.ncr_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("NCR not found".to_string()))
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_ncrs_by_tenant(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<NonConformanceReport>, ApiError> {
        self.ncr_repo.find_by_tenant(tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_ncrs_by_inspection(
        &self,
        inspection_id: i64,
    ) -> Result<Vec<NonConformanceReport>, ApiError> {
        self.ncr_repo.find_by_inspection(inspection_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn update_ncr(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateNonConformanceReport,
    ) -> Result<NonConformanceReport, ApiError> {
        self.ncr_repo.update(id, tenant_id, update).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn delete_ncr(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.ncr_repo.soft_delete(id, tenant_id, deleted_by).await
    }

    // Inspection soft-delete methods
    #[tracing::instrument(skip(self))]
    pub async fn restore_inspection(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Inspection, ApiError> {
        self.inspection_repo.restore(id, tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn list_deleted_inspections(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<Inspection>, ApiError> {
        self.inspection_repo.find_deleted(tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn destroy_inspection(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.inspection_repo.destroy(id, tenant_id).await
    }

    // NCR soft-delete methods
    #[tracing::instrument(skip(self))]
    pub async fn restore_ncr(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<NonConformanceReport, ApiError> {
        self.ncr_repo.restore(id, tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn list_deleted_ncrs(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<NonConformanceReport>, ApiError> {
        self.ncr_repo.find_deleted(tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn destroy_ncr(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.ncr_repo.destroy(id, tenant_id).await
    }
}

#[cfg(test)]
mod qc_tests {
    use super::*;
    use crate::domain::quality_control::model::{
        CreateInspection, CreateNonConformanceReport, InspectionStatus, NcrType,
    };
    use crate::domain::quality_control::repository::{
        InMemoryInspectionRepository, InMemoryNcrRepository,
    };
    use rust_decimal_macros::dec;
    use std::sync::Arc;

    fn create_qc_service() -> QualityControlService {
        let inspection_repo =
            Arc::new(InMemoryInspectionRepository::new()) as BoxInspectionRepository;
        let ncr_repo = Arc::new(InMemoryNcrRepository::new()) as BoxNcrRepository;
        QualityControlService::new(inspection_repo, ncr_repo)
    }

    #[tokio::test]
    async fn test_create_inspection() {
        let service = create_qc_service();
        let create = CreateInspection {
            tenant_id: 1,
            work_order_id: Some(1),
            product_id: 1,
            inspection_type: "Visual".to_string(),
            quantity_inspected: dec!(100),
            quantity_passed: dec!(95),
            quantity_failed: dec!(5),
            status: InspectionStatus::Passed,
            inspector_id: Some(1),
            notes: None,
        };
        let result = service.create_inspection(create).await;
        assert!(result.is_ok());
        let inspection = result.unwrap();
        assert_eq!(inspection.quantity_inspected, dec!(100));
        assert_eq!(inspection.status, InspectionStatus::Passed);
    }

    #[tokio::test]
    async fn test_create_ncr() {
        let service = create_qc_service();
        let create = CreateNonConformanceReport {
            tenant_id: 1,
            inspection_id: Some(1),
            product_id: 1,
            ncr_type: NcrType::Minor,
            description: "Scratch on surface".to_string(),
            root_cause: None,
            corrective_action: None,
            raised_by: 1,
        };
        let result = service.create_ncr(create).await;
        assert!(result.is_ok());
        let ncr = result.unwrap();
        assert_eq!(ncr.description, "Scratch on surface");
    }
}
