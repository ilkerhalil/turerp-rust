# Tier 2 — LDAP/AD integration. config GET returns 404 when no LDAP
# config row exists (none seeded); test and sync are POST-only.

# --- 1. LDAP config — 404 (no config seeded) ---
GET {{base_url}}/api/v1/ldap/config
Authorization: Bearer {{access_token}}
HTTP 404

# --- 2. LDAP test — POST-only route, GET 405 ---
GET {{base_url}}/api/v1/ldap/test
Authorization: Bearer {{access_token}}
HTTP 405

# --- 3. LDAP sync — POST-only route, GET 405 ---
GET {{base_url}}/api/v1/ldap/sync
Authorization: Bearer {{access_token}}
HTTP 405

# --- 4. No auth — 401 ---
GET {{base_url}}/api/v1/ldap/config
HTTP 401