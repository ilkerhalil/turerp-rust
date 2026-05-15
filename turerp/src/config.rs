//! Configuration management for Turerp ERP

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use config::ConfigError;
use serde::Deserialize;
use std::fmt;

/// Application environment
#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    #[default]
    Development,
    Production,
}

impl fmt::Display for Environment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Environment::Development => write!(f, "development"),
            Environment::Production => write!(f, "production"),
        }
    }
}

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
            .map_err(|_| ConfigError::Message("TURERP_DATABASE_URL must be set".to_string()))?;

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

/// Redis configuration
#[derive(Debug, Clone, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub enabled: bool,
    pub ttl_seconds: u64,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://127.0.0.1:6379".to_string(),
            enabled: false,
            ttl_seconds: 300,
        }
    }
}

impl RedisConfig {
    pub fn from_env() -> Self {
        let enabled = std::env::var("TURERP_REDIS_ENABLED")
            .ok()
            .map(|v| v.to_lowercase() == "true" || v == "1")
            .unwrap_or(false);

        Self {
            url: std::env::var("REDIS_URL")
                .or_else(|_| std::env::var("TURERP_REDIS_URL"))
                .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string()),
            enabled,
            ttl_seconds: std::env::var("TURERP_REDIS_TTL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(300),
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

impl JwtConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let secret = std::env::var("TURERP_JWT_SECRET")
            .map_err(|_| ConfigError::Message("TURERP_JWT_SECRET must be set".to_string()))?;

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

    /// Create a dev configuration (only for development/testing)
    #[cfg(any(test, debug_assertions))]
    pub fn dev() -> Self {
        Self {
            secret: "dev-secret-key-change-in-production-12345".to_string(),
            access_token_expiration: 3600,
            refresh_token_expiration: 604800,
        }
    }
}

/// CORS configuration
#[derive(Debug, Clone, Deserialize)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
    pub allowed_methods: Vec<String>,
    pub allowed_headers: Vec<String>,
    pub allow_credentials: bool,
    pub max_age: Option<u32>,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: vec!["*".to_string()],
            allowed_methods: vec![
                "GET".to_string(),
                "POST".to_string(),
                "PUT".to_string(),
                "DELETE".to_string(),
                "OPTIONS".to_string(),
            ],
            allowed_headers: vec!["Content-Type".to_string(), "Authorization".to_string()],
            allow_credentials: true,
            max_age: Some(3600),
        }
    }
}

impl CorsConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let allowed_origins: Vec<String> = std::env::var("TURERP_CORS_ORIGINS")
            .ok()
            .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default();

        Ok(Self {
            allowed_origins: if allowed_origins.is_empty() {
                vec!["*".to_string()]
            } else {
                allowed_origins
            },
            allowed_methods: std::env::var("TURERP_CORS_METHODS")
                .ok()
                .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_else(|| {
                    vec![
                        "GET".to_string(),
                        "POST".to_string(),
                        "PUT".to_string(),
                        "DELETE".to_string(),
                        "OPTIONS".to_string(),
                    ]
                }),
            allowed_headers: std::env::var("TURERP_CORS_HEADERS")
                .ok()
                .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_else(|| vec!["Content-Type".to_string(), "Authorization".to_string()]),
            allow_credentials: std::env::var("TURERP_CORS_CREDENTIALS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(true),
            max_age: std::env::var("TURERP_CORS_MAX_AGE")
                .ok()
                .and_then(|v| v.parse().ok()),
        })
    }

    /// Check if wildcard origin is allowed
    pub fn is_wildcard(&self) -> bool {
        self.allowed_origins.iter().any(|o| o == "*")
    }
}

/// Rate limiting configuration
#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitConfig {
    /// Trusted proxy IPs that may set X-Forwarded-For headers
    pub trusted_proxies: Vec<String>,
    /// Maximum requests per minute per IP
    pub requests_per_minute: u32,
    /// Maximum burst size
    pub burst_size: u32,
}

