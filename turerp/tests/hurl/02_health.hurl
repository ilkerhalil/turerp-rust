# Tier 1 — Liveness & readiness. No auth, no tenant scope.

# --- 1. Liveness: must always be 200 if the process is up ---
GET {{base_url}}/health/live
HTTP 200
[Asserts]
jsonpath "$.status" == "ok"
jsonpath "$.service" == "turerp-erp"
jsonpath "$.version" exists

# --- 2. Readiness: must be 200 only when deps (db, cache) are reachable ---
GET {{base_url}}/health/ready
HTTP 200
[Asserts]
jsonpath "$.status" == "ok"
jsonpath "$.deps.db" == "ok"
jsonpath "$.deps.cache" == "ok"

# --- 3. Observability health (OpenAPI-declared) ---
GET {{base_url}}/api/v1/observability/health
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.overall" == "Healthy"
jsonpath "$.version" exists
