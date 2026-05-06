//! Database layer

pub mod error;
pub mod pool;
pub mod tenant_registry;

#[cfg(feature = "postgres")]
pub mod job_repository;

pub use error::map_sqlx_error;
pub use pool::{create_pool, run_migrations};
pub use tenant_registry::{DatabaseConfig, TenantPoolRegistry};
