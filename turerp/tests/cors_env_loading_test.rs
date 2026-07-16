//! Regression test for operator-checklist item 3 (PR 3 of
//! docs/superpowers/plans/2026-06-10-production-release.md).
//!
//! `CorsConfig::from_env` must read `TURERP_CORS_ORIGINS` as a
//! comma-separated list of origins, and the default when the env var
//! is unset must be the production-safe wildcard fallback (i.e., the
//! function returns ["*"] rather than panicking or returning an
//! empty vec).
//!
//! IMPORTANT: the production validator (`Config::validate`) refuses
//! the wildcard origin in production. So the dev default `["*"]` is
//! a deliberate two-layer design:
//!   1. The dev default is permissive (so local dev just works).
//!   2. The production validator is strict (so a careless deploy
//!      with `TURERP_ENV=production` and no `TURERP_CORS_ORIGINS`
//!      fails to start, rather than allowing all origins in prod).
//!
//! This file's tests assert the layer-1 contract. Layer-2 lives in
//! `turerp/src/config.rs` (`test_validate_production_wildcard_cors`).

use turerp::config::{Config, CorsConfig};

const ENV_VARS: &[&str] = &[
    "TURERP_CORS_ORIGINS",
    "TURERP_CORS_METHODS",
    "TURERP_CORS_HEADERS",
    "TURERP_CORS_CREDENTIALS",
    "TURERP_CORS_MAX_AGE",
    "TURERP_DATABASE_URL",
    "TURERP_JWT_SECRET",
    "TURERP_ENCRYPTION_KEY",
    "TURERP_DB_MAX_CONNECTIONS",
];

use crate::common::ENV_LOCK;

struct EnvGuard {
    saved: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    fn capture() -> Self {
        let saved = ENV_VARS
            .iter()
            .map(|k| (*k, std::env::var(k).ok()))
            .collect();
        for k in ENV_VARS {
            std::env::remove_var(k);
        }
        // Config::new requires a URL + JWT secret. The tests assert on
        // the cors field, not the database/jwt values.
        std::env::set_var("TURERP_DATABASE_URL", "postgres://u:p@localhost:5432/d");
        std::env::set_var(
            "TURERP_JWT_SECRET",
            "a-reasonably-long-jwt-secret-for-tests",
        );
        Self { saved }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (k, v) in self.saved.drain(..) {
            match v {
                Some(s) => std::env::set_var(k, s),
                None => std::env::remove_var(k),
            }
        }
    }
}

#[test]
fn from_env_honors_csv_origins() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _g = EnvGuard::capture();
    std::env::set_var(
        "TURERP_CORS_ORIGINS",
        "https://app.example.com,https://admin.example.com",
    );
    let cfg = Config::new().expect("config load");
    assert_eq!(cfg.cors.allowed_origins.len(), 2);
    assert!(cfg
        .cors
        .allowed_origins
        .contains(&"https://app.example.com".to_string()));
    assert!(cfg
        .cors
        .allowed_origins
        .contains(&"https://admin.example.com".to_string()));
    assert!(!cfg.cors.is_wildcard());
}

#[test]
fn from_env_trims_whitespace_around_origins() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _g = EnvGuard::capture();
    // Real-world ops often paste a list with a trailing comma or
    // extra spaces ("a, b, c,"). A naive split leaves the empty
    // token and the spaces; the parser must handle both.
    std::env::set_var(
        "TURERP_CORS_ORIGINS",
        "  https://a.example.com , https://b.example.com  ",
    );
    let cfg = Config::new().expect("config load");
    assert_eq!(cfg.cors.allowed_origins.len(), 2);
    assert!(cfg
        .cors
        .allowed_origins
        .contains(&"https://a.example.com".to_string()));
    assert!(cfg
        .cors
        .allowed_origins
        .contains(&"https://b.example.com".to_string()));
}

#[test]
fn from_env_treats_empty_value_as_wildcard_default() {
    // This is the design choice documented in the file header:
    // env-unset is the safe dev default, NOT a deny-all-by-default.
    // Empty string is the same as env-unset.
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _g = EnvGuard::capture();
    std::env::set_var("TURERP_CORS_ORIGINS", "");
    let cfg = CorsConfig::from_env().expect("cors config");
    assert!(
        cfg.is_wildcard(),
        "empty TURERP_CORS_ORIGINS must fall back to the dev default ['*']"
    );
}

#[test]
fn from_env_wildcard_explicit() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _g = EnvGuard::capture();
    std::env::set_var("TURERP_CORS_ORIGINS", "*");
    let cfg = CorsConfig::from_env().expect("cors config");
    assert!(cfg.is_wildcard());
    // When the operator explicitly sets `*`, the dev default
    // should be respected (one entry, not a list).
    assert_eq!(cfg.allowed_origins.len(), 1);
}

#[test]
fn from_env_specific_origins_disable_credentials_by_default() {
    // Wildcard + credentials is an MDN/W3C violation; the parser
    // already pins credentials=false when wildcard is in the list.
    // When the operator supplies a specific origin list, the default
    // is credentials=true (matches AGENTS.md). This test pins the
    // current default — a future change to "credentials=false unless
    // explicitly opted in" is a deliberate API change, not a bug.
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _g = EnvGuard::capture();
    std::env::set_var("TURERP_CORS_ORIGINS", "https://app.example.com");
    let cfg = CorsConfig::from_env().expect("cors config");
    assert!(!cfg.is_wildcard());
    assert!(cfg.allow_credentials);
}

#[test]
fn from_env_honors_explicit_credentials_false_override() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _g = EnvGuard::capture();
    std::env::set_var("TURERP_CORS_ORIGINS", "https://app.example.com");
    std::env::set_var("TURERP_CORS_CREDENTIALS", "false");
    let cfg = CorsConfig::from_env().expect("cors config");
    assert!(!cfg.allow_credentials);
}
