# Tier 2 — IP whitelist. Admin-only tenant-scoped resource: regular user
# MUST get 403, unauthenticated 401. RBAC fence.

# --- 1. List IP whitelist rules as regular user — 403 ---
GET {{base_url}}/api/v1/ip-whitelist
Authorization: Bearer {{access_token}}
HTTP 403

# --- 2. No auth — 401 ---
GET {{base_url}}/api/v1/ip-whitelist
HTTP 401