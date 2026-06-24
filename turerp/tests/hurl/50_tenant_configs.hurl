# Tier 2 — Per-tenant configuration (TenantConfig REST API). The list
# returns a plain JSON array (not the paginated envelope).

# --- 1. Tenant configs list — 200, plain array ---
GET {{base_url}}/api/v1/tenant-configs
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 2. Lookup by non-existent key/id — 404 ---
GET {{base_url}}/api/v1/tenant-configs/nonexistent-key-xyz
Authorization: Bearer {{access_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 3. No auth — 401 ---
GET {{base_url}}/api/v1/tenant-configs
HTTP 401