//! Cari (Customer/Vendor) module

pub mod model;
pub mod repository;
pub mod service;

// Re-exports
pub use model::{Cari, CariResponse, CariStatus, CariType, CreateCari, UpdateCari};
pub use repository::{BoxCariRepository, CariRepository, InMemoryCariRepository};
pub use service::CariService;
