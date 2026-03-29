//! Tenant domain module

pub mod model;
#[cfg(feature = "postgres")]
pub mod postgres_repository;
pub mod repository;
pub mod service;

// Re-exports
pub use model::{generate_db_name, CreateTenant, Tenant, UpdateTenant};
#[cfg(feature = "postgres")]
pub use postgres_repository::PostgresTenantRepository;
pub use repository::{BoxTenantRepository, InMemoryTenantRepository, TenantRepository};
pub use service::TenantService;
