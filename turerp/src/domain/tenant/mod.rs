//! Tenant domain module

pub mod model;
pub mod repository;
pub mod service;

// Re-exports
pub use model::{generate_db_name, CreateTenant, Tenant, UpdateTenant};
pub use repository::{BoxTenantRepository, InMemoryTenantRepository, TenantRepository};
pub use service::TenantService;
