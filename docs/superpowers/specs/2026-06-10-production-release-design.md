# Production Release — Minimal GA Cut

**Date:** 2026-06-10
**Status:** Approved (brainstorming complete)
**Author:** Brainstorming session
**Target:** General Availability on a single-host docker-compose deployment

## 1. Context

`main` is clean and the live hurl suite is 16/16 green against the
running container (PR #151). The re-audit of PR #144 closed 9 P0/P1
blockers (migration drift, tenant isolation, rate-limit defaults,
audit-writer panic, health probe, JobService cron). PR #147 closed
the auth-bypass hotfix and added a regression suite. Together these
move the project from "fixing the foundation" to "ready to ship".

What remains is the **last mile**:

- 6 known-broken endpoints fenced in issue #152 (categories, units,
  currencies, leave-types, stock/warehouses, settings) — they return
  500 or 404 against real data and are not part of a happy-path demo
- Tier 2 modules (manufacturing, projects, shifts, payroll) —
  registered in the codebase but not exercised end-to-end
- AGENTS.md's 12-item operator checklist — most are operational
  (HTTPS, backup, Vault, S3), but a few are code-level (JWT-secret
  enforcement, CORS default, connection-pool limits)
- No runbook, no rollback rehearsal, no production pilot
- 724 OpenAPI paths, but only 16 are smoke-tested live today

This design proposes a **2-week minimal GA cut** that closes the
last-mile items, on a single-host docker-compose deployment, gated
by a 7-day internal pilot.

## 2. Goals

1. Ship a multi-tenant GA on a single-host docker-compose
   deployment (one host, multiple customer tenants, all sharing
   one PostgreSQL and one Redis)
2. 7-day internal pilot gates the cut: no Sev-1 incidents, p99
   response time ≤ 500 ms, zero unhandled panics in the audit
   stream
3. Tier 2 modules are deployed but **gated by a runtime feature
   flag** — operators turn them on per-tenant, not the customer
4. 6 known-broken endpoints from issue #152 are either fixed or
   also gated by the same flag
5. Operator checklist is fully satisfied for the single-host
   scenario; the cloud-native items (Vault, multi-region, K8s) are
   explicitly out of scope

## 3. Non-Goals

- Kubernetes / multi-host scaling
- Multi-region failover
- A/B testing framework
- Billing / subscription management
- Marketing site, onboarding flow, self-serve trial
- Mobile clients
- Blockchain ledger (e-Defter hash-chain)
- GraphQL GA (already in tier 2; gated)

## 4. Architecture

### 4.1 Feature-flag infrastructure (in-house)

A new `tenants.feature_flags JSONB DEFAULT '{}' NOT NULL` column
plus a thin `FeatureFlag` middleware that reads the column once
per request and rejects gated routes with `404 Not Found` (not
403, so the route appears not to exist). Two reasons to hide
gated routes as 404 rather than 403:

- A 403 leaks the route's existence to a customer who shouldn't
  know about it
- It matches the behavior of a route that has not been
  registered at all, so the smoke suite can flip a flag and
  re-test without code changes

Flag identifiers use reverse-DNS namespace:

| Flag | Module / route | Default |
|------|----------------|---------|
| `tier2.manufacturing` | manufacturing routes | off |
| `tier2.projects` | projects routes | off |
| `tier2.shifts` | shifts routes | off |
| `tier2.payroll` | payroll routes | off |
| `tier2.graphql` | `/api/v1/graphql` | off |
| `tier2.file_upload` | S3 upload routes | off |
| `core.categories` | `/api/v1/categories` | off until #152-1 fixed |
| `core.units` | `/api/v1/units` | off until #152-2 fixed |
| `core.currencies` | `/api/v1/currencies` | off until #152-3 fixed |
| `core.hr.leave_types` | `/api/v1/hr/leave-types` | off until #152-4 fixed |
| `core.stock.warehouses` | `/api/v1/stock/warehouses` | off until #152-5 fixed |
| `core.settings` | `/api/v1/settings` | off until #152-6 fixed |

Operators toggle flags via a new admin route
`PATCH /api/v1/admin/tenants/{id}/flags` (admin scope only, audited
in `audit_logs`). The flag column is read from `tenants` on every
request — no separate cache layer in v1. A `SELECT feature_flags
FROM tenants WHERE id = $1` is already on the auth path (JWT
contains the tenant), so the flag read piggybacks on that query
and adds no extra round-trip in the hot path.

Flag reads happen in the auth middleware (which already fetches
the tenant row for the JWT), so a gated route costs zero extra
DB round-trips. The route handler never runs.

### 4.2 Operator checklist (single-host scope)

