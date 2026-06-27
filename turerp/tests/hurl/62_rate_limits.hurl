# Tier 1 — Rate-limit dashboard (admin-only monitoring).
# NOTE: route is mounted at /api/admin/rate-limits (NOT /api/v1/...).
# RBAC (from src/api/v1/rate_limits.rs): AdminUser extractor → testuser 403,
# admin 200, no-auth 401.
# Why: rate_limits module had ZERO hurl coverage. This file fences the
# admin-only RBAC boundary (403 for a regular user) and the no-auth 401 —
# both run in every environment (no admin token needed). The admin happy
# path (200 + dashboard envelope) lives in 62_rate_limits_admin.hurl,
# which run-all.sh skips when no admin_token is available.

# --- 1. GET rate-limits as testuser — RBAC 403 (AdminUser required) ---
GET {{base_url}}/api/admin/rate-limits
Authorization: Bearer {{access_token}}
HTTP 403

# --- 2. GET rate-limits without auth — 401 (auth runs before RBAC) ---
GET {{base_url}}/api/admin/rate-limits
HTTP 401
[Asserts]
jsonpath "$.error" exists