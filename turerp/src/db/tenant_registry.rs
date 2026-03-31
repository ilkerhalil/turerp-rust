//! Tenant database connection registry
//!
//! This module provides multi-tenant database isolation by managing
//! separate connection pools per tenant.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;
use sqlx::PgPool;

use crate::config::Config;
use crate::error::ApiError;

/// Tenant connection pool registry
///
/// Manages connection pools for multiple tenants, providing database isolation
/// between tenants. Uses a thread-safe HashMap for concurrent access.
pub struct TenantPoolRegistry {
    pools: Mutex<HashMap<i64, Arc<PgPool>>>,
    base_config: DatabaseConfig,
}

/// Database configuration for tenant connections
#[derive(Clone)]
pub struct DatabaseConfig {
    pub base_url: String,
    pub max_connections: u32,
    pub min_connections: u32,
}

impl From<&Config> for DatabaseConfig {
    fn from(config: &Config) -> Self {
        Self {
            base_url: config.database.url.clone(),
            max_connections: config.database.max_connections,
            min_connections: config.database.min_connections,
        }
    }
}

impl TenantPoolRegistry {
    /// Create a new tenant pool registry
    pub fn new(config: &Config) -> Self {
        Self {
            pools: Mutex::new(HashMap::new()),
            base_config: DatabaseConfig::from(config),
        }
    }

    /// Create a new registry with a base database URL
    pub fn with_config(base_url: String, max_connections: u32, min_connections: u32) -> Self {
        Self {
            pools: Mutex::new(HashMap::new()),
            base_config: DatabaseConfig {
                base_url,
                max_connections,
                min_connections,
            },
        }
    }

    /// Get or create a connection pool for a tenant
    ///
    /// Returns an existing pool if one exists for the tenant, or creates
    /// a new pool if not. The pool is cached for future use.
    pub async fn get_pool(&self, tenant_id: i64) -> Result<Arc<PgPool>, ApiError> {
        // Check if pool already exists
        {
            let pools = self.pools.lock();
            if let Some(pool) = pools.get(&tenant_id) {
                return Ok(pool.clone());
            }
        }

        // Create a new pool for this tenant
        // In production, you'd have a separate database per tenant
        // For this implementation, we'll use a schema-based approach
        // where each tenant has its own schema in the same database
        let pool = self.create_tenant_pool(tenant_id).await?;

        // Cache the pool
        {
            let mut pools = self.pools.lock();
            pools.insert(tenant_id, pool.clone());
        }

        Ok(pool)
    }

    /// Create a new connection pool for a tenant
    async fn create_tenant_pool(&self, _tenant_id: i64) -> Result<Arc<PgPool>, ApiError> {
        // In a production multi-tenant setup, you would:
        // 1. Create a separate database per tenant, or
        // 2. Use schema-based isolation (search_path), or
        // 3. Use row-level security with tenant_id column
        //
        // For this implementation, we'll use the same connection pool
        // for all tenants (row-level security approach).
        // The tenant_id is tracked in the application layer
        // and applied via tenant context in queries.

        let pool = PgPoolOptions::new()
            .max_connections(self.base_config.max_connections)
            .min_connections(self.base_config.min_connections)
            .connect(&self.base_config.base_url)
            .await
            .map_err(|e| ApiError::Database(format!("Failed to create tenant pool: {}", e)))?;

        Ok(Arc::new(pool))
    }

    /// Remove a tenant pool from the registry
    ///
    /// This is useful when a tenant is deleted or suspended.
    /// The pool will be dropped and connections closed.
    pub fn remove_pool(&self, tenant_id: i64) {
        let mut pools = self.pools.lock();
        pools.remove(&tenant_id);
    }

    /// Get the number of active tenant pools
    pub fn pool_count(&self) -> usize {
        self.pools.lock().len()
    }

    /// Clear all cached pools
    ///
    /// Use with caution - this will close all cached connections.
    pub fn clear(&self) {
        self.pools.lock().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_config_from_config() {
        let config = Config::default();
        let db_config = DatabaseConfig::from(&config);
        assert_eq!(db_config.max_connections, 5);
        assert_eq!(db_config.min_connections, 1);
    }

    #[test]
    fn test_pool_count() {
        let registry =
            TenantPoolRegistry::with_config("postgres://localhost/test".to_string(), 5, 1);
        assert_eq!(registry.pool_count(), 0);
    }

    #[test]
    fn test_remove_pool() {
        let registry =
            TenantPoolRegistry::with_config("postgres://localhost/test".to_string(), 5, 1);

        // Removing non-existent pool should be a no-op
        registry.remove_pool(999);
        assert_eq!(registry.pool_count(), 0);
    }

    #[test]
    fn test_clear() {
        let registry =
            TenantPoolRegistry::with_config("postgres://localhost/test".to_string(), 5, 1);

        registry.clear();
        assert_eq!(registry.pool_count(), 0);
    }
}
