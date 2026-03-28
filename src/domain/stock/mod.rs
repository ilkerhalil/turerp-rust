//! Stock domain module

pub mod model;
pub mod repository;
pub mod service;

// Re-exports
pub use model::{
    CreateStockMovement, CreateWarehouse, MovementType, StockLevel, StockMovement, StockSummary,
    Warehouse, WarehouseStock,
};
pub use repository::{
    BoxStockLevelRepository, BoxStockMovementRepository, BoxWarehouseRepository,
    InMemoryStockLevelRepository, InMemoryStockMovementRepository, InMemoryWarehouseRepository,
    StockLevelRepository, StockMovementRepository, WarehouseRepository,
};
pub use service::StockService;
