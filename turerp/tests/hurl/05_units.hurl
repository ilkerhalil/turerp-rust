# Tier 1 — Units. Same P1 as categories: GET /units is gated
# (core.units, default-disabled). PR 2 fixed the underlying handler
# (postgres_repository had a phantom `company_id` column drift that caused
# FromRow decode to fail and the handler to return 404/500). The flag is
# off by default; operator flips it on after deploy.


# --- 1. Units list (gated; returns 200 + empty paginated body) ---
GET {{base_url}}/api/v1/units
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber
jsonpath "$.page" isNumber
jsonpath "$.per_page" isNumber
