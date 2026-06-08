//! Regression test for commit `fix(config): RateLimitConfig::from_env defaults`.
//!
//! `RateLimitConfig::from_env` must return the safe defaults (120 rpm / 30
//! burst) when the env vars are unset, not the throttling 10/3 from the
//! pre-fix code path. It must also honor valid overrides.

use turerp::config::RateLimitConfig;

/// All three tests in this binary mutate the same env vars. The
/// `EnvGuard` only restores vars at the end of a single test, but
/// cargo runs the three tests in parallel within a single binary
/// unless serialized. Without this mutex, test A's set_var/remove_var
/// races with test B's capture-and-restore, leading to flaky
/// failures. The lock is held only for the duration of the
/// env-mutating sections (not the asserts), so it does not add
/// wall-clock latency.
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Names of every env var the test mutates. Centralized so the cleanup
/// teardown at the bottom cannot miss one.
const ENV_VARS: &[&str] = &[
    "TURERP_RATE_LIMIT_REQUESTS_PER_MINUTE",
    "TURERP_RATE_LIMIT_BURST",
    "TURERP_TRUSTED_PROXIES",
];

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
fn from_env_returns_safe_defaults_when_unset() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _g = EnvGuard::capture();
    let cfg = RateLimitConfig::from_env();
    assert_eq!(
        cfg.requests_per_minute, 120,
        "from_env must return the safe default of 120 rpm when env var is unset"
    );
    assert_eq!(
        cfg.burst_size, 30,
        "from_env must return the safe default of 30 burst when env var is unset"
    );
    assert!(
        cfg.trusted_proxies.is_empty(),
        "trusted_proxies must default to empty when env var is unset"
    );
}

#[test]
fn from_env_honors_overrides() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _g = EnvGuard::capture();
    std::env::set_var("TURERP_RATE_LIMIT_REQUESTS_PER_MINUTE", "240");
    std::env::set_var("TURERP_RATE_LIMIT_BURST", "60");
    std::env::set_var("TURERP_TRUSTED_PROXIES", "10.0.0.1, 10.0.0.2");
    let cfg = RateLimitConfig::from_env();
    assert_eq!(cfg.requests_per_minute, 240);
    assert_eq!(cfg.burst_size, 60);
    assert_eq!(cfg.trusted_proxies, vec!["10.0.0.1", "10.0.0.2"]);
}

#[test]
fn from_env_ignores_invalid_overrides_and_keeps_safe_defaults() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _g = EnvGuard::capture();
    std::env::set_var("TURERP_RATE_LIMIT_REQUESTS_PER_MINUTE", "not-a-number");
    std::env::set_var("TURERP_RATE_LIMIT_BURST", "");
    let cfg = RateLimitConfig::from_env();
    assert_eq!(cfg.requests_per_minute, 120);
    assert_eq!(cfg.burst_size, 30);
}
