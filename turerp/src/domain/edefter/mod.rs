//! e-Defter domain module
//!
//! Provides Turkish e-Defter (electronic ledger) integration with GIB
//! (Gelir İdaresi Başkanlığı), including Yevmiye defteri, Büyük defter,
//! and Berat signing structures for ledger period management.

pub mod gib;
pub mod model;
pub mod repository;
pub mod service;

#[cfg(feature = "postgres")]
pub mod postgres_repository;

// Re-exports
pub use model::{
    BalanceCheckResult, BeratInfo, CreateLedgerPeriod, EDefterStatus, LedgerPeriod,
    LedgerPeriodResponse, LedgerType, YevmiyeEntry, YevmiyeLine,
};
pub use repository::{BoxEDefterRepository, EDefterRepository, InMemoryEDefterRepository};
pub use service::EDefterService;
