# TurERP — On-call Runbook

**First step in any incident:** check `/health/ready` and `docker compose ps`.

This runbook is the **first thing an on-call reads**. Eight incident
scenarios cover the failure modes observed during the 2-week GA cut
pilot (see `docs/superpowers/specs/2026-06-10-production-release-design.md`
§ 4.4 for the design rationale). Each section ends with an "if not
resolved" escalation note.

**Conventions:**

- All commands assume you are on the host that runs `turerp` in
  docker-compose. The app container is `turerp`, the DB is `db`,
  Redis is `redis`.
- Database connection string used in commands: `psql -U turerp -d turerp`
  inside the `db` container.
- The `ADMIN_TOKEN` is a short-lived JWT issued to the `admin` user in
  tenant 1. Get one with the standard login flow.
- Times are UTC unless otherwise noted; the host runs in UTC.

## 1. Service is down

**Symptoms:** `/health/ready` returns non-200; load balancer
health-check fails; customers see 502/503 from the Caddy reverse proxy
(see PR for the TLS path — will be added in this release cut).

**Triage:**

1. Is the app container running?

   ```bash
   docker compose ps
   docker compose logs --tail=200 turerp
   ```

2. Is the database up?

   ```bash
   docker compose ps db
   docker compose exec -T db pg_isready -U turerp
   ```

3. Is Redis up?

   ```bash
   docker compose ps redis
   docker compose exec -T redis redis-cli ping
   ```

**Common root causes:**

- **App crash-looping on startup**: read the panic in
  `docker compose logs turerp`. The most common cause post-#144 is a
  config parse failure — the env vars documented in `AGENTS.md` §
  "Configuration" are required. A common regression is a missing
  `TURERP_JWT_SECRET` in a fresh environment.
- **Audit writer panic**: the recovery added in PR #144 spools rows to
  `pg_audit_dlq` (see § 6) and the process stays up. If you see the
  process restart, the panic-recovery itself failed; collect the panic
  stack and page secondary.
- **DB unreachable**: see § 5 — connection pool exhausted. The app
  fails readiness and Caddy returns 502.
- **Redis unreachable**: the rate-limiter and the job queue both
  depend on Redis. The app will refuse auth (no rate-limit state
  store) and the jobs will not run (see § 7). Restore Redis first.

**Escalation:** if the app is down for >15 min, page secondary on-call.
Sev-1: page on-call lead.

## 2. 429 storm (rate limit hit)

**Symptoms:** a tenant (or a script hammering one endpoint) is being
rate-limited; the rate-limit middleware returns `429 Too Many Requests`.

**Triage:**

1. Confirm the rate-limit stats endpoint reflects the spike:

   ```bash
   curl -s http://127.0.0.1:8080/api/v1/observability/rate-limit-stats | jq .
   ```

2. Identify the tenant and endpoint driving the spike. The
   `rate-limit-stats` response includes per-key counters.

3. If the spike is legitimate (a real customer burst), raise the
   quota by restarting the app with a higher env var:

   ```bash
   # In docker-compose.yml, the `turerp` service's environment:
   TURERP_RATE_LIMIT_PER_MINUTE=300
   docker compose up -d turerp
   ```

   The default is 60 req/min per IP. The follow-up PR will add a
   per-tenant admin route; for v1 the limit is global.

4. If the spike is abusive (a single IP), block at the Caddy layer:

   ```bash
   # Caddyfile: add a `remote_ip` deny rule, then `docker compose restart caddy`
   @abuse remote_ip 198.51.100.42
   respond @abuse 429
   ```

**Escalation:** if the rate limit is the *cause* of an outage (a
legitimate script tripped the limiter and customers see 429s for
>10 min), raise the quota and notify the affected tenant. If the
abuse is sustained (>1h from a single source), block at the firewall
and notify security.

## 3. Login lockout

**Symptoms:** a real user reports they cannot log in. The
`login_attempts` table has 5 rows for the same username within 15
minutes — the lockout window.

**Resolution:**

```bash
docker compose exec -T db psql -U turerp -d turerp -c \
  "DELETE FROM login_attempts WHERE username = 'theuser';"
```

The user can log in immediately after.

**Why this happens:** 5 wrong passwords → 15-minute lockout. The
window is hard-coded in `domain/auth/service.rs::authenticate`. PR
#144 added this as a brute-force defense. The lockout is per-username,
not per-IP, so a shared office NAT will lock out a whole team after 5
distinct users mistype the password.

