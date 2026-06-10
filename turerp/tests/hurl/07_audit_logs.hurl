# Tier 1 — Audit logs. RBAC test: regular user (testuser) is denied,
# 401 is returned for anonymous. Admin role would return 200.

# --- 1. As testuser (role=user) — must be 403 ---
GET {{base_url}}/api/v1/audit-logs
Authorization: Bearer {{access_token}}
HTTP 403
