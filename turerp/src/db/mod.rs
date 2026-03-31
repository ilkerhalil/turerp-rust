//! Database layer

pub mod pool;
pub mod tenant_registry;

pub use pool::{create_pool, run_migrations};
pub use tenant_registry::{DatabaseConfig, TenantPoolRegistry};
