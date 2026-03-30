//! Project service for business logic

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
}

impl ProjectService {
    pub fn new(
        project_repo: BoxProjectRepository,
        wbs_repo: BoxWbsItemRepository,
        cost_repo: BoxProjectCostRepository,
    ) -> Self {
        Self {
            project_repo,
            wbs_repo,
            cost_repo,
        }
    }
    pub async fn create_project(&self, create: CreateProject) -> Result<Project, ApiError> {
        self.project_repo.create(create).await
    }
    pub async fn get_project(&self, id: i64) -> Result<Project, ApiError> {
        self.project_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Project not found".to_string()))
    }
    pub async fn get_projects_by_tenant(&self, tenant_id: i64) -> Result<Vec<Project>, ApiError> {
        self.project_repo.find_by_tenant(tenant_id).await
    }
    pub async fn update_project_status(
        &self,
        id: i64,
        status: ProjectStatus,
    ) -> Result<Project, ApiError> {
        self.project_repo.update_status(id, status).await
    }
    pub async fn create_wbs_item(&self, create: CreateWbsItem) -> Result<WbsItem, ApiError> {
        self.wbs_repo.create(create).await
    }
    pub async fn get_wbs_by_project(&self, project_id: i64) -> Result<Vec<WbsItem>, ApiError> {
        self.wbs_repo.find_by_project(project_id).await
    }
    pub async fn update_wbs_progress(
        &self,
        id: i64,
        progress: f64,
        hours: f64,
    ) -> Result<WbsItem, ApiError> {
        self.wbs_repo.update_progress(id, progress, hours).await
    }
    pub async fn create_project_cost(
        &self,
        create: CreateProjectCost,
    ) -> Result<ProjectCost, ApiError> {
        self.cost_repo.create(create).await
    }
    pub async fn get_project_costs(&self, project_id: i64) -> Result<Vec<ProjectCost>, ApiError> {
        self.cost_repo.find_by_project(project_id).await
    }
    pub async fn get_profitability(
        &self,
        project_id: i64,
        revenue: f64,
    ) -> Result<ProjectProfitability, ApiError> {
        let project = self.get_project(project_id).await?;
        let actual_cost = self.cost_repo.find_total_by_project(project_id).await?;
        let profit = revenue - actual_cost;
        let margin = if revenue > 0.0 {
            (profit / revenue) * 100.0
        } else {
            0.0
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::project::model::CostType;
    use crate::domain::project::repository::{
        InMemoryProjectCostRepository, InMemoryProjectRepository, InMemoryWbsItemRepository,
    };
    use std::sync::Arc;

    fn create_service() -> ProjectService {
        let project_repo = Arc::new(InMemoryProjectRepository::new()) as BoxProjectRepository;
        let wbs_repo = Arc::new(InMemoryWbsItemRepository::new()) as BoxWbsItemRepository;
        let cost_repo = Arc::new(InMemoryProjectCostRepository::new()) as BoxProjectCostRepository;
        ProjectService::new(project_repo, wbs_repo, cost_repo)
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
            budget: 10000.0,
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
                budget: 1000.0,
            })
            .await
            .unwrap();
        let wbs = service
            .create_wbs_item(CreateWbsItem {
                project_id: project.id,
                parent_id: None,
                name: "Phase 1".to_string(),
                code: "1.0".to_string(),
                planned_hours: 40.0,
            })
            .await
            .unwrap();
        assert_eq!(wbs.planned_hours, 40.0);
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
                budget: 1000.0,
            })
            .await
            .unwrap();
        let cost = service
            .create_project_cost(CreateProjectCost {
                project_id: project.id,
                wbs_item_id: None,
                cost_type: CostType::Labor,
                amount: 500.0,
                description: "Work".to_string(),
                incurred_at: chrono::Utc::now(),
            })
            .await
            .unwrap();
        assert_eq!(cost.amount, 500.0);
    }
}