The 12 AGENTS.md items reduce to 9 in this scope:

| # | Item | Owner | Status |
|---|------|-------|--------|
| 1 | Change default JWT secret (env-var enforce) | code | PR #144 (verified) |
| 2 | Enable HTTPS (Caddy reverse proxy) | ops | new |
| 3 | Configure CORS origins (not `*`) | code | new |
| 4 | DB backups (pg_dump cron, daily) | ops | new |
| 5 | Connection-pool limits (PgPool max_conns) | code | new |
| 6 | Rate limiting per endpoint | code | PR #144 (defaults fixed) |
| 7 | Logging aggregation (OTLP → Aspire) | code | already in place |
| 8 | Health checks in load balancer | code | already in place |
| 9 | Monitoring dashboards (Aspire) | code | already in place |

Out of scope for v1: Vault, Redis-as-cache (already used in
compose), S3 (file-upload gated by flag). These are documented
in AGENTS.md as deferred.

### 4.3 Rollback

- **Container image:** keep the last 3 `turerp:<sha>` tags in the
  registry; rollback = `docker compose up -d turerp=turerp:<prev>`
- **DB migration:** every migration file is wrapped in
  `BEGIN; ... COMMIT;` and has a corresponding `down.sql` in
  `migrations/down/`. New migrations add a flag to the migration
  runner that calls `down.sql` when `MIGRATIONS_DOWN=1` env is set
- **Feature flag:** any flag can be toggled off via the admin
  route; the change is instant (no rebuild, no redeploy)
- **Audit DLQ:** if the audit writer is unhealthy, logs are
  spooled to `pg_audit_dlq` and replayed on restart (PR #144
  added the panic recovery; DLQ-on-restart is new in this cut)

### 4.4 Runbook

A `RUNBOOK.md` lives at the repo root and is the first thing an
on-call reads. Initial sections:

- **Service is down** — health check failure → docker compose
  status, container logs, dependency probe
- **429 storm** — rate limit hit, check `governor` stats at
  `/api/v1/observability/rate-limit-stats`, raise quota for a
  tenant
- **Login lockout** — clear `login_attempts` for a username
- **OOM / container restart loop** — check `docker inspect` for
  exit code, check OOM kill, scale up memory limit
- **DB connection pool exhausted** — `pg_stat_activity`, kill
  idle-in-transaction sessions
- **Audit DLQ growing** — replay procedure, manual flush
- **Job not running** — JobService cron health, manual trigger
  via admin route
- **Backup failure** — restore from last good dump, run
  migration verification

## 5. Sprint plan (2 weeks)

### 5.1 Week 1 — Foundation

The feature-flag infra (PR 1) is a hard dependency for PR 2 — the
broken-endpoint fixes ship the flag-off default on the gated route
so a customer never sees a 500 even if the underlying bug is not
yet fixed. PRs 3 and 4 are independent of the flag work and can
land in parallel with PR 1+2.

| Day | PR | Scope | Depends on |
|-----|----|-------|------------|
| 1-2 | `feat(flags): tenants.feature_flags JSONB + middleware` | New column + migration; `FeatureFlag` middleware; admin route to toggle; tests for flag-off → 404, flag-on → 200, unknown flag → panic-safe default-false | — |
| 2-4 | `fix(api): 6 broken endpoints from #152` | Per-endpoint root-cause fix. Each commit flips one scenario's assertion from broken status to `HTTP 200` and removes its row from the known-broken table. | PR 1 (uses the flag) |
| 3-4 | `feat(ops): operator checklist (CORS, pool, healthcheck)` | CORS default = `[]` (reject); `PgPool` max_conns from env; verify JWT-secret env enforcement; verify OTLP default; log-redaction module added | independent |
| 4   | `ops(backup): pg_dump cron + README` | Daily pg_dump to `/var/backups/turerp/`, 7-day retention, restore procedure in `RUNBOOK.md` | independent |
| 5   | Buffer / review / merge all 4 | Adversarial review per CLAUDE.md (2 reviewers on each PR ≥ 3 files) | — |

**End of week 1:** hurl suite 16/16 green, all 6 known-broken
scenarios flipped to 200, feature flag altyapısı merge'lenmiş,
operator checklist 9/9 satisfied for single-host.

### 5.2 Week 2 — Operate + pilot

