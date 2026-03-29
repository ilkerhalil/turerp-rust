//! Cari (Customer/Vendor) module

pub mod model;
#[cfg(feature = "postgres")]
pub mod postgres_repository;
pub mod repository;
pub mod service;

// Re-exports
pub use model::{Cari, CariResponse, CariStatus, CariType, CreateCari, UpdateCari};
#[cfg(feature = "postgres")]
pub use postgres_repository::PostgresCariRepository;
pub use repository::{BoxCariRepository, CariRepository, InMemoryCariRepository};
pub use service::CariService;