**Escalation:** if the lockout is recurring (a real user keeps
mistyping), have them reset their password via the
`/api/v1/auth/forgot-password` flow. The 15-minute window is
intentional — do not raise it without a security review.

## 4. OOM / container restart loop

**Symptoms:** the `turerp` container is in a restart loop; logs end
with `Killed` rather than a panic.

**Triage:**

1. Confirm OOM kill:

   ```bash
   docker inspect turerp_turerp_1 | jq '.[0].State'
   docker inspect turerp_turerp_1 | jq '.[0].HostConfig.Memory'
   ```

2. If `OOMKilled: true`, the container hit its `mem_limit`.

**Resolution:**

Raise the memory limit in `docker-compose.yml`:

```yaml
    mem_limit: 1g   # was 512m
```

`docker compose up -d turerp`.

**Why this happens:** the default 512m limit is set for the small
test user; a pilot tenant with 10k+ rows can exhaust it on a single
large query. The memory profile grows linearly with the in-flight
request count, so the most common cause is a thundering herd on
`/api/v1/reports/*` or `/api/v1/invoices` (the cross-module invoice
list joins 4 tables and can spike to 800 MB during a full-table scan).

**Permanent fix (post-pilot):** add a per-query memory cap and stream
large responses. Filed as a v1.1 follow-up; for v1, raise the limit.

**Escalation:** if the OOM recurs after raising to 2g, there is a
real memory leak. Collect a heap profile with `docker exec turerp
kill -SIGUSR1 1` (the rust binary writes a `heap.json` to `/tmp`),
download it, and open an issue.

## 5. DB connection pool exhausted

**Symptoms:** requests hang for >30s and then time out with
`500 Internal Server Error`; logs show
`sqlx::Error::PoolTimedOut` or `connection refused`.

**Triage:**

1. Find active connections:

   ```bash
   docker compose exec -T db psql -U turerp -d turerp -c \
     "SELECT pid, state, query_start, query FROM pg_stat_activity
      WHERE state != 'idle' ORDER BY query_start;"
   ```

2. Kill idle-in-transaction sessions (the most common cause — a
   client opened a transaction and never closed it):

   ```bash
   docker compose exec -T db psql -U turerp -d turerp -c \
     "SELECT pg_terminate_backend(pid) FROM pg_stat_activity
      WHERE state = 'idle in transaction';"
   ```

3. If the issue persists, raise the pool size. The default is
   `num_cpus * 4` (set by `TURERP_DB_MAX_CONNECTIONS`).
   For the pilot we target 25.

   ```bash
   # In docker-compose.yml, the `turerp` service's environment:
   TURERP_DB_MAX_CONNECTIONS=25
   docker compose up -d turerp
   ```

**Why this happens:** Postgres has a hard limit (`max_connections`,
default 100) shared across all clients. A pool of 25 in the app
plus 5 for `psql` and replication leaves 70 for everything else. A
background job (e.g. `daily_cleanup` from § 7) running at the same
time as a tenant burst can saturate the pool.

**Escalation:** if the pool is exhausted at the default size with
no idle-in-transaction sessions, there is a connection leak in the
app. Open an issue with the `pg_stat_activity` output.

## 6. Audit DLQ growing

**Symptoms:** the `pg_audit_dlq` table is non-empty and growing;
audit events are not landing in `audit_logs`.

**Triage:**

1. Confirm the DLQ is growing:

   ```bash
   docker compose exec -T db psql -U turerp -d turerp -c \
     "SELECT count(*), min(created_at), max(created_at) FROM pg_audit_dlq;"
   ```

2. Inspect a sample row to see the error:

   ```bash
   docker compose exec -T db psql -U turerp -d turerp -c \
     "SELECT id, created_at, error_message, payload
      FROM pg_audit_dlq ORDER BY id DESC LIMIT 5;"
   ```

3. Replay the DLQ (added in the rollback-replay PR — pending):

   ```bash
   docker compose exec -T turerp /app/turerp replay-audit-dlq
   ```

   The replay CLI drains the DLQ into `audit_logs` and exits 0 on
   success. The CLI is idempotent — running it twice is safe.

4. If the DLQ keeps growing after replay, the underlying writer is
   broken. The error message in the DLQ row will tell you why
   (e.g. `foreign key violation`, `permission denied`).

