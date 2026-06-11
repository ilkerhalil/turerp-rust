# Hurl Live Smoke Tests

End-to-end smoke tests for the turerp API, executed against a **running
container**. Each `.hurl` file is a self-contained scenario: one or more
HTTP requests with assertions on the response.

The suite is **not** a replacement for `cargo test`. It is a complement:
unit tests verify code paths in isolation, hurl verifies the assembled
system is actually answering the questions the OpenAPI spec describes.

## Why

- **Live auth smoke test rule** (AGENTS.md): any PR that touches auth,
  user, MFA, JWT, password, or RBAC must run a live smoke test. This
  directory is the reusable form of that test.
- **Multi-tenant boundary check**: every list endpoint is asserted to
  return only items from the caller's tenant.
- **RBAC negative paths**: regular users are expected to get `403` on
  admin-only resources; this prevents silent privilege escalation.
- **Regression fence for known P0/P1**: a few scenarios assert a
  *deliberate* `404` or `500` to keep visible the bugs that are still
  pending fix (currencies, settings, categories, units, leave-types,
  stock/warehouses). When the underlying route is fixed, the scenario
  flips its assertion to `200` and starts gating merges.

## Setup

### 1. Install hurl

```bash
cargo install hurl --locked
```

This is a one-time install (~5–10 min from source). Prebuilt binaries
are also available from <https://hurl.dev/docs/installation.html>.

### 2. Start the stack

```bash
cd turerp
docker compose up -d
# wait for /health/ready to return 200
until curl -sf http://127.0.0.1:8080/health/ready >/dev/null; do sleep 2; done
```

### 3. Seed the testuser (one-time per DB)

The hurl suite logs in as `testuser` (tenant 1). If it doesn't exist
yet, register it:

```bash
curl -X POST http://127.0.0.1:8080/api/v1/auth/register \
  -H 'Content-Type: application/json' \
  -d '{
    "username":"testuser",
    "email":"testuser@turerp.local",
    "password":"TestUser123!Pass",
    "tenant_id":1,
    "full_name":"Test User"
  }'
```

The password is a smoke-test fixture, not a real credential.

> **If you see HTTP 401 from the wrapper's login**: 5 consecutive wrong
> passwords lock the user out for 15 minutes. Clear via:
> ```bash
> docker compose exec -T db psql -U turerp -d turerp \
>   -c "DELETE FROM login_attempts WHERE username = 'testuser';"
> ```

## Running

### All scenarios

```bash
./run-all.sh
```

The wrapper logs in once via `curl` and writes `access_token`,
`refresh_token`, `user_id`, and `tenant_id` to a temp variables file,
then invokes every `NN_*.hurl` with `--variables-file` and a 2-second
pause between scenarios to keep the request rate under the per-IP
governor limit (default 60 req/min). Prints a pass/fail summary and
exits non-zero on any failure.

### Single scenario

```bash
hurl --test \
  --variable base_url=http://127.0.0.1:8080 \
  --variable access_token="$ACCESS_TOKEN" \
  01_auth.hurl
```

### Custom base URL / password

```bash
BASE_URL=http://staging.turerp.local \
TURERP_TEST_PASSWORD='real-test-pw' \
  ./run-all.sh
```

## Layout

```
tests/hurl/
├── README.md            # this file
├── run-all.sh           # one-shot login + invokes every NN_*.hurl
├── .env.example         # documented test_password value
└── 01_auth.hurl
    02_health.hurl
    03_users.hurl
    ...
    16_settings.hurl
    17_manufacturing.hurl
    18_projects.hurl
    19_shifts.hurl
    20_payroll.hurl
    21_graphql.hurl
    22_file_upload.hurl
```

### Why not hurl's native multi-file?

Hurl 8.x does not support cross-file capture sharing (the
`{{ import { ... } from "..." }}` directive is a planned feature, not
shipped). The wrapper script (login once, pass a variables-file) is the
simplest workaround until the upstream feature lands.

### Tier breakdown

| Tier | Files | Style |
|------|-------|-------|
| 1 — root resources | 01–10 | auth, health, users, basic CRUD, RBAC, negative paths |
| 1.5 — domain | 11–16 | HR, accounting, CRM, stock, invoices, settings |
| 2 — gated modules | 17–22 | manufacturing, projects, shifts, payroll, graphql, file upload |

Tier 2 scenarios all assert the **default-OFF** state of the
`tier2.*` feature flags. When the operator enables a flag via
`/api/v1/feature-flags/{id}/enable`, the same routes return 200
(positive assertions are run by enabling the flag and re-running
the suite — see RUNBOOK § 5 for the flip procedure).

### Known-broken scenarios (assert the bug)

| File | Endpoint | Status | Tracking |
|------|----------|--------|----------|
| `04_categories.hurl` | `GET /api/v1/categories` | `500` | follow-up to #152 |
| `05_units.hurl` | `GET /api/v1/units` | `500` | follow-up to #152 |
| `06_currencies.hurl` | `GET /api/v1/currencies` | `404` | follow-up to #152 |
| `11_hr_employees.hurl` | `GET /api/v1/hr/leave-types` | `500` | follow-up to #152 |
| `14_stock_items.hurl` | `GET /api/v1/stock/warehouses` | `500` | follow-up to #152 |
| `16_settings.hurl` | `GET /api/v1/settings` | `404` | follow-up to #152 |

These scenarios exist **on purpose**: they assert the broken status, so
that when the underlying route is fixed, the scenario fails loudly and
forces an update. This is cheaper than letting a "this works" assertion
slip past code review.

## Adding a new scenario

1. Pick a name: `NN_<resource>.hurl` (next available number).
2. Reference the shared variables from `--variables-file`:

   ```hurl
   GET {{base_url}}/api/v1/<resource>
   Authorization: Bearer {{access_token}}
   HTTP 200
   [Asserts]
   jsonpath "$.items" isCollection
   ```

3. Write at least one happy path and one negative path per security
   boundary. If the endpoint is known to be broken, flip the assertion
   to the observed status and add a comment pointing at the tracking
   issue.
4. Run `./run-all.sh` to confirm.

## Out of scope

- Manufacturing, projects, shifts, payroll (Tier 2 — by-design deferred)
- Performance / load testing (use k6, wrk)
- Contract / OpenAPI drift testing (use spectral)
- Frontend E2E (use Playwright)
- CI workflow integration (deferred to a follow-up PR)
