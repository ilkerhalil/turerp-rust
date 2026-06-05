//! Production-readiness tests for `Config::validate`.
//!
//! These tests document the contract enforced by `validate()` when
//! `environment == Environment::Production`. Each test produces a
//! `Config` that fails exactly one rule and asserts that `validate()`
//! returns `Err` mentioning the relevant keyword.

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use turerp::config::{Config, CorsConfig, DatabaseConfig, Environment, JwtConfig, RateLimitConfig};

fn prod_base() -> Config {
    Config {
        environment: Environment::Production,
        database: DatabaseConfig {
            url: "postgres://x:y@localhost/z".into(),
            max_connections: 10,
            min_connections: 2,
            acquire_timeout_secs: 30,
            idle_timeout_secs: 600,
            max_lifetime_secs: 1800,
        },
        jwt: JwtConfig {
            secret: "aGg3N2RmZ2hqOEBrc2RqZmhosdKJF8sdfkjhsdkjfh".into(),
            access_token_expiration: 3600,
            refresh_token_expiration: 604800,
        },
        cors: CorsConfig {
            allowed_origins: vec!["https://app.example.com".into()],
            ..Default::default()
        },
        encryption_key: BASE64.encode([0u8; 32]),
        rate_limit: RateLimitConfig {
            requests_per_minute: 120,
            burst_size: 30,
            trusted_proxies: vec![],
        },
        ..Default::default()
    }
}

#[test]
fn prod_empty_db_url_fails() {
    let mut c = prod_base();
    c.database.url = String::new();
    let err = c
        .validate()
        .expect_err("empty DB URL must fail in production");
    assert!(err.to_string().contains("TURERP_DATABASE_URL"));
}

#[test]
fn prod_low_rate_limit_fails() {
    let mut c = prod_base();
    c.rate_limit.requests_per_minute = 10;
    let err = c.validate().expect_err("rate limit 10 must fail");
    assert!(err.to_string().contains("rate_limit"));
}

#[test]
fn prod_low_burst_fails() {
    let mut c = prod_base();
    c.rate_limit.burst_size = 3;
    let err = c.validate().expect_err("burst 3 must fail");
    assert!(err.to_string().contains("burst_size"));
}

#[test]
fn prod_jwt_access_zero_fails() {
    let mut c = prod_base();
    c.jwt.access_token_expiration = 0;
    let err = c.validate().expect_err("access=0 must fail");
    assert!(err.to_string().contains("access_token_expiration"));
}

#[test]
fn prod_jwt_access_too_long_fails() {
    let mut c = prod_base();
    c.jwt.access_token_expiration = 86401;
    let err = c.validate().expect_err("access=86401 must fail");
    assert!(err.to_string().contains("86400"));
}

#[test]
fn prod_jwt_refresh_shorter_than_access_fails() {
    let mut c = prod_base();
    c.jwt.refresh_token_expiration = c.jwt.access_token_expiration;
    let err = c.validate().expect_err("refresh==access must fail");
    assert!(err.to_string().contains("refresh_token_expiration"));
}

#[test]
fn prod_jwt_refresh_too_long_fails() {
    let mut c = prod_base();
    c.jwt.refresh_token_expiration = 2_592_001;
    let err = c.validate().expect_err("refresh=30d+1s must fail");
    assert!(err.to_string().contains("2592000"));
}

#[test]
fn prod_empty_encryption_key_fails() {
    let mut c = prod_base();
    c.encryption_key = String::new();
    let err = c.validate().expect_err("empty encryption key must fail");
    assert!(err.to_string().to_lowercase().contains("encryption"));
}

#[test]
fn prod_invalid_encryption_key_fails() {
    let mut c = prod_base();
    c.encryption_key = "not-base64-!!".into();
    let err = c.validate().expect_err("invalid base64 must fail");
    assert!(err.to_string().contains("TURERP_ENCRYPTION_KEY"));
}

#[test]
fn prod_encryption_key_wrong_length_fails() {
    let mut c = prod_base();
    // 16 bytes (128 bits) instead of 32 (256 bits)
    c.encryption_key = BASE64.encode([0u8; 16]);
    let err = c.validate().expect_err("16-byte key must fail");
    assert!(err.to_string().contains("32 bytes"));
}

#[test]
fn prod_valid_config_passes() {
    assert!(prod_base().validate().is_ok());
}

#[test]
fn dev_empty_db_url_passes() {
    let mut c = Config::default();
    c.database.url = String::new();
    assert!(c.validate().is_ok());
}

#[test]
fn dev_default_rate_limit_passes() {
    // Default Config has 10 rpm / 3 burst; this is fine in dev.
    assert!(Config::default().validate().is_ok());
}
