//! Project repository

use async_trait::async_trait;
use chrono::Utc;
use parking_lot::Mutex;
use std::sync::Arc;

use crate::domain::project::model::{
    CreateProject, CreateProjectCost, CreateWbsItem, Project, ProjectCost, ProjectStatus, WbsItem,
};
use crate::error::ApiError;

#[async_trait]
pub trait ProjectRepository: Send + Sync {
    async fn create(&self, project: CreateProject) -> Result<Project, ApiError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<Project>, ApiError>;
    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Project>, ApiError>;
    async fn find_by_cari(&self, cari_id: i64) -> Result<Vec<Project>, ApiError>;
    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: ProjectStatus,
    ) -> Result<Vec<Project>, ApiError>;
    async fn update_status(&self, id: i64, status: ProjectStatus) -> Result<Project, ApiError>;
    async fn update_actual_cost(&self, id: i64, cost: f64) -> Result<Project, ApiError>;
    async fn delete(&self, id: i64) -> Result<(), ApiError>;
}

#[async_trait]
pub trait WbsItemRepository: Send + Sync {
    async fn create(&self, item: CreateWbsItem) -> Result<WbsItem, ApiError>;
    async fn find_by_project(&self, project_id: i64) -> Result<Vec<WbsItem>, ApiError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<WbsItem>, ApiError>;
    async fn update_progress(
        &self,
        id: i64,
        progress: f64,
        hours: f64,
    ) -> Result<WbsItem, ApiError>;
    async fn delete(&self, id: i64) -> Result<(), ApiError>;
}

#[async_trait]
pub trait ProjectCostRepository: Send + Sync {
    async fn create(&self, cost: CreateProjectCost) -> Result<ProjectCost, ApiError>;
    async fn find_by_project(&self, project_id: i64) -> Result<Vec<ProjectCost>, ApiError>;
    async fn find_total_by_project(&self, project_id: i64) -> Result<f64, ApiError>;
    async fn delete(&self, id: i64) -> Result<(), ApiError>;
}

pub type BoxProjectRepository = Arc<dyn ProjectRepository>;
pub type BoxWbsItemRepository = Arc<dyn WbsItemRepository>;
pub type BoxProjectCostRepository = Arc<dyn ProjectCostRepository>;

pub struct InMemoryProjectRepository {
    projects: Mutex<std::collections::HashMap<i64, Project>>,
    next_id: Mutex<i64>,
}

impl InMemoryProjectRepository {
    pub fn new() -> Self {
        Self {
            projects: Mutex::new(std::collections::HashMap::new()),
            next_id: Mutex::new(1),
        }
    }
}
impl Default for InMemoryProjectRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProjectRepository for InMemoryProjectRepository {
    async fn create(&self, create: CreateProject) -> Result<Project, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;
        let mut next_id = self.next_id.lock();
        let id = *next_id;
        *next_id += 1;
        let now = Utc::now();
        let project = Project {
            id,
            tenant_id: create.tenant_id,
            name: create.name,
            description: create.description,
            cari_id: create.cari_id,
            status: ProjectStatus::Planning,
            start_date: create.start_date,
            end_date: create.end_date,
            budget: create.budget,
            actual_cost: 0.0,
            created_at: now,
            updated_at: now,
        };
        self.projects.lock().insert(id, project.clone());
        Ok(project)
    }
    async fn find_by_id(&self, id: i64) -> Result<Option<Project>, ApiError> {
        Ok(self.projects.lock().get(&id).cloned())
    }
    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Project>, ApiError> {
        let p = self.projects.lock();
        Ok(p.values()
            .filter(|x| x.tenant_id == tenant_id)
            .cloned()
            .collect())
    }
    async fn find_by_cari(&self, cari_id: i64) -> Result<Vec<Project>, ApiError> {
        let p = self.projects.lock();
        Ok(p.values()
            .filter(|x| x.cari_id == Some(cari_id))
            .cloned()
            .collect())
    }
    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: ProjectStatus,
    ) -> Result<Vec<Project>, ApiError> {
        let p = self.projects.lock();
        Ok(p.values()
            .filter(|x| x.tenant_id == tenant_id && x.status == status)
            .cloned()
            .collect())
    }
    async fn update_status(&self, id: i64, status: ProjectStatus) -> Result<Project, ApiError> {
        let mut p = self.projects.lock();
        let proj = p
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound("Project not found".to_string()))?;
        proj.status = status;
        proj.updated_at = Utc::now();
        Ok(proj.clone())
    }
    async fn update_actual_cost(&self, id: i64, cost: f64) -> Result<Project, ApiError> {
        let mut p = self.projects.lock();
        let proj = p
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound("Project not found".to_string()))?;
        proj.actual_cost = cost;
        proj.updated_at = Utc::now();
        Ok(proj.clone())
    }
    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        self.projects.lock().remove(&id);
        Ok(())
    }
}