/// Metrics configuration
#[derive(Debug, Clone, Deserialize)]
pub struct MetricsConfig {
    /// Whether metrics collection is enabled
    pub enabled: bool,
    /// Path for the metrics endpoint
    pub path: String,
    /// OTLP endpoint for Aspire Dashboard (e.g. http://localhost:4317)
    pub otlp_endpoint: String,
    /// Whether OTLP export is enabled
    pub otlp_enabled: bool,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            path: "/metrics".to_string(),
            otlp_endpoint: "http://localhost:4317".to_string(),
            otlp_enabled: false,
        }
    }
}

impl MetricsConfig {
    pub fn from_env() -> Self {
        Self {
            enabled: std::env::var("TURERP_METRICS_ENABLED")
                .ok()
                .map(|v| v == "true" || v == "1")
                .unwrap_or(true),
            path: std::env::var("TURERP_METRICS_PATH")
                .ok()
                .unwrap_or_else(|| "/metrics".to_string()),
            otlp_endpoint: std::env::var("TURERP_OTLP_ENDPOINT")
                .ok()
                .unwrap_or_else(|| "http://localhost:4317".to_string()),
            otlp_enabled: std::env::var("TURERP_OTLP_ENABLED")
                .ok()
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            trusted_proxies: Vec::new(),
            requests_per_minute: 10,
            burst_size: 3,
        }
    }
}

impl RateLimitConfig {
    pub fn from_env() -> Self {
        let trusted_proxies = std::env::var("TURERP_TRUSTED_PROXIES")
            .ok()
            .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default();

        Self {
            trusted_proxies,
            requests_per_minute: std::env::var("TURERP_RATE_LIMIT_REQUESTS_PER_MINUTE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
            burst_size: std::env::var("TURERP_RATE_LIMIT_BURST")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3),
        }
    }

    /// Check if trusted proxies are configured
    pub fn has_trusted_proxies(&self) -> bool {
        !self.trusted_proxies.is_empty()
    }
}

/// Localization configuration
#[derive(Debug, Clone, Deserialize)]
pub struct LocalizationConfig {
    pub default_locale: String,
}

impl Default for LocalizationConfig {
    fn default() -> Self {
        Self {
            default_locale: "en".to_string(),
        }
    }
}

impl LocalizationConfig {
    pub fn from_env() -> Self {
        Self {
            default_locale: std::env::var("TURERP_DEFAULT_LOCALE")
                .ok()
                .unwrap_or_else(|| "en".to_string()),
        }
    }
}

/// CDC configuration
#[derive(Debug, Clone, Deserialize)]
pub struct CdcConfig {
    pub enabled: bool,
    pub channels: Vec<String>,
}

impl Default for CdcConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            channels: vec![
                "invoice_changes".to_string(),
                "cari_changes".to_string(),
                "stock_changes".to_string(),
                "payment_changes".to_string(),
            ],
        }
    }
}

impl CdcConfig {
    pub fn from_env() -> Self {
        let enabled = std::env::var("TURERP_CDC_ENABLED")
            .ok()
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);
        let channels = std::env::var("TURERP_CDC_CHANNELS")
            .ok()
            .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_else(|| Self::default().channels);
        Self { enabled, channels }
    }
}

/// Security headers configuration
#[derive(Debug, Clone, Deserialize)]
pub struct SecurityHeadersConfig {
    pub enabled: bool,
}

/// Encryption configuration
#[derive(Debug, Clone, Deserialize)]
pub struct EncryptionConfig {
    pub key: String,
}

/// Secrets management configuration
#[derive(Debug, Clone)]
pub struct SecretsConfig {
    pub vault_enabled: bool,
    pub vault_addr: String,
    pub vault_token: String,
    pub vault_mount: String,
    pub fallback_to_env: bool,
}

impl Default for SecurityHeadersConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

impl SecurityHeadersConfig {
    pub fn from_env() -> Self {
        Self {
            enabled: std::env::var("TURERP_SECURITY_HEADERS_ENABLED")
                .ok()
                .map(|v| v == "true" || v == "1")
                .unwrap_or(true),
        }
    }
}

