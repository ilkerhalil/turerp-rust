# Tier 1 — Settings. PR #145 left the routes registered as
# #[post("/settings")]/etc, mounted under /api — exposing
# /api/settings rather than the v1 path. Issue #152 flips the
# route attribute prefix to /v1/... so the GlobalGate rule
# ("/api/v1/settings" → core.settings) matches a real route.

# --- 1. Settings list — fixed by #152 ---
# The /v1/settings endpoint returns a paginated envelope with
# {data, page, per_page, total, total_pages} (uses "data" key,
# not the "items" key used by other list endpoints).
GET {{base_url}}/api/v1/settings
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.data" isCollection
jsonpath "$.total" isNumber

# --- 2. Specific setting — fixed by #152, seeded by scripts/seed_sample_data.sql ---
# The /v1/settings/{key} route is reachable. default_locale is seeded for
# tenant 1, so the handler returns 200 with the setting row.
GET {{base_url}}/api/v1/settings/default_locale
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.key" == "default_locale"