pub struct InMemoryWbsItemRepository {
    items: Mutex<std::collections::HashMap<i64, WbsItem>>,
    next_id: Mutex<i64>,
}
impl InMemoryWbsItemRepository {
    pub fn new() -> Self {
        Self {
            items: Mutex::new(std::collections::HashMap::new()),
            next_id: Mutex::new(1),
        }
    }
}
impl Default for InMemoryWbsItemRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WbsItemRepository for InMemoryWbsItemRepository {
    async fn create(&self, create: CreateWbsItem) -> Result<WbsItem, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;
        let mut next_id = self.next_id.lock();
        let id = *next_id;
        *next_id += 1;
        let item = WbsItem {
            id,
            project_id: create.project_id,
            parent_id: create.parent_id,
            name: create.name,
            code: create.code,
            planned_hours: create.planned_hours,
            actual_hours: 0.0,
            progress: 0.0,
            sort_order: *next_id as i32,
        };
        self.items.lock().insert(id, item.clone());
        Ok(item)
    }
    async fn find_by_project(&self, project_id: i64) -> Result<Vec<WbsItem>, ApiError> {
        let i = self.items.lock();
        Ok(i.values()
            .filter(|x| x.project_id == project_id)
            .cloned()
            .collect())
    }
    async fn find_by_id(&self, id: i64) -> Result<Option<WbsItem>, ApiError> {
        Ok(self.items.lock().get(&id).cloned())
    }
    async fn update_progress(
        &self,
        id: i64,
        progress: f64,
        hours: f64,
    ) -> Result<WbsItem, ApiError> {
        let mut i = self.items.lock();
        let item = i
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound("WBS item not found".to_string()))?;
        item.progress = progress;
        item.actual_hours = hours;
        Ok(item.clone())
    }
    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        self.items.lock().remove(&id);
        Ok(())
    }
}

pub struct InMemoryProjectCostRepository {
    costs: Mutex<std::collections::HashMap<i64, ProjectCost>>,
    next_id: Mutex<i64>,
}
impl InMemoryProjectCostRepository {
    pub fn new() -> Self {
        Self {
            costs: Mutex::new(std::collections::HashMap::new()),
            next_id: Mutex::new(1),
        }
    }
}
impl Default for InMemoryProjectCostRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProjectCostRepository for InMemoryProjectCostRepository {
    async fn create(&self, create: CreateProjectCost) -> Result<ProjectCost, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;
        let mut next_id = self.next_id.lock();
        let id = *next_id;
        *next_id += 1;
        let cost = ProjectCost {
            id,
            project_id: create.project_id,
            wbs_item_id: create.wbs_item_id,
            cost_type: create.cost_type,
            amount: create.amount,
            description: create.description,
            incurred_at: create.incurred_at,
            created_at: Utc::now(),
        };
        self.costs.lock().insert(id, cost.clone());
        Ok(cost)
    }
    async fn find_by_project(&self, project_id: i64) -> Result<Vec<ProjectCost>, ApiError> {
        let c = self.costs.lock();
        Ok(c.values()
            .filter(|x| x.project_id == project_id)
            .cloned()
            .collect())
    }
    async fn find_total_by_project(&self, project_id: i64) -> Result<f64, ApiError> {
        let c = self.costs.lock();
        Ok(c.values()
            .filter(|x| x.project_id == project_id)
            .map(|x| x.amount)
            .sum())
    }
    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        self.costs.lock().remove(&id);
        Ok(())
    }
}
