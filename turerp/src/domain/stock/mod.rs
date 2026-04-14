//! Stock domain module

pub mod model;
#[cfg(feature = "postgres")]
pub mod postgres_repository;
pub mod repository;
pub mod service;

// Re-exports
pub use model::{
    CreateStockMovement, CreateWarehouse, MovementType, StockLevel, StockMovement, StockSummary,
    Warehouse, WarehouseStock,
};
#[cfg(feature = "postgres")]
pub use postgres_repository::{
    PostgresStockLevelRepository, PostgresStockMovementRepository, PostgresWarehouseRepository,
};
pub use repository::{
    BoxStockLevelRepository, BoxStockMovementRepository, BoxWarehouseRepository,
    InMemoryStockLevelRepository, InMemoryStockMovementRepository, InMemoryWarehouseRepository,
    StockLevelRepository, StockMovementRepository, WarehouseRepository,
};
pub use service::StockService;