pub struct Config {
    pub environment: Environment,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub jwt: JwtConfig,
    pub cors: CorsConfig,
    pub rate_limit: RateLimitConfig,
    pub metrics: MetricsConfig,
    pub localization: LocalizationConfig,
    pub cdc: CdcConfig,
    pub security_headers: SecurityHeadersConfig,
    pub secrets: SecretsConfig,
    pub encryption_key: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            environment: Environment::Development,
            server: ServerConfig::default(),
            database: DatabaseConfig {
                url: String::new(),
                max_connections: 10,
                min_connections: 5,
            },
            redis: RedisConfig::default(),
            jwt: JwtConfig {
                secret: String::new(),
                access_token_expiration: 3600,
                refresh_token_expiration: 604800,
            },
            cors: CorsConfig::default(),
            rate_limit: RateLimitConfig::default(),
            metrics: MetricsConfig::default(),
            localization: LocalizationConfig::default(),
            cdc: CdcConfig::default(),
            security_headers: SecurityHeadersConfig::default(),
            secrets: SecretsConfig::default(),
            encryption_key: "dGVzdC1rZXktZm9yLXRlc3Rpbmctb25seS0xMjM0NTY=".to_string(),
        }
    }
}

impl SecretsConfig {
    pub fn from_env() -> Self {
        let vault_enabled = std::env::var("TURERP_VAULT_ENABLED")
            .ok()
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

        Self {
            vault_enabled,
            vault_addr: std::env::var("TURERP_VAULT_ADDR")
                .unwrap_or_else(|_| "http://127.0.0.1:8200".to_string()),
            vault_token: std::env::var("TURERP_VAULT_TOKEN").unwrap_or_default(),
            vault_mount: std::env::var("TURERP_VAULT_MOUNT")
                .unwrap_or_else(|_| "secret".to_string()),
            fallback_to_env: std::env::var("TURERP_VAULT_FALLBACK_TO_ENV")
                .ok()
                .map(|v| v == "true" || v == "1")
                .unwrap_or(true),
        }
    }
}

impl Default for SecretsConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

/// Redact password from a PostgreSQL connection URL to avoid leaking credentials.
fn redact_password_in_url(url: &str) -> String {
    // Simple parser for postgres://user:password@host/db
    if let Some(at_pos) = url.find('@') {
        if let Some(protocol_end) = url.find("://") {
            let prefix = &url[..protocol_end + 3];
            let before_at = &url[protocol_end + 3..at_pos];
            let after_at = &url[at_pos..];
            if let Some(colon_pos) = before_at.find(':') {
                // user:password -> user:****
                return format!("{}{}:****{}", prefix, &before_at[..colon_pos], after_at);
            }
        }
    }
    url.to_string()
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Redact password from database URL to avoid credential leakage in logs
        let db_display = redact_password_in_url(&self.database.url);
        write!(
            f,
            "Config(server: {}:{}, db: {})",
            self.server.host, self.server.port, db_display
        )
    }
}

