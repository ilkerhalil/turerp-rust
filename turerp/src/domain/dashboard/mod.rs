//! Dashboard domain module

pub mod model;
pub mod postgres_repository;
pub mod repository;
pub mod service;

pub use model::{
    AgingBucket, ChartData, ChartDataset, CreateWidgetConfig, DashboardFilter,
    DashboardWidgetConfig, ExpenseSummary, KpiFormat, KpiName, KpiResponse, KpiWidget,
    RevenueByCategory, SalesPeriod, TopProduct, WidgetPosition, WidgetType,
};
pub use postgres_repository::PostgresDashboardRepository;
pub use repository::{BoxDashboardRepository, DashboardRepository, InMemoryDashboardRepository};
pub use service::DashboardService;
