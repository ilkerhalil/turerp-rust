//! Project service for business logic

use rust_decimal::Decimal;

use crate::common::pagination::PaginatedResult;
use crate::domain::cari::repository::BoxCariRepository;
use crate::domain::project::model::{
    CreateProject, CreateProjectCost, CreateWbsItem, Project, ProjectCost, ProjectProfitability,
    ProjectStatus, WbsItem,
};
use crate::domain::project::repository::{
    BoxProjectCostRepository, BoxProjectRepository, BoxWbsItemRepository,
};
use crate::error::ApiError;

#[derive(Clone)]
pub struct ProjectService {
    project_repo: BoxProjectRepository,
    wbs_repo: BoxWbsItemRepository,
    cost_repo: BoxProjectCostRepository,
    cari_repo: BoxCariRepository,
}

impl ProjectService {
    pub fn new(
        project_repo: BoxProjectRepository,
        wbs_repo: BoxWbsItemRepository,
        cost_repo: BoxProjectCostRepository,
        cari_repo: BoxCariRepository,
    ) -> Self {
        Self {
            project_repo,
            wbs_repo,
            cost_repo,
            cari_repo,
        }
    }
    #[tracing::instrument(skip(self))]
    pub async fn create_project(&self, create: CreateProject) -> Result<Project, ApiError> {
        // Parent-ownership precheck: an optional cari_id must reference a cari
        // owned by the caller's tenant (auth-overwritten in create.tenant_id)
        // before the project INSERT, to prevent a cross-tenant orphan FK.
        if let Some(cari_id) = create.cari_id {
            self.cari_repo
                .find_by_id(cari_id, create.tenant_id)
                .await?
                .ok_or_else(|| ApiError::NotFound("Cari not found".to_string()))?;
        }
        self.project_repo.create(create).await
    }
    #[tracing::instrument(skip(self))]
    pub async fn get_project(&self, id: i64, tenant_id: i64) -> Result<Project, ApiError> {
        self.project_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Project not found".to_string()))
    }
    #[tracing::instrument(skip(self))]
    pub async fn get_projects_by_tenant(&self, tenant_id: i64) -> Result<Vec<Project>, ApiError> {
        self.project_repo.find_by_tenant(tenant_id).await
    }
    #[tracing::instrument(skip(self))]
    pub async fn get_projects_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Project>, ApiError> {
        let params = crate::common::pagination::PaginationParams { page, per_page };
        params.validate().map_err(ApiError::Validation)?;
        self.project_repo
            .find_by_tenant_paginated(tenant_id, page, per_page)
            .await
    }
    #[tracing::instrument(skip(self))]
    pub async fn update_project_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: ProjectStatus,
    ) -> Result<Project, ApiError> {
        self.project_repo.update_status(id, tenant_id, status).await
    }
    #[tracing::instrument(skip(self))]
    pub async fn create_wbs_item(
        &self,
        tenant_id: i64,
        create: CreateWbsItem,
    ) -> Result<WbsItem, ApiError> {
        // Parent-ownership precheck: project_id (required) must reference a
        // tenant-owned project; parent_id (optional, WBS self-reference) must
        // reference a tenant-owned WBS item — both before the INSERT.
        self.project_repo
            .find_by_id(create.project_id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Project not found".to_string()))?;
        if let Some(parent_id) = create.parent_id {
            self.wbs_repo
                .find_by_id(parent_id, tenant_id)
                .await?
                .ok_or_else(|| ApiError::NotFound("WBS parent not found".to_string()))?;
        }
        self.wbs_repo.create(tenant_id, create).await
    }
    #[tracing::instrument(skip(self))]
    pub async fn get_wbs_by_project(
        &self,
        project_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<WbsItem>, ApiError> {
        // Verify project belongs to tenant before delegating to repo.
        self.get_project(project_id, tenant_id).await?;
        self.wbs_repo.find_by_project(project_id, tenant_id).await
    }
    #[tracing::instrument(skip(self))]
    pub async fn get_wbs_item(&self, id: i64, tenant_id: i64) -> Result<WbsItem, ApiError> {
        self.wbs_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("WBS item not found".to_string()))
    }
    #[tracing::instrument(skip(self))]
    pub async fn update_wbs_progress(
        &self,
        id: i64,
        tenant_id: i64,
        progress: Decimal,
        hours: Decimal,
    ) -> Result<WbsItem, ApiError> {
        self.wbs_repo
            .update_progress(id, tenant_id, progress, hours)
            .await
    }
    #[tracing::instrument(skip(self))]
    pub async fn delete_wbs_item(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.wbs_repo.delete(id, tenant_id).await
    }
    #[tracing::instrument(skip(self))]
    pub async fn create_project_cost(
        &self,
        tenant_id: i64,
        create: CreateProjectCost,
    ) -> Result<ProjectCost, ApiError> {
        // Parent-ownership precheck: project_id (required) must reference a
        // tenant-owned project; wbs_item_id (optional) must reference a
        // tenant-owned WBS item — both before the INSERT.
        self.project_repo
            .find_by_id(create.project_id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Project not found".to_string()))?;
        if let Some(wbs_item_id) = create.wbs_item_id {
            self.wbs_repo
                .find_by_id(wbs_item_id, tenant_id)
                .await?
                .ok_or_else(|| ApiError::NotFound("WBS item not found".to_string()))?;
        }
        self.cost_repo.create(tenant_id, create).await
    }
    #[tracing::instrument(skip(self))]
    pub async fn get_project_costs(
        &self,
        project_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<ProjectCost>, ApiError> {
        // Verify project belongs to tenant before delegating to repo.
        self.get_project(project_id, tenant_id).await?;
        self.cost_repo.find_by_project(project_id, tenant_id).await
    }
    #[tracing::instrument(skip(self))]
    pub async fn get_profitability(
        &self,
        project_id: i64,
        tenant_id: i64,
        revenue: Decimal,
    ) -> Result<ProjectProfitability, ApiError> {
        let project = self.get_project(project_id, tenant_id).await?;
        let actual_cost = self
            .cost_repo
            .find_total_by_project(project_id, tenant_id)
            .await?;
        let profit = revenue - actual_cost;
        let margin = if revenue > Decimal::ZERO {
            (profit / revenue) * Decimal::ONE_HUNDRED
        } else {
            Decimal::ZERO
        };
        Ok(ProjectProfitability {
            project_id,
            project_name: project.name,
            budget: project.budget,
            actual_cost,
            revenue,
            profit,
            profit_margin: margin,
        })
    }

    // Soft delete operations
    #[tracing::instrument(skip(self))]
    pub async fn soft_delete_project(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.project_repo
            .soft_delete(id, tenant_id, deleted_by)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn restore_project(&self, id: i64, tenant_id: i64) -> Result<Project, ApiError> {
        self.project_repo.restore(id, tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn list_deleted_projects(&self, tenant_id: i64) -> Result<Vec<Project>, ApiError> {
        self.project_repo.find_deleted(tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn destroy_project(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.project_repo.destroy(id, tenant_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::cari::model::{CariType, CreateCari};
    use crate::domain::cari::repository::{BoxCariRepository, InMemoryCariRepository};
    use crate::domain::project::model::CostType;
    use crate::domain::project::repository::{
        InMemoryProjectCostRepository, InMemoryProjectRepository, InMemoryWbsItemRepository,
    };
    use rust_decimal_macros::dec;
    use std::sync::Arc;

    fn create_service() -> ProjectService {
        let project_repo = Arc::new(InMemoryProjectRepository::new()) as BoxProjectRepository;
        let wbs_repo = Arc::new(InMemoryWbsItemRepository::new()) as BoxWbsItemRepository;
        let cost_repo = Arc::new(InMemoryProjectCostRepository::new()) as BoxProjectCostRepository;
        let cari_repo = Arc::new(InMemoryCariRepository::new()) as BoxCariRepository;
        ProjectService::new(project_repo, wbs_repo, cost_repo, cari_repo)
    }

    /// Like `create_service` but also returns the cari repo so tests can seed
    /// caris on specific tenants for cross-tenant IDOR negatives.
    fn create_service_with_cari() -> (ProjectService, BoxCariRepository) {
        let project_repo = Arc::new(InMemoryProjectRepository::new()) as BoxProjectRepository;
        let wbs_repo = Arc::new(InMemoryWbsItemRepository::new()) as BoxWbsItemRepository;
        let cost_repo = Arc::new(InMemoryProjectCostRepository::new()) as BoxProjectCostRepository;
        let cari_repo = Arc::new(InMemoryCariRepository::new()) as BoxCariRepository;
        let service = ProjectService::new(project_repo, wbs_repo, cost_repo, cari_repo.clone());
        (service, cari_repo)
    }

    /// Helper: seed a cari on a tenant via the repo and return its id.
    async fn seed_cari(cari_repo: &BoxCariRepository, tenant_id: i64) -> i64 {
        let cari = cari_repo
            .create(CreateCari {
                code: format!("CARI-{}", tenant_id),
                name: format!("Cari T{}", tenant_id),
                cari_type: CariType::Customer,
                tax_number: None,
                tax_office: None,
                identity_number: None,
                email: None,
                phone: None,
                address: None,
                city: None,
                country: None,
                postal_code: None,
                credit_limit: Decimal::ZERO,
                default_currency: "TRY".to_string(),
                tenant_id,
                company_id: 0,
                created_by: 1,
            })
            .await
            .unwrap();
        cari.id
    }

    #[tokio::test]
    async fn test_create_project() {
        let service = create_service();
        let create = CreateProject {
            tenant_id: 1,
            name: "Test Project".to_string(),
            description: None,
            cari_id: None,
            start_date: None,
            end_date: None,
            budget: dec!(10000),
        };
        let result = service.create_project(create).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().status, ProjectStatus::Planning);
    }

    #[tokio::test]
    async fn test_create_wbs_item() {
        let service = create_service();
        let project = service
            .create_project(CreateProject {
                tenant_id: 1,
                name: "P1".to_string(),
                description: None,
                cari_id: None,
                start_date: None,
                end_date: None,
                budget: dec!(1000),
            })
            .await
            .unwrap();
        let wbs = service
            .create_wbs_item(
                1,
                CreateWbsItem {
                    project_id: project.id,
                    parent_id: None,
                    name: "Phase 1".to_string(),
                    code: "1.0".to_string(),
                    planned_hours: dec!(40),
                },
            )
            .await
            .unwrap();
        assert_eq!(wbs.planned_hours, dec!(40));
    }

    #[tokio::test]
    async fn test_project_cost() {
        let service = create_service();
        let project = service
            .create_project(CreateProject {
                tenant_id: 1,
                name: "P1".to_string(),
                description: None,
                cari_id: None,
                start_date: None,
                end_date: None,
                budget: dec!(1000),
            })
            .await
            .unwrap();
        let cost = service
            .create_project_cost(
                1,
                CreateProjectCost {
                    project_id: project.id,
                    wbs_item_id: None,
                    cost_type: CostType::Labor,
                    amount: dec!(500),
                    description: "Work".to_string(),
                    incurred_at: chrono::Utc::now(),
                },
            )
            .await
            .unwrap();
        assert_eq!(cost.amount, dec!(500));
    }

    #[tokio::test]
    async fn test_create_project_rejects_foreign_cari() {
        let (service, cari_repo) = create_service_with_cari();
        // Seed a cari on tenant 2, then try to attach it to a tenant-1 project.
        let foreign_cari_id = seed_cari(&cari_repo, 2).await;
        let result = service
            .create_project(CreateProject {
                tenant_id: 1,
                name: "P1".to_string(),
                description: None,
                cari_id: Some(foreign_cari_id),
                start_date: None,
                end_date: None,
                budget: dec!(1000),
            })
            .await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ApiError::NotFound(msg) if msg == "Cari not found"
        ));
    }

    #[tokio::test]
    async fn test_create_project_accepts_owned_cari() {
        let (service, cari_repo) = create_service_with_cari();
        let owned_cari_id = seed_cari(&cari_repo, 1).await;
        let result = service
            .create_project(CreateProject {
                tenant_id: 1,
                name: "P1".to_string(),
                description: None,
                cari_id: Some(owned_cari_id),
                start_date: None,
                end_date: None,
                budget: dec!(1000),
            })
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_wbs_item_rejects_foreign_project() {
        let service = create_service();
        // Create a project on tenant 2.
        let foreign_project = service
            .create_project(CreateProject {
                tenant_id: 2,
                name: "Foreign".to_string(),
                description: None,
                cari_id: None,
                start_date: None,
                end_date: None,
                budget: dec!(1000),
            })
            .await
            .unwrap();
        // Try to attach a WBS item to it as tenant 1.
        let result = service
            .create_wbs_item(
                1,
                CreateWbsItem {
                    project_id: foreign_project.id,
                    parent_id: None,
                    name: "Phase 1".to_string(),
                    code: "1.0".to_string(),
                    planned_hours: dec!(40),
                },
            )
            .await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ApiError::NotFound(msg) if msg == "Project not found"
        ));
    }

    #[tokio::test]
    async fn test_create_wbs_item_rejects_foreign_parent() {
        let service = create_service();
        // Owned project on tenant 1.
        let project = service
            .create_project(CreateProject {
                tenant_id: 1,
                name: "P1".to_string(),
                description: None,
                cari_id: None,
                start_date: None,
                end_date: None,
                budget: dec!(1000),
            })
            .await
            .unwrap();
        // Foreign WBS parent on tenant 2 (under a tenant-2 project).
        let foreign_project = service
            .create_project(CreateProject {
                tenant_id: 2,
                name: "P2".to_string(),
                description: None,
                cari_id: None,
                start_date: None,
                end_date: None,
                budget: dec!(1000),
            })
            .await
            .unwrap();
        let foreign_parent = service
            .create_wbs_item(
                2,
                CreateWbsItem {
                    project_id: foreign_project.id,
                    parent_id: None,
                    name: "Foreign Phase".to_string(),
                    code: "1.0".to_string(),
                    planned_hours: dec!(40),
                },
            )
            .await
            .unwrap();
        // Tenant 1 references the tenant-2 WBS item as parent.
        let result = service
            .create_wbs_item(
                1,
                CreateWbsItem {
                    project_id: project.id,
                    parent_id: Some(foreign_parent.id),
                    name: "Phase 1".to_string(),
                    code: "1.0".to_string(),
                    planned_hours: dec!(40),
                },
            )
            .await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ApiError::NotFound(msg) if msg == "WBS parent not found"
        ));
    }

    #[tokio::test]
    async fn test_create_project_cost_rejects_foreign_project() {
        let service = create_service();
        let foreign_project = service
            .create_project(CreateProject {
                tenant_id: 2,
                name: "Foreign".to_string(),
                description: None,
                cari_id: None,
                start_date: None,
                end_date: None,
                budget: dec!(1000),
            })
            .await
            .unwrap();
        let result = service
            .create_project_cost(
                1,
                CreateProjectCost {
                    project_id: foreign_project.id,
                    wbs_item_id: None,
                    cost_type: CostType::Labor,
                    amount: dec!(500),
                    description: "Work".to_string(),
                    incurred_at: chrono::Utc::now(),
                },
            )
            .await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ApiError::NotFound(msg) if msg == "Project not found"
        ));
    }

    #[tokio::test]
    async fn test_create_project_cost_rejects_foreign_wbs_item() {
        let service = create_service();
        // Owned project on tenant 1.
        let project = service
            .create_project(CreateProject {
                tenant_id: 1,
                name: "P1".to_string(),
                description: None,
                cari_id: None,
                start_date: None,
                end_date: None,
                budget: dec!(1000),
            })
            .await
            .unwrap();
        // Foreign WBS item on tenant 2.
        let foreign_project = service
            .create_project(CreateProject {
                tenant_id: 2,
                name: "P2".to_string(),
                description: None,
                cari_id: None,
                start_date: None,
                end_date: None,
                budget: dec!(1000),
            })
            .await
            .unwrap();
        let foreign_wbs = service
            .create_wbs_item(
                2,
                CreateWbsItem {
                    project_id: foreign_project.id,
                    parent_id: None,
                    name: "Foreign Phase".to_string(),
                    code: "1.0".to_string(),
                    planned_hours: dec!(40),
                },
            )
            .await
            .unwrap();
        // Tenant 1 references the tenant-2 WBS item on a cost.
        let result = service
            .create_project_cost(
                1,
                CreateProjectCost {
                    project_id: project.id,
                    wbs_item_id: Some(foreign_wbs.id),
                    cost_type: CostType::Labor,
                    amount: dec!(500),
                    description: "Work".to_string(),
                    incurred_at: chrono::Utc::now(),
                },
            )
            .await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ApiError::NotFound(msg) if msg == "WBS item not found"
        ));
    }
}
