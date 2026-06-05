//! Default-value contract tests for `RateLimitConfig`.
//!
//! The pre-hardening defaults were 10 rpm / 3 burst, which would
//! have throttled every active user in a production environment.
//! These tests pin the safer defaults (≥ 60 rpm / ≥ 10 burst) so
//! any future refactor that accidentally lowers them fails the test
//! suite.

use turerp::config::RateLimitConfig;

#[test]
fn rate_limit_defaults_are_safer_than_pre_hardening() {
    let cfg = RateLimitConfig::default();
    // Pre-hardening: 10/3. Post-hardening: 120/30.
    assert!(cfg.requests_per_minute >= 60, "rpm default too low");
    assert!(cfg.burst_size >= 10, "burst default too low");
}

#[test]
fn rate_limit_default_trusted_proxies_is_empty() {
    // Trusted proxies is operator config, never a default.
    let cfg = RateLimitConfig::default();
    assert!(cfg.trusted_proxies.is_empty());
}