impl Config {
    /// Load configuration from environment variables
    ///
    /// Required environment variables (production):
    /// - TURERP_DATABASE_URL: PostgreSQL connection string
    /// - TURERP_JWT_SECRET: Secret key for JWT tokens (min 32 chars in production)
    ///
    /// Optional environment variables:
    /// - TURERP_ENV: Environment (development/production, default: development)
    /// - TURERP_SERVER_HOST: Server host (default: 0.0.0.0)
    /// - TURERP_SERVER_PORT: Server port (default: 8000)
    /// - TURERP_DB_MAX_CONNECTIONS: Max DB connections (default: 10)
    /// - TURERP_DB_MIN_CONNECTIONS: Min DB connections (default: 5)
    /// - TURERP_JWT_ACCESS_EXPIRATION: Access token expiry in seconds (default: 3600)
    /// - TURERP_JWT_REFRESH_EXPIRATION: Refresh token expiry in seconds (default: 604800)
    /// - TURERP_CORS_ORIGINS: Comma-separated allowed origins (default: *)
    /// - TURERP_CORS_METHODS: Comma-separated allowed methods (default: GET,POST,PUT,DELETE,OPTIONS)
    /// - TURERP_CORS_HEADERS: Comma-separated allowed headers (default: Content-Type,Authorization)
    pub fn new() -> Result<Self, ConfigError> {
        let environment = std::env::var("TURERP_ENV")
            .ok()
            .map(|s| match s.to_lowercase().as_str() {
                "production" | "prod" => Environment::Production,
                _ => Environment::Development,
            })
            .unwrap_or_default();

        let server = ServerConfig {
            host: std::env::var("TURERP_SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("TURERP_SERVER_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8000),
        };

        let database = DatabaseConfig::from_env()?;
        let redis = RedisConfig::from_env();
        let jwt = JwtConfig::from_env()?;
        let cors = CorsConfig::from_env()?;
        let rate_limit = RateLimitConfig::from_env();
        let metrics = MetricsConfig::from_env();
        let localization = LocalizationConfig::from_env();
        let cdc = CdcConfig::from_env();
        let security_headers = SecurityHeadersConfig::from_env();
        let secrets = SecretsConfig::from_env();
        let encryption_key = std::env::var("TURERP_ENCRYPTION_KEY")
            .unwrap_or_else(|_| "dGVzdC1rZXktZm9yLXRlc3Rpbmctb25seS0xMjM0NTY=".to_string());

        Ok(Self {
            environment,
            server,
            database,
            redis,
            jwt,
            cors,
            rate_limit,
            metrics,
            localization,
            cdc,
            security_headers,
            secrets,
            encryption_key,
        })
    }

