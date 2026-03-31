//! Tenant domain module

pub mod model;
#[cfg(feature = "postgres")]
pub mod postgres_repository;
pub mod repository;
pub mod service;

// Re-exports
pub use model::{
    generate_db_name, CreateTenant, CreateTenantConfig, Tenant, TenantConfig, TenantConfigResponse,
    UpdateTenant, UpdateTenantConfig,
};
#[cfg(feature = "postgres")]
pub use postgres_repository::PostgresTenantRepository;
pub use repository::{
    BoxTenantConfigRepository, BoxTenantRepository, InMemoryTenantConfigRepository,
    InMemoryTenantRepository, TenantConfigRepository, TenantRepository,
};
pub use service::TenantService;
