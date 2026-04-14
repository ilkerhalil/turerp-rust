//! Project domain module

pub mod model;
#[cfg(feature = "postgres")]
pub mod postgres_repository;
pub mod repository;
pub mod service;

pub use model::{
    CostType, CreateProject, CreateProjectCost, CreateWbsItem, Project, ProjectCost,
    ProjectProfitability, ProjectStatus, WbsItem,
};
#[cfg(feature = "postgres")]
pub use postgres_repository::{
    PostgresProjectCostRepository, PostgresProjectRepository, PostgresWbsItemRepository,
};
pub use repository::{
    BoxProjectCostRepository, BoxProjectRepository, BoxWbsItemRepository,
    InMemoryProjectCostRepository, InMemoryProjectRepository, InMemoryWbsItemRepository,
    ProjectCostRepository, ProjectRepository, WbsItemRepository,
};
pub use service::ProjectService;