    /// Validate configuration for production use
    pub fn validate(&self) -> Result<(), ConfigError> {
        // In production, enforce security requirements
        if matches!(self.environment, Environment::Production) {
            // JWT secret must be strong
            if self.jwt.secret.len() < 32 {
                return Err(ConfigError::Message(
                    "JWT_SECRET must be at least 32 characters in production".to_string(),
                ));
            }

            // JWT secret should not contain common weak patterns
            let weak_patterns = ["dev", "test", "secret", "password", "change", "production"];
            for pattern in weak_patterns {
                if self.jwt.secret.to_lowercase().contains(pattern) {
                    return Err(ConfigError::Message(format!(
                        "JWT_SECRET contains weak pattern '{}' - use a secure random string",
                        pattern
                    )));
                }
            }

            // Encryption key must not be the hardcoded default
            if self.encryption_key == "dGVzdC1rZXktZm9yLXRlc3Rpbmctb25seS0xMjM0NTY=" {
                return Err(ConfigError::Message(
                    "TURERP_ENCRYPTION_KEY is set to the default test key in production. \
                     Generate a secure 32-byte base64-encoded key and set TURERP_ENCRYPTION_KEY."
                        .to_string(),
                ));
            }

            // CORS should not be wildcard in production
            if self.cors.is_wildcard() {
                return Err(ConfigError::Message(
                    "CORS is configured to allow all origins (*) in production mode. \
                     Set TURERP_CORS_ORIGINS to specific domains."
                        .to_string(),
                ));
            }

            // Warn if rate limiting trusts X-Forwarded-For without trusted proxies
            if !self.rate_limit.has_trusted_proxies() {
                tracing::warn!(
                    "No trusted proxies configured (TURERP_TRUSTED_PROXIES). \
                     Rate limiting will use direct peer IP and ignore X-Forwarded-For headers. \
                     If behind a load balancer, set TURERP_TRUSTED_PROXIES to trust forwarded headers."
                );
            }
        }

        Ok(())
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

    /// Check if running in production mode
    pub fn is_production(&self) -> bool {
        matches!(self.environment, Environment::Production)
    }

    /// Check if running in development mode
    pub fn is_development(&self) -> bool {
        matches!(self.environment, Environment::Development)
    }

    /// Resolve secrets from Vault (or fallback to env) and update config fields
    pub async fn resolve_secrets(
        &mut self,
        service: &dyn crate::common::secrets::SecretsService,
    ) -> Result<(), ConfigError> {
        // Database URL
        if let Some(url) = service
            .get_secret("turerp/database", "url")
            .await
            .map_err(|e| {
                ConfigError::Message(format!("Failed to resolve database secret: {}", e))
            })?
        {
            self.database.url = url;
        }

        // JWT secret
        if let Some(secret) = service
            .get_secret("turerp/jwt", "secret")
            .await
            .map_err(|e| ConfigError::Message(format!("Failed to resolve JWT secret: {}", e)))?
        {
            self.jwt.secret = secret;
        }

        // Redis URL (optional)
        if let Some(url) = service
            .get_secret("turerp/redis", "url")
            .await
            .ok()
            .flatten()
        {
            self.redis.url = url;
        }

        Ok(())
    }

    /// Decode the base64-encoded encryption key into a 32-byte array.
    /// Panics if the key is not valid base64 or not exactly 32 bytes.
    pub fn encryption_key_bytes(&self) -> [u8; 32] {
        let decoded = BASE64
            .decode(&self.encryption_key)
            .expect("encryption_key must be valid base64");
        decoded
            .try_into()
            .expect("encryption_key must decode to exactly 32 bytes")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.server.port, 8000);
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.jwt.access_token_expiration, 3600);
        assert!(config.cors.is_wildcard());
    }

    #[test]
    fn test_tenant_database_url() {
        let config = Config {
            database: DatabaseConfig {
                url: "postgres://postgres:postgres@localhost:5432/turerp".to_string(),
                max_connections: 10,
                min_connections: 5,
            },
            ..Default::default()
        };
        let tenant_url = config.tenant_database_url("tenant_abc");
        assert!(tenant_url.contains("tenant_abc"));
        assert!(tenant_url.contains("turerp") || tenant_url.contains("postgres"));
    }

    #[test]
    fn test_environment_default() {
        let config = Config::default();
        assert!(config.is_development());
        assert!(!config.is_production());
    }

    #[test]
    fn test_environment_display() {
        assert_eq!(format!("{}", Environment::Development), "development");
        assert_eq!(format!("{}", Environment::Production), "production");
    }

    #[test]
    fn test_cors_wildcard() {
        let cors = CorsConfig::default();
        assert!(cors.is_wildcard());
    }

    #[test]
    fn test_cors_specific_origins() {
        let cors = CorsConfig {
            allowed_origins: vec!["https://example.com".to_string()],
            ..Default::default()
        };
        assert!(!cors.is_wildcard());
    }

    #[test]
    fn test_cors_multiple_origins() {
        let cors = CorsConfig {
            allowed_origins: vec![
                "https://example.com".to_string(),
                "https://api.example.com".to_string(),
            ],
            ..Default::default()
        };
        assert!(!cors.is_wildcard());
        assert_eq!(cors.allowed_origins.len(), 2);
    }

    #[test]
    fn test_config_display() {
        let config = Config::default();
        let display = format!("{}", config);
        assert!(display.contains("0.0.0.0:8000"));
    }

    #[test]
    fn test_validate_development_mode() {
        let config = Config::default();
        // Development mode should always pass validation
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_production_weak_jwt_secret() {
        let config = Config {
            environment: Environment::Production,
            jwt: JwtConfig {
                secret: "dev-secret-do-not-use-in-production".to_string(),
                access_token_expiration: 3600,
                refresh_token_expiration: 604800,
            },
            cors: CorsConfig {
                allowed_origins: vec!["https://example.com".to_string()],
                ..Default::default()
            },
            ..Default::default()
        };

        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("weak pattern"));
    }

    #[test]
    fn test_validate_production_short_jwt_secret() {
        let config = Config {
            environment: Environment::Production,
            jwt: JwtConfig {
                secret: "short".to_string(),
                access_token_expiration: 3600,
                refresh_token_expiration: 604800,
            },
            cors: CorsConfig {
                allowed_origins: vec!["https://example.com".to_string()],
                ..Default::default()
            },
            ..Default::default()
        };

        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("32 characters"));
    }

    #[test]
    fn test_validate_production_strong_jwt_secret() {
        let config = Config {
            environment: Environment::Production,
            jwt: JwtConfig {
                secret: "aGg3N2RmZ2hqOEBrc2RqZmhosdKJF8sdfkjhsdkjfh".to_string(), // Strong random-looking secret
                access_token_expiration: 3600,
                refresh_token_expiration: 604800,
            },
            cors: CorsConfig {
                allowed_origins: vec!["https://example.com".to_string()],
                ..Default::default()
            },
            encryption_key: "YWJiY2NkZGVmZmdnaGhpaWpra2xsbW1ubm9vcHFyc3R1dnd4eXoxMjM0NTY="
                .to_string(),
            ..Default::default()
        };

        let result = config.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_production_wildcard_cors() {
        let config = Config {
            environment: Environment::Production,
            jwt: JwtConfig {
                secret: "aGg3N2RmZ2hqOEBrc2RqZmhosdKJF8sdfkjhsdkjfh".to_string(),
                access_token_expiration: 3600,
                refresh_token_expiration: 604800,
            },
            cors: CorsConfig {
                allowed_origins: vec!["*".to_string()],
                ..Default::default()
            },
            encryption_key: "YWJiY2NkZGVmZmdnaGhpaWpra2xsbW1ubm9vcHFyc3R1dnd4eXoxMjM0NTY="
                .to_string(),
            ..Default::default()
        };

        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("CORS"));
        assert!(err.contains("*"));
    }

    #[test]
    fn test_server_config_default() {
        let server = ServerConfig::default();
        assert_eq!(server.host, "0.0.0.0");
        assert_eq!(server.port, 8000);
    }

    #[test]
    fn test_jwt_config_dev() {
        let jwt = JwtConfig::dev();
        assert!(jwt.secret.contains("dev"));
        assert_eq!(jwt.access_token_expiration, 3600);
        assert_eq!(jwt.refresh_token_expiration, 604800);
    }

    #[test]
    fn test_config_master_database_url() {
        let config = Config {
            database: DatabaseConfig {
                url: "postgres://user:pass@host/db".to_string(),
                max_connections: 10,
                min_connections: 5,
            },
            ..Default::default()
        };
        let url = config.master_database_url();
        assert!(!url.is_empty());
    }

    #[test]
    fn test_tenant_database_url_edge_cases() {
        let config = Config {
            database: DatabaseConfig {
                url: "postgres://user:pass@host/db".to_string(),
                max_connections: 10,
                min_connections: 5,
            },
            ..Default::default()
        };

        let tenant_url = config.tenant_database_url("newdb");
        assert_eq!(tenant_url, "postgres://user:pass@host/newdb");
    }

    #[test]
    fn test_cors_default_methods() {
        let cors = CorsConfig::default();
        assert!(cors.allowed_methods.contains(&"GET".to_string()));
        assert!(cors.allowed_methods.contains(&"POST".to_string()));
        assert!(cors.allowed_methods.contains(&"PUT".to_string()));
        assert!(cors.allowed_methods.contains(&"DELETE".to_string()));
        assert!(cors.allowed_methods.contains(&"OPTIONS".to_string()));
    }

    #[test]
    fn test_cors_default_headers() {
        let cors = CorsConfig::default();
        assert!(cors.allowed_headers.contains(&"Content-Type".to_string()));
        assert!(cors.allowed_headers.contains(&"Authorization".to_string()));
    }

    #[test]
    fn test_cors_credentials_default() {
        let cors = CorsConfig::default();
        assert!(cors.allow_credentials);
    }

    #[test]
    fn test_environment_equality() {
        assert_eq!(Environment::Development, Environment::Development);
        assert_eq!(Environment::Production, Environment::Production);
        assert_ne!(Environment::Development, Environment::Production);
    }
}
