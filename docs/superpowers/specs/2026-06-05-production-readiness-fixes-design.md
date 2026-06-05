# Production-Readiness Fix Plan — turerp-rust

## Context

A deep production-readiness audit completed on 2026-06-05 surfaced 14 findings across migrations, background-worker lifecycle, configuration validation, health checks, and edge-case handling. The original ask was "eksik PostgreSQL repo wiring" — that turned out to be a subset of a larger issue: Postgres repositories were wired in `lib.rs:create_app_state` (PR #126), but several of the tables they reference have **never been created in any migration**, so production Postgres deployments crash at first query against those endpoints.

**Key facts discovered:**

- 10 PostgreSQL tables (4 documents, 3 shift/attendance, 3 archive) referenced by `*Row` structs and SQL queries do not exist in `turerp/migrations/*.sql`. They were never created.
- `turerp/migrations/034_inter_company_and_revoked_tokens.sql` is on disk but missing from `turerp/src/db/pool.rs` MIGRATIONS list (PR #131 added the repos but did not register the migration).
- `create_app_state_unified` (`turerp/src/lib.rs:1714-1722`) silently falls back to in-memory storage when `database.url` is empty, even in `Environment::Production`. `Config::validate()` (line 616) does not check this. An operator who forgets to set `TURERP_DATABASE_URL` will start a server that LOOKS healthy and loses every transaction on restart.
- Background workers (`JobExecutor`, `BackgroundEvaluator`, audit writer) cannot be cleanly stopped on SIGTERM — shutdown channels are stored but never triggered.
- `/health/ready` covers only DB + cache; it does not verify job scheduler, observability evaluator, or audit pipeline. A crashed worker leaves the server returning 200 while silently losing data.
- Per-call timeouts are missing on the readiness probe, so a hung Redis can exhaust the actix worker pool.

**No previous design exists.** The audit (agent task) produced the finding list; this design groups them into a small number of PRs and orders them so the build stays green at every step.

## Goals

1. Make a fresh `cargo run --features postgres` install with a clean PostgreSQL instance work end-to-end with no `relation does not exist` errors.
2. Make a production deployment with empty `TURERP_DATABASE_URL` refuse to start instead of silently serving in-memory storage.
3. Make `Ctrl-C` (SIGTERM) drain all background workers within 5 seconds.
4. Make `/health/ready` return 503 within 2 seconds when any dependency is degraded.
5. Make every spawned background task recoverable from a panic.
6. Validate configuration in `Config::validate()` to catch silent misconfigurations before they reach production traffic.

## Non-Goals

- Schema redesign — we add missing tables, not refactor existing ones.
- Performance tuning beyond the rate-limit defaults; deeper perf work is a separate audit.
- Adding new features or endpoints; this plan is bug-fixes and missing tables only.
- Replacing any existing InMemory implementation with a Postgres one — `InMemoryJobScheduler` stays.
- Removing the `InMemory` mode entirely — it is the default for `cargo test` and local development.

## Approach

The fixes are split into **5 PRs**, each independently shippable, each closing a coherent set of findings. PRs run sequentially because some of the later changes depend on the AppState shape established in the earlier ones.

### PR-1 (P0): Missing migrations + production-time DATABASE_URL enforcement

**Why first:** Without migrations the Postgres build is broken at runtime. Without the URL check a production deploy can silently lose all data. Both are P0 and block the production release.

**Changes:**

1. `turerp/migrations/035_core_tables.sql` (NEW) — create the 10 missing tables. Schema derived from the `*Row` structs in the corresponding `postgres_repository.rs` files. The four sources of truth are:
   - `turerp/src/domain/document/postgres_repository.rs` (lines 200, 224, 252, 283, 384, 409, 416, 423, 509, 539, 583, 596, 658, 697, 716, 734, 754) → `documents`, `document_categories`, `document_links`, `document_versions`
   - `turerp/src/domain/shift/postgres_repository.rs` (lines 124, 143, 169, 229, 297, 313, 406, 423, 446, 464, 530, 547, 665, 717, 736, 763, 788, 811, 881, 898) → `shifts`, `shift_assignments`, `attendance_records`
   - `turerp/src/domain/archive/postgres_repository.rs` (lines 119, 146, 175, 229, 353, 380, 418, 503, 620, 651, 682, 712, 742, 790) → `archive_policies`, `archive_jobs`, `archive_records`

   Each table includes `tenant_id BIGINT NOT NULL`, `created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()`, `updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()`, and `is_deleted BOOLEAN NOT NULL DEFAULT false` (to match the soft-delete pattern from migration 020). Indexes on `tenant_id` plus any composite indexes implied by the WHERE clauses in the repository code.

2. `turerp/migrations/036_register_034.sql` (NEW) — empty `.sql` that re-registers migration 034 in the migration history. **No** — this is the wrong approach. Instead:

2'. `turerp/src/db/pool.rs` — add the missing `Migration` entry for `034_inter_company_and_revoked_tokens.sql` to the `MIGRATIONS` slice. Read 034 to confirm it does not contain a self-insert into the migrations table (other migrations do this); if it does, this is a one-line append.

3. `turerp/src/config.rs::validate()` — at the top of the `Production` branch, add:
   ```rust
   if self.database.url.is_empty() {
       return Err(ConfigError::Message(
           "TURERP_DATABASE_URL must be set in production".to_string(),
       ));
   }
   ```

**Verification:**
- `cd turerp && DATABASE_URL=sqlite::memory: TURERP_ENVIRONMENT=production cargo run` exits non-zero with the new error message.
- `cd turerp && TURERP_DATABASE_URL=postgres://localhost cargo run` reaches the migrations step and creates all 34 → 35 tables.
- `cargo test --lib` still passes (InMemory mode unaffected).
- New `cargo test --test migration_inventory_test` (small) that greps the migration directory and asserts every table referenced by any `*Row::table` or `FROM <name>` in `postgres_repository.rs` files has a matching `CREATE TABLE` in a migration with version ≤ max registered migration.

**Out of scope (kept for later PRs):** shutdown, panic recovery, health-check timeouts.

---

### PR-2 (P1): Background worker shutdown coordination

**Why second:** Once we can boot cleanly, the next most disruptive outage scenario is "deploy, hit SIGTERM, the server hangs for 30 seconds and the new instance cannot bind". This PR fixes that.

**Changes:**

1. `turerp/src/main.rs`:
   - Stop binding `_obs_shutdown_tx` to `_`; bind to `obs_shutdown_tx`.
   - Capture the `JoinHandle` returned by `evaluator.start(obs_shutdown_rx)` into `evaluator_handle`.
   - In `JobExecutor::start` (in `turerp/src/common/job_executor.rs`), change the API so it returns a `JoinHandle<()>` and a `ShutdownSender`. Have `main` hold both.
   - After `HttpServer::run().await` completes, in a `tokio::time::timeout(Duration::from_secs(5), join_all(handles))`:
     1. Send shutdown signals.
     2. Await all handles.
     3. Log a warning if the timeout elapses.

2. `turerp/src/common/job_executor.rs`:
   - `pub fn start(self) -> (JoinHandle<()>, ShutdownSender)` instead of `pub async fn start(self)`. Shutdown sender is `mpsc::Sender<()>`.

3. `turerp/src/common/background_evaluator.rs`:
   - `pub fn start(self, shutdown_rx: mpsc::Receiver<()>) -> JoinHandle<()>` (already returns the handle; just don't ignore it).

**Verification:**
- `cargo run`, then in another terminal `kill -TERM <pid>`. Server logs "background workers stopped" and exits within 5 seconds.
- `cargo test --lib job_executor` and `cargo test --lib background_evaluator` cover the shutdown path.

---

### PR-3 (P1): Panic recovery in background tasks + audit backpressure

**Why third:** A single bug in a worker (e.g., a null pointer, a panic in a SQL query) should not silently halt audit logging and alert evaluation for the lifetime of the process. This is a data-loss risk during extended operations.

**Changes:**

1. `turerp/src/common/job_executor.rs` — wrap the main loop body in `AssertUnwindSafe(...).catch_unwind()` and restart the loop with exponential backoff (1s, 2s, 4s, capped at 30s) after each panic. Reset backoff after 60s of clean execution.

2. `turerp/src/common/background_evaluator.rs` — same pattern.

3. `turerp/src/middleware/audit.rs::spawn_audit_writer` — same pattern, plus the writer task.

4. `turerp/src/middleware/audit.rs` — change `sender.try_send(event)` to a tiered strategy:
   - If the response status is ≥ 500 OR the request was an auth failure OR it was a write to a sensitive entity (auth, mfa, role, permission), use `sender.send().await` (blocks, preserves the event).
   - Otherwise, keep `try_send` (drop with warn on overflow).

   The classification function lives next to the event emitter in the audit middleware.

**Verification:**
- New unit tests: `test_job_executor_recovers_from_panic` injects a `bool` flag into the loop that triggers `panic!()` on the 3rd iteration; asserts the loop continues afterward.
- Same for `background_evaluator` and `audit_writer`.
- Manual: start the server, point a script at a non-existent DB, verify audit events are still flushed after the DB recovers.

---

### PR-4 (P1/P2): Health checks + config validation hardening

**Why fourth:** At this point the server can boot, run workers, and recover from panics. Now we make sure a load balancer can correctly detect when something is wrong, and that we catch silly misconfigurations at startup.

**Changes:**

1. `turerp/src/main.rs::health_ready`:
   - Wrap the DB and cache probes in `tokio::time::timeout(Duration::from_secs(2), ...)`. Treat timeout as unhealthy.
   - Add a third probe: an `AppState.infra.job_scheduler.health_check()` method (added in step 2).
   - Add a fourth probe: read `AppState.infra.last_audit_flush_at` (added in step 3); if older than 60s, return 503.

2. `turerp/src/common/jobs.rs::JobScheduler` trait — add `async fn health_check(&self) -> Result<(), ApiError>`. Postgres impl runs `SELECT 1 FROM jobs LIMIT 1`; InMemory impl returns `Ok(())`.

3. `turerp/src/middleware/audit.rs` — add `last_audit_flush_at: Arc<AtomicU64>` (seconds since epoch) to a shared `AuditState` struct held in `AppState.infra`. Update on every successful flush. Expose via `pub fn last_flush_unix()`.

4. `turerp/src/config.rs::validate()`:
   - If `environment == Production`:
     - `rate_limit.requests_per_minute >= 60` (warn at <60, error at <10).
     - `rate_limit.burst_size >= 10` (warn at <10, error at <3).
     - `jwt.access_token_expiration > 0 && access_token_expiration <= 86400` (24h upper bound).
     - `jwt.refresh_token_expiration > access_token_expiration && refresh_token_expiration <= 2592000` (30d upper bound).
     - `encryption_key` base64-decodes to exactly 32 bytes (move from `encryption_key_bytes` lazy check to startup-time check).

**Verification:**
- New tests in `turerp/tests/config_validation_test.rs` cover each validation rule.
- Manual: `curl /health/ready` returns 200 in a healthy environment and 503 when the DB pool is exhausted (simulate by setting `max_connections=1` and holding it).
- `cargo test --lib middleware::audit` covers `last_flush_unix` updates.

---

### PR-5 (P2/P3): Rate-limit defaults + Duration edge cases

**Why last:** Polish and edge cases. Nothing here is required for production, but cleaning them up closes the audit and prevents future operators from shooting themselves.

**Changes:**

1. `turerp/src/config.rs::RateLimitConfig::default()` — change `requests_per_minute: 10, burst_size: 3` to `requests_per_minute: 120, burst_size: 30`. These are still conservative but no longer unusable in production.

2. `turerp/src/common/jobs.rs::cleanup` — replace `unwrap_or(chrono::Duration::MAX)` with `.expect("older_than must be non-negative")` (it's an internal API; the only caller is the admin job cleanup endpoint, and we want it to fail loud at the call site if a negative value is ever passed).

3. `turerp/src/config.rs::DatabaseConfig::from_env` — change `url` from required-and-panicking to required-and-errored with a clear message. (Currently it returns `Err` which is fine; the main loop's `unwrap_or_default` masks the error — see PR-1. PR-1 fixes the masking; this PR makes the error message clearer.)

**Verification:**
- `cargo test --lib rate_limit` shows new defaults.
- `cargo test --lib jobs` covers the negative-duration case.
- `cargo test` (full suite) still green.

---

## Architecture

The 5 PRs do not change the runtime architecture. The AppState shape gains two new fields in PR-4 (`last_audit_flush_at`, `job_scheduler` already exists). No new public API, no new trait methods other than `JobScheduler::health_check`.

```
                ┌──────────────────────────────────────────────┐
                │  main.rs (PR-1: prod URL check)              │
                │  main.rs (PR-2: shutdown coordination)       │
                │  main.rs (PR-4: health_check timeouts)       │
                └──────────────┬───────────────────────────────┘
                               │
                ┌──────────────▼───────────────────────────────┐
                │  create_app_state_unified                    │
                │  ├─ InMemory mode (DATABASE_URL empty)       │
                │  └─ Postgres mode  (PR-1: now requires URL)  │
                └──────────────┬───────────────────────────────┘
                               │
        ┌──────────────────────┼──────────────────────────┐
        ▼                      ▼                          ▼
   InMemory repos         Postgres repos           Migrations
   (unchanged)            (PR-1: 035 adds 10       (PR-1: 034
                           missing tables)           registered,
                                                     035 added)
        │                      │                          │
        └──────────────┬───────┴──────────────────────────┘
                       ▼
              Background workers
              ├─ JobExecutor    (PR-2: shutdown, PR-3: panic recovery)
              ├─ BackgroundEvaluator (PR-2: shutdown, PR-3: panic recovery)
              └─ Audit writer   (PR-3: panic recovery, PR-4: last_flush)
```

## Data Flow

**Migrations (PR-1):** On startup, `db::pool::run_migrations` iterates the MIGRATIONS slice in version order, applies each `*.sql` inside a transaction, records the version in `_migrations`, and skips already-applied ones. After PR-1, the slice includes 034 (inter_company + revoked_tokens) and 035 (documents/shifts/archive). The slice is sorted by version string.

**Production validation (PR-1, PR-4):** `Config::validate()` runs after `Config::new()`. If it returns `Err`, `main` exits with code 1 before opening any sockets. After PR-1, the validation also covers `database.url`. After PR-4, it covers rate limits and JWT expirations.

**Shutdown (PR-2):** `HttpServer::run().await` resolves when the server stops accepting connections. The `await` is followed by `tokio::time::timeout(5s, join_all(handles))` after sending shutdown signals. If the timeout elapses, the process exits anyway with a `tracing::error!` so a wrapper like systemd can capture the failed shutdown in journal.

**Panic recovery (PR-3):** `AssertUnwindSafe(future).catch_unwind()` returns `Result<R, Box<dyn Any>>`. The wrapper around each background loop logs the panic, increments a backoff counter, sleeps for `min(2^counter, 30)` seconds, and re-runs. A successful 60s of execution resets the counter. This is similar to supervisor trees in Erlang/OTP.

**Health check (PR-4):** `/health/ready` returns:
- `200 {"status":"ok","deps":{"db":"ok","cache":"ok","scheduler":"ok","audit":"ok"}}` when all four probes succeed within 2s.
- `503 {"status":"degraded","deps":{...}}` when any probe fails or times out. The body lists which dependency is unhealthy so an operator does not need to read logs.

## Error Handling

- All panics in background tasks are caught (PR-3). The catch handler logs `tracing::error!` with the panic payload and stack backtrace (via `std::backtrace::Backtrace::capture`).
- Health-check probes return `Result<(), ApiError>` and the handler formats the error message into the JSON response (no sensitive details leak).
- Config validation errors are returned through `ConfigError::Message`; the message is short, identifies the bad env var name, and gives the recommended value. No internal state is leaked.
- Migration failures (PR-1): the existing `run_migrations` already returns `Err`; `main` exits with code 1 and a clear log line. No partial state is left because each migration runs in a transaction.

## Testing

Per PR, the test additions are:

| PR | New tests | Modified tests |
|----|-----------|----------------|
| 1  | `migration_inventory_test` (table-vs-repo cross-check) | `test_validate_*` (4 new rules) |
| 2  | `job_executor_shutdown_test`, `background_evaluator_shutdown_test` | none |
| 3  | `panic_recovery_test` (3 variants, one per worker) | `audit_overflow_test` (sensitive events preserved) |
| 4  | `health_check_timeout_test`, `audit_last_flush_test`, `config_validation_test` (5+ new rules) | none |
| 5  | `rate_limit_defaults_test`, `cleanup_negative_duration_test` | none |

All tests must pass with `cargo test` and `cargo clippy -- -D warnings`. No new clippy warnings introduced.

## Risk & Rollback

- **PR-1 is the highest-risk PR.** Adding a new migration to a database that already has data must not break. The new `035_core_tables.sql` uses `CREATE TABLE IF NOT EXISTS` so re-running is safe. If the migration is applied on a fresh DB and the schemas are wrong, the rollback is `DROP TABLE <name>` for the 10 new tables — straightforward because no production data exists in them yet (the repositories are wired but the API endpoints they back have not been exercised in production).
- **PR-2 changing `JobExecutor::start`'s signature** affects all callers. `main.rs` is the only caller. Easy to find and update.
- **PR-3 panic recovery** adds a small CPU cost on each loop iteration (the `catch_unwind` boundary). Negligible at our tick rates (1s for jobs, 30s for evaluator).
- **PR-4 health-check timeout of 2s** could cause flapping if any dependency has normal latency > 2s. The 2s budget is comfortable for a `SELECT 1` against a healthy PG and a `PING` against a local Redis. We will measure on staging before promoting to prod.
- **PR-5 default rate-limit bump from 10 to 120 rpm** changes a visible default. Document the change in the changelog. Operators who depend on the old value can still set it via env.

## Open Questions

- (PR-1) Does `034_inter_company_and_revoked_tokens.sql` register itself in `_migrations`? The agent that ran the audit did not have visibility into that. If it does not, the one-line `pool.rs` addition is sufficient. If it does, the registration will conflict. I will read 034's contents during plan execution and adjust.
- (PR-4) `last_audit_flush_at` granularity: 1-second precision is enough for "is the audit pipeline alive". A `AtomicU64` of seconds-since-epoch is the simplest correct choice.

## Files to Modify (per PR)

**PR-1 (P0):**
- `turerp/migrations/035_core_tables.sql` (NEW)
- `turerp/src/db/pool.rs` (add 034 and 035 to MIGRATIONS)
- `turerp/src/config.rs::validate` (production DATABASE_URL check)
- `turerp/tests/migration_inventory_test.rs` (NEW)

**PR-2 (P1):**
- `turerp/src/main.rs` (shutdown signal + handle collection)
- `turerp/src/common/job_executor.rs` (return JoinHandle + ShutdownSender)
- `turerp/src/common/background_evaluator.rs` (return JoinHandle)
- `turerp/tests/job_executor_shutdown_test.rs` (NEW)
- `turerp/tests/background_evaluator_shutdown_test.rs` (NEW)

**PR-3 (P1):**
- `turerp/src/common/job_executor.rs` (panic recovery)
- `turerp/src/common/background_evaluator.rs` (panic recovery)
- `turerp/src/middleware/audit.rs` (panic recovery + tiered backpressure)
- `turerp/tests/panic_recovery_test.rs` (NEW)
- `turerp/tests/audit_overflow_test.rs` (NEW)

**PR-4 (P1/P2):**
- `turerp/src/main.rs::health_ready` (timeouts + scheduler + audit probes)
- `turerp/src/common/jobs.rs::JobScheduler` (add `health_check` method)
- `turerp/src/db/job_repository.rs` (Postgres impl of `health_check`)
- `turerp/src/middleware/audit.rs` (expose `last_flush_unix`)
- `turerp/src/config.rs::validate` (rate-limit + JWT + encryption-key checks)
- `turerp/src/lib.rs::AppState` (add `last_audit_flush_at` field)
- `turerp/tests/health_check_timeout_test.rs` (NEW)
- `turerp/tests/config_validation_test.rs` (NEW)

**PR-5 (P2/P3):**
- `turerp/src/config.rs::RateLimitConfig::default` (10→120, 3→30)
- `turerp/src/common/jobs.rs::cleanup` (expect instead of unwrap_or(MAX))
- `turerp/src/config.rs::DatabaseConfig::from_env` (clearer error message)
- `turerp/tests/rate_limit_defaults_test.rs` (NEW)
- `turerp/tests/cleanup_negative_duration_test.rs` (NEW)

## Out of Scope (future work)

- Replacing InMemory with Postgres for any domain not already migrated.
- Adding new audit-event types or extending the audit pipeline.
- Refactoring the health-check handler into a `HealthService` struct (one PR, but it is structural, not a bug-fix; deferred).
- Migrating from `config` crate to a typed env-only loader; the current `Config::new` is fine.
- Adding OpenTelemetry traces for the new health probes; defer until OTel refactor.
