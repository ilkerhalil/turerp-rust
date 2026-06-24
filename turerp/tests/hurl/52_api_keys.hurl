# Tier 2 — API keys. Admin-only: the list lives at
# /api-keys/tenant/{tenant_id} and a regular user MUST get 403.
# Unauthenticated requests get 401 (auth before RBAC).

# --- 1. List API keys for tenant 1 as regular user — 403 ---
GET {{base_url}}/api/v1/api-keys/tenant/1
Authorization: Bearer {{access_token}}
HTTP 403

# --- 2. No auth — 401 ---
GET {{base_url}}/api/v1/api-keys/tenant/1
HTTP 401