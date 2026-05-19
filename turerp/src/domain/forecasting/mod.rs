//! Inventory forecasting domain module
//!
//! Provides demand prediction, reorder suggestions, stock level alerts,
//! and forecast reports using simple statistical methods (moving average).

pub mod model;
pub mod postgres_repository;
pub mod repository;
pub mod service;

// Re-exports
pub use model::{
    DemandDataPoint, DemandForecast, ForecastPeriod, ForecastReport, ForecastRequest,
    ReorderRequest, ReorderSuggestion, ReorderUrgency, StockAlert, StockAlertRequest,
    StockAlertType,
};
pub use postgres_repository::PostgresForecastingRepository;
pub use repository::{
    BoxForecastingRepository, ForecastProduct, ForecastingRepository, HistoricalSale,
    InMemoryForecastingRepository,
};
pub use service::ForecastingService;
