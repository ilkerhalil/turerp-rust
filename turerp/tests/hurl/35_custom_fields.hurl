# Tier 2 — Custom fields. Admin-only resource: a regular user (testuser)
# MUST get 403, and an unauthenticated request MUST get 401 (auth runs
# before RBAC). This is the RBAC privilege-escalation fence.

# --- 1. List custom fields as regular user — 403 ---
GET {{base_url}}/api/v1/custom-fields
Authorization: Bearer {{access_token}}
HTTP 403

# --- 2. No auth — 401 ---
GET {{base_url}}/api/v1/custom-fields
HTTP 401