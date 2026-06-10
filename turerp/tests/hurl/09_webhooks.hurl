# Tier 1 — Webhooks. RBAC: list is open to authenticated users; CRUD
# mutations are admin-only. This file covers the read path and the
# negative (401 without auth) path.


# --- 1. List webhooks as testuser — 200 (list is auth-only, not admin) ---
GET {{base_url}}/api/v1/webhooks
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 2. List without auth — 401 ---
GET {{base_url}}/api/v1/webhooks
HTTP 401
[Asserts]
jsonpath "$.error" exists

# --- 3. Get non-existent webhook — 404 ---
GET {{base_url}}/api/v1/webhooks/999999
Authorization: Bearer {{access_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists
