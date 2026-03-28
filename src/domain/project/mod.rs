//! Project domain module

pub mod model;
pub mod repository;
pub mod service;

pub use model::{
    CostType, CreateProject, CreateProjectCost, CreateWbsItem, Project, ProjectCost,
    ProjectProfitability, ProjectStatus, WbsItem,
};
pub use repository::{
    BoxProjectCostRepository, BoxProjectRepository, BoxWbsItemRepository,
    InMemoryProjectCostRepository, InMemoryProjectRepository, InMemoryWbsItemRepository,
    ProjectCostRepository, ProjectRepository, WbsItemRepository,
};
pub use service::ProjectService;
