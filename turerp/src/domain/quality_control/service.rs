//! Quality control service

use crate::domain::manufacturing::repository::BoxWorkOrderRepository;
use crate::domain::product::repository::BoxProductRepository;
use crate::domain::quality_control::model::{
    CreateInspection, CreateNonConformanceReport, Inspection, NonConformanceReport,
    UpdateInspection, UpdateNonConformanceReport,
};
use crate::domain::quality_control::repository::{BoxInspectionRepository, BoxNcrRepository};
use crate::domain::user::repository::BoxUserRepository;
use crate::domain::user::service::ensure_user_owned;
use crate::error::ApiError;

#[derive(Clone)]
pub struct QualityControlService {
    inspection_repo: BoxInspectionRepository,
    ncr_repo: BoxNcrRepository,
    product_repo: BoxProductRepository,
    work_order_repo: BoxWorkOrderRepository,
    user_repo: BoxUserRepository,
}

impl QualityControlService {
    pub fn new(
        inspection_repo: BoxInspectionRepository,
        ncr_repo: BoxNcrRepository,
        product_repo: BoxProductRepository,
        work_order_repo: BoxWorkOrderRepository,
        user_repo: BoxUserRepository,
    ) -> Self {
        Self {
            inspection_repo,
            ncr_repo,
            product_repo,
            work_order_repo,
            user_repo,
        }
    }

