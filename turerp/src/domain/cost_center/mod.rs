//! Cost Center / Profit Center domain module
//!
//! Provides cost tracking and profitability analysis across business units.

pub mod model;
pub mod postgres_repository;
pub mod repository;
pub mod service;

pub use model::{
    AllocationResponse, BulkRestoreFailed, BulkRestoreResponse, CostCenter, CostCenterAllocation,
    CostCenterResponse, CostCenterType, CreateAllocation, CreateCostCenter, ProfitabilityReport,
    UpdateCostCenter,
};
pub use postgres_repository::PostgresCostCenterRepository;
pub use repository::{BoxCostCenterRepository, CostCenterRepository, InMemoryCostCenterRepository};
pub use service::CostCenterService;
