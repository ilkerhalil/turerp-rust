//! Regression test for operator-checklist item 5 (PR 3 of
//! docs/superpowers/plans/2026-06-10-production-release.md).
//!
//! `DatabaseConfig::from_env` must read `TURERP_DB_MAX_CONNECTIONS` and
//! pass the value through to `PgPoolOptions::max_connections` in
//! `db::create_pool`. The default when the env var is unset is
//! `num_cpus::get() * 4` (not the plan's literal "10" — see PR body
//! for the deviation rationale: AGENTS.md documents this convention
//! and the rest of the codebase depends on it).
//!
//! Invalid values must be ignored (the env parser returns None and
//! falls back to the default) rather than panicking on startup.

use turerp::config::{Config, DatabaseConfig};

/// Every env var the test mutates. Centralized so the cleanup
/// teardown at the bottom cannot miss one. If a future test needs to
/// mutate a new var, add it here and to `EnvGuard::capture` so the
/// restoration logic stays correct.
const ENV_VARS: &[&str] = &[
    "TURERP_DB_MAX_CONNECTIONS",
    "TURERP_DB_MIN_CONNECTIONS",
    "TURERP_DB_ACQUIRE_TIMEOUT",
    "TURERP_DB_IDLE_TIMEOUT",
    "TURERP_DB_MAX_LIFETIME",
    "TURERP_DATABASE_URL",
    "TURERP_JWT_SECRET",
    "TURERP_ENCRYPTION_KEY",
    "TURERP_CORS_ORIGINS",
];

/// cargo runs tests in parallel within a single binary unless
/// serialized. Without this mutex, test A's set_var/remove_var races
/// with test B's capture-and-restore. The lock is held only for the
/// env-mutating sections (not the asserts), so it does not add
/// wall-clock latency.
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
        // DatabaseConfig::from_env requires a URL — pre-set a throwaway
        // value so the env-unset path can be exercised. The tests
        // assert on max_connections, not the URL.
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
fn from_env_honors_max_connections_override() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _g = EnvGuard::capture();
    std::env::set_var("TURERP_DB_MAX_CONNECTIONS", "25");
    let cfg = DatabaseConfig::from_env().expect("config load");
    assert_eq!(
        cfg.max_connections, 25,
        "TURERP_DB_MAX_CONNECTIONS=25 must be honored by from_env"
    );
}

#[test]
fn from_env_falls_back_to_cpu_default_when_unset() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _g = EnvGuard::capture();
    let cfg = DatabaseConfig::from_env().expect("config load");
    let expected = num_cpus::get() as u32 * 4;
    assert_eq!(
        cfg.max_connections, expected,
        "unset TURERP_DB_MAX_CONNECTIONS must fall back to num_cpus * 4, not panic or use 0"
    );
    // Sanity: a value of zero would deadlock the pool on first query.
    assert!(cfg.max_connections > 0);
}

#[test]
fn from_env_ignores_non_numeric_max_connections_and_keeps_default() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _g = EnvGuard::capture();
    std::env::set_var("TURERP_DB_MAX_CONNECTIONS", "not-a-number");
    let cfg = DatabaseConfig::from_env().expect("config load");
    // Non-numeric values must not crash startup; the parser returns
    // None and the default kicks in. This is the safe failure mode
    // for an env-controlled pool size.
    let expected = num_cpus::get() as u32 * 4;
    assert_eq!(cfg.max_connections, expected);
}

/// End-to-end: a full `Config::new()` call with a non-default
/// max_connections must preserve the value through to the typed
/// DatabaseConfig. This is the operator-facing surface — they set the
/// env var, restart the service, and expect the new value to take
/// effect. (A bug where the env var is read but not stored in the
/// struct would slip through the focused tests above.)
#[test]
fn config_new_propagates_max_connections_from_env() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _g = EnvGuard::capture();
    std::env::set_var("TURERP_DB_MAX_CONNECTIONS", "42");
    let cfg = Config::new().expect("config load");
    assert_eq!(cfg.database.max_connections, 42);
}