    // Inspection methods
    #[tracing::instrument(skip(self))]
    pub async fn create_inspection(
        &self,
        create: CreateInspection,
    ) -> Result<Inspection, ApiError> {
        // Parent-ownership precheck: the body-controlled `product_id` (required)
        // and `work_order_id` (optional) must belong to the caller's tenant
        // before the INSERT, otherwise a tenant-A admin could file an inspection
        // referencing a tenant-B product/work order (cross-tenant orphan write).
        // `create.tenant_id` is overwritten from auth by the handler before this
        // call, so it is the caller's real tenant.
        // Validate the request shape BEFORE the parent-ownership precheck so a
        // malformed request (e.g. `product_id <= 0`, bad quantities) is rejected
        // as 400 BadRequest rather than masked as a 404 precheck miss. The repo
        // re-validates (harmless) on the INSERT path.
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;
        let tenant_id = create.tenant_id;
        self.product_repo
            .find_by_id(create.product_id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Product not found".to_string()))?;
        if let Some(work_order_id) = create.work_order_id {
            self.work_order_repo
                .find_by_id(work_order_id, tenant_id)
                .await?
                .ok_or_else(|| ApiError::NotFound("Work order not found".to_string()))?;
        }
        // Parent-ownership precheck: a body-supplied `inspector_id` (the user
        // who performed the inspection) must belong to the caller's tenant.
        // `None` is a legitimate "unassigned" inspection and is NOT rejected.
        if let Some(id) = create.inspector_id {
            ensure_user_owned(&self.user_repo, id, tenant_id).await?;
        }
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
        // Parent-ownership precheck: a body-supplied `inspector_id` on update
        // must belong to the caller's tenant. The repo UPDATE persists
        // `inspector_id` via COALESCE, so without this gate a tenant-A admin
        // could re-stamp a tenant-B user id onto an existing inspection.
        // `None` leaves the stored value untouched and is NOT rejected.
        if let Some(id) = update.inspector_id {
            ensure_user_owned(&self.user_repo, id, tenant_id).await?;
        }
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
        // Parent-ownership precheck: the body-controlled `product_id` (required)
        // and `inspection_id` (optional) must belong to the caller's tenant
        // before the INSERT, otherwise a tenant-A admin could file an NCR
        // referencing a tenant-B product/inspection (cross-tenant orphan write).
        // `create.tenant_id` is overwritten from auth by the handler before this
        // call, so it is the caller's real tenant.
        // Validate the request shape BEFORE the parent-ownership precheck so a
        // malformed request (e.g. `product_id <= 0`, empty description) is
        // rejected as 400 BadRequest rather than masked as a 404 precheck miss.
        // The repo re-validates (harmless) on the INSERT path.
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;
        let tenant_id = create.tenant_id;
        self.product_repo
            .find_by_id(create.product_id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Product not found".to_string()))?;
        if let Some(inspection_id) = create.inspection_id {
            self.inspection_repo
                .find_by_id(inspection_id, tenant_id)
                .await?
                .ok_or_else(|| ApiError::NotFound("Inspection not found".to_string()))?;
        }
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
    use crate::domain::manufacturing::model::{CreateWorkOrder, WorkOrderPriority};
    use crate::domain::manufacturing::repository::{
        BoxWorkOrderRepository, InMemoryWorkOrderRepository,
    };
    use crate::domain::product::model::CreateProduct;
    use crate::domain::product::repository::{BoxProductRepository, InMemoryProductRepository};
    use crate::domain::quality_control::model::{
        CreateInspection, CreateNonConformanceReport, InspectionStatus, NcrType,
    };
    use crate::domain::quality_control::repository::{
        InMemoryInspectionRepository, InMemoryNcrRepository,
    };
    use crate::domain::user::model::{CreateUser, Role};
    use crate::domain::user::repository::{BoxUserRepository, InMemoryUserRepository};
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use std::sync::Arc;

    async fn create_qc_service() -> (
        QualityControlService,
        BoxProductRepository,
        BoxWorkOrderRepository,
        BoxUserRepository,
    ) {
        let inspection_repo =
            Arc::new(InMemoryInspectionRepository::new()) as BoxInspectionRepository;
        let ncr_repo = Arc::new(InMemoryNcrRepository::new()) as BoxNcrRepository;
        let product_repo = Arc::new(InMemoryProductRepository::new()) as BoxProductRepository;
        let work_order_repo =
            Arc::new(InMemoryWorkOrderRepository::new()) as BoxWorkOrderRepository;
        let user_repo = Arc::new(InMemoryUserRepository::new()) as BoxUserRepository;
        // Seed a user per tenant: auto-id 1 for tenant-1 (resolves the happy-path
        // tests that stamp `inspector_id: Some(1)`) and auto-id 2 for tenant-2
        // (the foreign referent the reject tests target).
        for tenant in [1, 2] {
            user_repo
                .create(
                    CreateUser {
                        username: format!("t{}qc", tenant),
                        email: format!("t{}qc@example.com", tenant),
                        full_name: format!("Tenant {} qc user", tenant),
                        password: "password123456".to_string(),
                        tenant_id: tenant,
                        role: Some(Role::User),
                    },
                    "hash".to_string(),
                )
                .await
                .expect("seed user");
        }
        let service = QualityControlService::new(
            inspection_repo,
            ncr_repo,
            product_repo.clone(),
            work_order_repo.clone(),
            user_repo.clone(),
        );
        (service, product_repo, work_order_repo, user_repo)
    }

    /// Returns the tenant-2 user id (a foreign user) for the `inspector_id`
    /// reject tests.
    async fn foreign_inspector_id(user_repo: &BoxUserRepository) -> i64 {
        user_repo
            .find_all(2)
            .await
            .expect("list tenant-2 users")
            .into_iter()
            .map(|u| u.id)
            .next()
            .expect("tenant-2 user seeded")
    }

    /// Seed a product on `tenant_id` and return its id. The InMemory repo uses a
    /// global auto-id counter, so the first seeded product is id 1, the next id 2,
    /// etc. `find_by_id` filters by tenant_id, so an id seeded on tenant 1 does
    /// NOT exist on tenant 2 (genuine cross-tenant NotFound, no false pass).
    async fn seed_product(repo: &BoxProductRepository, tenant_id: i64) -> i64 {
        let product = repo
            .create(CreateProduct {
                tenant_id,
                company_id: 1,
                code: format!("PROD-{}-{}", tenant_id, uuid::Uuid::new_v4()),
                name: format!("Product for tenant {}", tenant_id),
                description: None,
                category_id: None,
                unit_id: None,
                barcode: None,
                purchase_price: Decimal::ZERO,
                sale_price: Decimal::ZERO,
                tax_rate: Decimal::ZERO,
            })
            .await
            .expect("seed product");
        product.id
    }

    /// Seed a work order on `tenant_id` (referencing `product_id`) and return its
    /// id. The InMemory work-order repo does not validate the product, so any
    /// tenant-owned product id is fine.
    async fn seed_work_order(
        repo: &BoxWorkOrderRepository,
        tenant_id: i64,
        product_id: i64,
    ) -> i64 {
        let wo = repo
            .create(CreateWorkOrder {
                tenant_id,
                name: format!("WO-{}", uuid::Uuid::new_v4()),
                product_id,
                quantity: dec!(1),
                bom_id: None,
                routing_id: None,
                priority: WorkOrderPriority::Normal,
                planned_start: None,
                planned_end: None,
            })
            .await
            .expect("seed work order");
        wo.id
    }

    #[tokio::test]
    async fn test_create_inspection() {
        let (service, product_repo, work_order_repo, _user_repo) = create_qc_service().await;
        let product_id = seed_product(&product_repo, 1).await;
        let work_order_id = seed_work_order(&work_order_repo, 1, product_id).await;
        let create = CreateInspection {
            tenant_id: 1,
            work_order_id: Some(work_order_id),
            product_id,
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
    async fn test_create_inspection_rejects_foreign_product() {
        // Tenant-1 product; tenant-2 caller references it -> NotFound.
        let (service, product_repo, _work_order_repo, _user_repo) = create_qc_service().await;
        let foreign_product_id = seed_product(&product_repo, 1).await;
        let create = CreateInspection {
            tenant_id: 2,
            work_order_id: None,
            product_id: foreign_product_id,
            inspection_type: "Visual".to_string(),
            quantity_inspected: dec!(100),
            quantity_passed: dec!(95),
            quantity_failed: dec!(5),
            status: InspectionStatus::Passed,
            inspector_id: Some(1),
            notes: None,
        };
        let result = service.create_inspection(create).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ApiError::NotFound(msg) if msg == "Product not found"
        ));
    }

    #[tokio::test]
    async fn test_create_inspection_rejects_foreign_work_order() {
        // Tenant-2 product (passes the product precheck) + tenant-1 work order ->
        // NotFound on the work order, isolating the work-order precheck.
        let (service, product_repo, work_order_repo, _user_repo) = create_qc_service().await;
        let product_id = seed_product(&product_repo, 2).await;
        let foreign_work_order_id = seed_work_order(&work_order_repo, 1, product_id).await;
        let create = CreateInspection {
            tenant_id: 2,
            work_order_id: Some(foreign_work_order_id),
            product_id,
            inspection_type: "Visual".to_string(),
            quantity_inspected: dec!(100),
            quantity_passed: dec!(95),
            quantity_failed: dec!(5),
            status: InspectionStatus::Passed,
            inspector_id: Some(1),
            notes: None,
        };
        let result = service.create_inspection(create).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ApiError::NotFound(msg) if msg == "Work order not found"
        ));
    }

    #[tokio::test]
    async fn test_create_ncr() {
        let (service, product_repo, _work_order_repo, _user_repo) = create_qc_service().await;
        let product_id = seed_product(&product_repo, 1).await;
        // Seed an inspection on tenant 1 so the optional inspection_id is owned.
        let inspection = service
            .create_inspection(CreateInspection {
                tenant_id: 1,
                work_order_id: None,
                product_id,
                inspection_type: "Visual".to_string(),
                quantity_inspected: dec!(100),
                quantity_passed: dec!(95),
                quantity_failed: dec!(5),
                status: InspectionStatus::Passed,
                inspector_id: Some(1),
                notes: None,
            })
            .await
            .expect("seed inspection");
        let create = CreateNonConformanceReport {
            tenant_id: 1,
            inspection_id: Some(inspection.id),
            product_id,
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

    #[tokio::test]
    async fn test_create_ncr_rejects_foreign_product() {
        // Tenant-1 product; tenant-2 caller references it -> NotFound.
        let (service, product_repo, _work_order_repo, _user_repo) = create_qc_service().await;
        let foreign_product_id = seed_product(&product_repo, 1).await;
        let create = CreateNonConformanceReport {
            tenant_id: 2,
            inspection_id: None,
            product_id: foreign_product_id,
            ncr_type: NcrType::Minor,
            description: "Scratch on surface".to_string(),
            root_cause: None,
            corrective_action: None,
            raised_by: 1,
        };
        let result = service.create_ncr(create).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ApiError::NotFound(msg) if msg == "Product not found"
        ));
    }

    #[tokio::test]
    async fn test_create_ncr_rejects_foreign_inspection() {
        // Tenant-2 product (passes the product precheck) + tenant-1 inspection ->
        // NotFound on the inspection, isolating the inspection precheck.
        let (service, product_repo, _work_order_repo, _user_repo) = create_qc_service().await;
        let product_id = seed_product(&product_repo, 2).await;
        let foreign_inspection = service
            .create_inspection(CreateInspection {
                tenant_id: 1,
                work_order_id: None,
                product_id: seed_product(&product_repo, 1).await,
                inspection_type: "Visual".to_string(),
                quantity_inspected: dec!(100),
                quantity_passed: dec!(95),
                quantity_failed: dec!(5),
                status: InspectionStatus::Passed,
                inspector_id: Some(1),
                notes: None,
            })
            .await
            .expect("seed foreign inspection");
        let create = CreateNonConformanceReport {
            tenant_id: 2,
            inspection_id: Some(foreign_inspection.id),
            product_id,
            ncr_type: NcrType::Minor,
            description: "Scratch on surface".to_string(),
            root_cause: None,
            corrective_action: None,
            raised_by: 1,
        };
        let result = service.create_ncr(create).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ApiError::NotFound(msg) if msg == "Inspection not found"
        ));
    }

    #[tokio::test]
    async fn test_create_inspection_rejects_foreign_inspector() {
        // Tenant-1 product (passes the product precheck) + tenant-2 inspector ->
        // NotFound on the inspector, isolating the inspector_id precheck.
        let (service, product_repo, _work_order_repo, user_repo) = create_qc_service().await;
        let foreign = foreign_inspector_id(&user_repo).await;
        let create = CreateInspection {
            tenant_id: 1,
            work_order_id: None,
            product_id: seed_product(&product_repo, 1).await,
            inspection_type: "Visual".to_string(),
            quantity_inspected: dec!(100),
            quantity_passed: dec!(95),
            quantity_failed: dec!(5),
            status: InspectionStatus::Passed,
            inspector_id: Some(foreign),
            notes: None,
        };
        let result = service.create_inspection(create).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ApiError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_update_inspection_rejects_foreign_inspector() {
        // The UPDATE path persists `inspector_id` via COALESCE, so re-stamping a
        // foreign user id onto an existing tenant-1 inspection must be rejected.
        let (service, product_repo, _work_order_repo, user_repo) = create_qc_service().await;
        let foreign = foreign_inspector_id(&user_repo).await;
        let inspection = service
            .create_inspection(CreateInspection {
                tenant_id: 1,
                work_order_id: None,
                product_id: seed_product(&product_repo, 1).await,
                inspection_type: "Visual".to_string(),
                quantity_inspected: dec!(100),
                quantity_passed: dec!(95),
                quantity_failed: dec!(5),
                status: InspectionStatus::Passed,
                inspector_id: Some(1),
                notes: None,
            })
            .await
            .expect("seed inspection");
        let result = service
            .update_inspection(
                inspection.id,
                1,
                UpdateInspection {
                    status: None,
                    quantity_passed: None,
                    quantity_failed: None,
                    inspector_id: Some(foreign),
                    notes: None,
                },
            )
            .await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ApiError::NotFound(_)));
    }
}