**Why this happens:** the audit writer (added in PR #144) is on
the request hot path — every state-changing endpoint writes a row.
If the write fails (e.g. DB is in read-only mode during a backup,
or a constraint is violated by a regression), the row is spooled
to the DLQ rather than blocking the request. The DLQ is the
recovery path: replay once the writer is healthy.

**Escalation:** if the DLQ depth exceeds 100 (the pilot gate), the
writer is broken and audit coverage is at risk. Page secondary.

## 7. Job not running

**Symptoms:** a scheduled job (e.g. `daily_cleanup`,
`recompute_balances`) is not running; the `last_run_at` is stale
by more than 2x the cron interval.

**Triage:**

1. List the jobs and their last-run status:

   ```bash
   curl -s http://127.0.0.1:8080/api/v1/jobs | jq '.items[] | {name, last_run_at, last_status}'
   ```

2. Manual trigger (admin scope required):

   ```bash
   curl -X POST http://127.0.0.1:8080/api/v1/jobs/daily_cleanup/run \
     -H "Authorization: Bearer $ADMIN_TOKEN"
   ```

3. If the manual trigger also fails, check the job logs:

   ```bash
   docker compose logs turerp | grep -i 'daily_cleanup\|job_scheduler'
   ```

**Why this happens:** the job scheduler was hardened in PR #144
(cron parser fix). The most common cause of "job not running" is
that the app restarted during the cron window and the in-memory
scheduler state was lost. The next tick after restart re-registers
the schedule, but the missed run is not back-filled. Trigger
manually if the missed run is critical.

**Escalation:** if a job is missing for >25h (a full cron cycle
plus grace), the scheduler is broken. Restart the app:
`docker compose restart turerp`. If the issue persists, the cron
config in the DB is wrong — check the `jobs` table.

## 8. Backup failure

**Symptoms:** no backup file in `/var/backups/turerp/` for >25h;
the cron log `/var/log/turerp-backup.log` has an error.

**Triage:**

1. Confirm the cron is even running:

   ```bash
   ls -lt /var/backups/turerp/ | head -5
   tail -50 /var/log/turerp-backup.log
   ```

2. Run the backup manually:

   ```bash
   /opt/turerp/scripts/backup_pg.sh
   ```

3. If `pg_dump` blocks, the most common cause is a long-running
   transaction holding a lock. See § 5 — kill idle-in-transaction
   sessions and retry.

4. If the backup file is empty (0 bytes), the container is not
   running. Start the DB:

   ```bash
   docker compose up -d db
   /opt/turerp/scripts/backup_pg.sh
   ```

5. Verify the dump is restorable (in a fresh DB, not the live one):

   ```bash
   docker exec -i turerp_db_1 createdb -U turerp turerp_verify
   docker exec -i turerp_db_1 pg_restore -U turerp -d turerp_verify \
     < /var/backups/turerp/turerp-$(date -u +%Y%m%d).sql.gz
   docker compose exec -T db psql -U turerp -d turerp_verify -c "\dt" | head
   docker exec -i turerp_db_1 dropdb -U turerp turerp_verify
   ```

**Why this happens:** the backup script uses
`--no-acquire --serializable-deferrable` (per the safety review in
the design spec § 6) to avoid locking tables on a busy DB. These flags are the
safer choice but slower; on a heavily-loaded DB the dump can take
>10 min. If the cron window is too tight, the dump runs over the
next cron tick and skips it. The default cron is `17 2 * * *`
(off the :00 spike), which leaves a 23h43m buffer — enough for
dumps up to 1h on a 1 GB DB.

**Escalation:** if the last good backup is >48h old, the data
window is at risk. Trigger a manual dump, verify it is
restorable, and file an incident. The pilot gate requires
backups to be no older than 25h at all times.

---

## Off-hours escalation

| Severity | Definition | Action |
|----------|-----------|--------|
| **Sev-1** | Service down for >15 min OR audit DLQ depth > 100 OR data-loss risk (no backup for >48h) | Page on-call lead immediately. Notify CEO. |
| **Sev-2** | Degraded but no customer impact (e.g. one tenant's rate limit too low, a single job missed, a backup file with one bad row) | Next business day. Open a follow-up issue. |
| **Sev-3** | Cosmetic / docs / non-urgent | Backlog. Open an issue and tag `runbook-followup`. |

**Page rotation:** see the team wiki (link in
`docs/OPERATIONS.md`, TBD). The on-call lead has the authority to
escalate Sev-2 to Sev-1 if customer impact is reported.

## How this runbook stays accurate

This runbook is updated as part of the **post-pilot retrospective**
(task 9.4 in the production-release plan). Any incident that
required a step *not* in this runbook is a runbook gap; the fix is
to add the step before closing the incident ticket. The runbook is
the institutional memory of the on-call rotation.

**Owners:** SRE on-call (rotation). PRs to this file go through the
production-release cut reviewer.
