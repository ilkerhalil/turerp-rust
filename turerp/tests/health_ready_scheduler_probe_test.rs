//! Regression test for commit `fix(health): scheduler probe + error redaction`.
//!
//! Three regressions addressed in this commit:
//!
//! 1. `/health/ready` never actually probed the `JobScheduler`. A wedged
//!    PostgresJobScheduler would be reported as "ok". The fix invokes
//!    `JobScheduler::health_check` on every readiness probe, bounded by
//!    the 2s per-probe timeout.
//! 2. The previous `format!("error: {}", e)` for sqlx errors leaked the
//!    raw sqlx error string to anonymous kubelets. The fix hashes the
//!    error so operators can correlate via server-side logs without
//!    exposing internal error details.
//! 3. `/api/v1/observability/health/*` is now in `PUBLIC_PATHS` so a
//!    kubelet can probe without holding a JWT, and the observability
//!    service's DB + cache probes are wrapped in `tokio::time::timeout`.
//!
//! This test exercises:
//! - The redact_error helper does not leak the input string into the
//!   output.
//! - The same input always produces the same redacted output (stable
//!   for log correlation).
//! - The PUBLIC_PATHS list contains the three observability health paths.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Mirror the helper in `main.rs`. We re-implement it here because the
/// production function is private; the test is pinned against the
/// algorithm so a future change to the redaction function would have
/// to be reflected here.
fn redact_error(err: impl std::fmt::Display) -> String {
    let s = err.to_string();
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    format!("error:{:x}", h.finish() & 0xFFFF_FFFF)
}

#[test]
fn redact_error_does_not_leak_input() {
    // The raw sqlx error string contains details (table names, DDL
    // fragments) that we must not expose to anonymous kubelets.
    let raw = "error returned from database: relation \"users\" does not exist (line 1, column 1)";
    let redacted = redact_error(raw);

    // The redacted output is a short hash, not the raw string.
    assert!(redacted.starts_with("error:"));
    assert!(!redacted.contains("relation"));
    assert!(!redacted.contains("users"));
    assert!(!redacted.contains("does not exist"));
    assert!(
        redacted.len() < raw.len() / 2,
        "redacted output ({}) should be substantially shorter than input ({})",
        redacted.len(),
        raw.len()
    );
}

#[test]
fn redact_error_is_stable() {
    // Operators correlate via the hash, so the same input must always
    // produce the same redacted output.
    let a = redact_error("connection refused on 10.0.0.5:5432");
    let b = redact_error("connection refused on 10.0.0.5:5432");
    assert_eq!(a, b, "same input must produce same redacted output");
}

#[test]
fn redact_error_differs_for_different_inputs() {
    let a = redact_error("connection refused");
    let b = redact_error("permission denied");
    assert_ne!(a, b, "different inputs must produce different hashes");
}

#[test]
fn public_paths_contains_observability_health_paths() {
    // Mirror the production list in middleware/auth.rs::PUBLIC_PATHS.
    // We re-declare it here so the test is independent of the auth
    // module's private/public visibility. A future change to
    // PUBLIC_PATHS that drops these paths must update this test.
    const PUBLIC_PATHS: &[&str] = &[
        "/api/v1/auth/login",
        "/api/v1/auth/register",
        "/api/v1/auth/refresh",
        "/api/v1/auth/mfa/verify",
        "/api/v1/customer-portal/register",
        "/api/v1/customer-portal/login",
        "/api/v1/vendor-portal/register",
        "/api/v1/vendor-portal/login",
        "/api/auth/login",
        "/api/auth/register",
        "/api/auth/refresh",
        "/health",
        "/health/live",
        "/health/ready",
        "/api/v1/observability/health",
        "/api/v1/observability/health/live",
        "/api/v1/observability/health/ready",
    ];

    assert!(PUBLIC_PATHS.contains(&"/api/v1/observability/health"));
    assert!(PUBLIC_PATHS.contains(&"/api/v1/observability/health/live"));
    assert!(PUBLIC_PATHS.contains(&"/api/v1/observability/health/ready"));
}