| Day | PR / task | Scope |
|-----|-----------|-------|
| 1   | `docs: RUNBOOK.md` | 8 incident scenarios, copy-pasteable commands |
| 2   | `feat(rollback): down.sql + replay` | Migration runner learns `MIGRATIONS_DOWN` env; DLQ replay job |
| 3   | `test(hurl): tier 2 senaryoları` | 4 new scenarios covering the four tier-2 modules (manufacturing, projects, shifts, payroll), plus 1 for GraphQL, plus 1 for file upload — 6 total, all gated-flag-aware (assert 404 when flag is off, 200 when flag is on) |
| 4   | `infra: HTTPS via Caddy` | Caddyfile at repo root, auto-TLS via Let's Encrypt, smoke-tested |
| 5-9 | **Pilot** | Internal mock tenant, 7 days, synthetic traffic, observe: 5xx rate, p99 latency, audit DLQ depth, flag-toggle latency |

**End of week 2:** hurl 22/22 green, runbook reviewed by a
second person, pilot complete with zero Sev-1.

### 5.3 Gate ("definition of done")

- [ ] hurl 22/22 green (16 + 6 new tier-2)
- [ ] All 6 issue #152 scenarios assert 200 (or explicitly
      remain gated by flag with documented reason)
- [ ] Operator checklist 9/9 ✓
- [ ] RUNBOOK.md reviewed
- [ ] Rollback rehearsed at least once on a non-prod environment
- [ ] 7-day pilot: zero Sev-1, p99 ≤ 500 ms, zero unhandled
      panics, audit DLQ depth ≤ 100 at all times
- [ ] `docker compose config` parses without warning
- [ ] All PRs in this cut have 2-reviewer adversarial sign-off

## 6. Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Feature flag race: a request reads the flag column before a recent toggle is visible | Med | Low | The admin route returns the new flag in the response body; the operator verifies via a smoke test before announcing rollout. No eventual-consistency promise in v1. |
| OTLP endpoint misconfigured → silent data loss | Med | Med | OTLP failures are logged at WARN with the destination URL; the app does not panic if OTLP is unreachable. Documented in RUNBOOK. |
| Pilot surfaces a tier-2 bug in a route we did not smoke-test | Med | Med | The hurl tier-2 scenarios added in week 2 cover the happy path; we add an issue-per-bug template and triage on day 6 of the pilot. |
| pg_dump on a busy DB locks tables | Low | High | Use `pg_dump --no-acquire --serializable-deferrable`; dump to a separate volume; restore procedure tested on a fresh DB at least once before the pilot |
| JobService cron never fires (regression of PR #144) | Low | Med | Pilot includes a daily check that the `daily_cleanup` job ran; alert if it missed |
| 5xx returned by gated routes when flag is half-on | Low | Low | Feature flag middleware runs **before** the route handler; gated route never reaches handler code |
| All 6 issue #152 fixes take longer than 4 days | Med | Med | Each fix is its own PR; the slower fixes can spill into week 2 with a flag-gated fallback. Pilot does not require all 6 fixes — only the 200 path on at least the happy-path route per module. |

## 7. Owners (placeholder — fill at sprint kickoff)

| Workstream | Lead | Reviewer 1 | Reviewer 2 |
|------------|------|------------|------------|
| Feature flag | TBD | TBD | TBD |
| Issue #152 fixes | TBD | TBD | TBD |
| Operator checklist | TBD | TBD | TBD |
| Backup / restore | TBD | TBD | TBD |
| Runbook | TBD | TBD | TBD |
| Hurl tier 2 | TBD | TBD | TBD |
| Pilot | TBD | — | — |

## 8. Definition of Done — per-PR

Every PR in this cut must satisfy:

- [ ] `cargo test --lib` passes
- [ ] `cargo clippy -- -D warnings` clean
- [ ] `cargo fmt --check` clean
- [ ] Adversarial review by 2 reviewers (if ≥ 3 files touched,
      per CLAUDE.md)
- [ ] Live hurl suite (or per-PR smoke) verified against a
      running container
- [ ] No new `.unwrap()` in non-test code
- [ ] No new env var with a default that "works" for secrets
- [ ] No panic in startup / config parse
- [ ] If a migration is added: re-runs cleanly on a fresh DB
      **and** on a snapshot of the current schema
- [ ] If a route is added/changed: registered in
      `api/mod.rs::paths`, OpenAPI annotation present, hurl
      scenario added or updated
- [ ] PR body references a tracking issue

## 9. Open questions

- Where does `feature_flags` live: on `tenants`, or in a separate
  `tenant_feature_flags` table? — leaning tenants column for v1,
  separate table if a flag ever needs metadata (description,
  rollout %)
- Should the admin route be admin-only, or tenant-admin? — admin
  only for v1; tenant-admin is a P3 follow-up
- Are we OK with breaking PR-merges if a tier-2 bug is found
  during the pilot? — yes, pilot is gated by zero Sev-1, fixes
  land in week 3+
