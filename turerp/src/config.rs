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

impl DatabaseConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let url = std::env::var("TURERP_DATABASE_URL")
            .map_err(|_| ConfigError::Message(
                "TURERP_DATABASE_URL must be set".to_string()
            ))?;

        Ok(Self {
            url,
            max_connections: std::env::var("TURERP_DB_MAX_CONNECTIONS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
            min_connections: std::env::var("TURERP_DB_MIN_CONNECTIONS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5),
        })
    }
}

/// JWT configuration
#[derive(Debug, Clone, Deserialize)]
pub struct JwtConfig {
    pub secret: String,
    pub access_token_expiration: i64,
    pub refresh_token_expiration: i64,
}

impl JwtConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let secret = std::env::var("TURERP_JWT_SECRET")
            .map_err(|_| ConfigError::Message(
                "TURERP_JWT_SECRET must be set".to_string()
            ))?;

        Ok(Self {
            secret,
            access_token_expiration: std::env::var("TURERP_JWT_ACCESS_EXPIRATION")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3600),
            refresh_token_expiration: std::env::var("TURERP_JWT_REFRESH_EXPIRATION")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(604800),
        })
    }
}

/// Application configuration
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub jwt: JwtConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            database: DatabaseConfig {
                url: "postgres://postgres:postgres@localhost:5432/turerp".to_string(),
                max_connections: 10,
                min_connections: 5,
            },
            jwt: JwtConfig {
                secret: "dev-secret-do-not-use-in-production".to_string(),
                access_token_expiration: 3600,
                refresh_token_expiration: 604800,
            },
        }
    }
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
    /// Load configuration from environment variables
    ///
    /// Required environment variables:
    /// - TURERP_DATABASE_URL: PostgreSQL connection string
    /// - TURERP_JWT_SECRET: Secret key for JWT tokens
    ///
    /// Optional environment variables:
    /// - TURERP_SERVER_HOST: Server host (default: 0.0.0.0)
    /// - TURERP_SERVER_PORT: Server port (default: 8000)
    /// - TURERP_DB_MAX_CONNECTIONS: Max DB connections (default: 10)
    /// - TURERP_DB_MIN_CONNECTIONS: Min DB connections (default: 5)
    /// - TURERP_JWT_ACCESS_EXPIRATION: Access token expiry in seconds (default: 3600)
    /// - TURERP_JWT_REFRESH_EXPIRATION: Refresh token expiry in seconds (default: 604800)
    pub fn new() -> Result<Self, ConfigError> {
        let server = ServerConfig {
            host: std::env::var("TURERP_SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("TURERP_SERVER_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8000),
        };

        let database = DatabaseConfig::from_env()?;
        let jwt = JwtConfig::from_env()?;

        Ok(Self {
            server,
            database,
            jwt,
        })
    }

    /// Get database URL reference for master database
    pub fn master_database_url(&self) -> &str {
        &self.database.url
    }

    /// Get database URL for a specific tenant
    pub fn tenant_database_url(&self, db_name: &str) -> String {
        if let Some(idx) = self.database.url.rfind('/') {
            format!("{}/{}", &self.database.url[..idx], db_name)
        } else {
            self.database.url.clone()
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
