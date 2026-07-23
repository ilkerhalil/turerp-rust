//! POS (Point of Sale) integration domain module

pub mod model;
pub mod repository;
pub mod service;

pub use model::{
    CreatePosSale, CreatePosSaleLine, CreatePosTerminal, CreateZReport, PosSale, PosSaleLine,
    PosSaleResponse, PosTerminal, PosTerminalResponse, PosTerminalStatus, SyncQueueItem,
    SyncQueueStatus, UpdatePosTerminal, ZReport, ZReportResponse, ZReportStatus,
};
pub use repository::{
    BoxPosSaleRepository, BoxPosTerminalRepository, BoxZReportRepository,
    InMemoryPosSaleRepository, InMemoryPosTerminalRepository, InMemoryZReportRepository,
    PosSaleRepository, PosTerminalRepository, ZReportRepository,
};
pub use service::PosService;
