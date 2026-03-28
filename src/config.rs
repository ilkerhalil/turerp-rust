//! Configuration management for Turerp ERP

use config::{ConfigError, File};
use serde::Deserialize;
use std::fmt;

/// Server configuration
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8000,
        }
    }
}

/// Database configuration
#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgres://postgres:postgres@localhost:5432/turerp".to_string(),
            max_connections: 10,
            min_connections: 5,
        }
    }
}

/// JWT configuration
#[derive(Debug, Clone, Deserialize)]
pub struct JwtConfig {
    pub secret: String,
    pub access_token_expiration: i64,
    pub refresh_token_expiration: i64,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: "your-secret-key-change-in-production".to_string(),
            access_token_expiration: 3600,    // 1 hour
            refresh_token_expiration: 604800, // 7 days
        }
    }
}

/// Application configuration
#[derive(Debug, Clone, Deserialize, Default)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub jwt: JwtConfig,
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Config(server: {}:{}, db: {})",
            self.server.host, self.server.port, self.database.url
        )
    }
}

impl Config {
    /// Load configuration from a file and environment variables
    pub fn new() -> Result<Self, ConfigError> {
        let config = config::Config::builder()
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 8000)?
            .set_default("database.max_connections", 10)?
            .set_default("database.min_connections", 5)?
            .set_default("jwt.access_token_expiration", 3600)?
            .set_default("jwt.refresh_token_expiration", 604800)?
            .add_source(File::with_name("config/settings").required(false))
            .add_source(config::Environment::default().prefix("TURERP"))
            .build()?;

        config.try_deserialize()
    }

    /// Get database URL for master database
    pub fn master_database_url(&self) -> String {
        self.database.url.clone()
    }

    /// Get database URL for a specific tenant
    pub fn tenant_database_url(&self, db_name: &str) -> String {
        // Replace the database name in the URL
        let base = self.database.url.clone();
        if let Some(idx) = base.rfind('/') {
            format!("{}/{}", &base[..idx], db_name)
        } else {
            base
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.server.port, 8000);
        assert_eq!(config.jwt.access_token_expiration, 3600);
    }

    #[test]
    fn test_tenant_database_url() {
        let config = Config::default();
        let tenant_url = config.tenant_database_url("tenant_abc");
        assert!(tenant_url.contains("tenant_abc"));
    }
}
