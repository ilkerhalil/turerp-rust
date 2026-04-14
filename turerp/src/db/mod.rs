//! Database layer

pub mod error;
pub mod pool;
pub mod tenant_registry;

pub use error::map_sqlx_error;
pub use pool::{create_pool, run_migrations};
pub use tenant_registry::{DatabaseConfig, TenantPoolRegistry};
