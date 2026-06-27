# Tier 1 — Rate-limit dashboard: ADMIN happy path (split from 62_rate_limits.hurl).
# NOTE: route is mounted at /api/admin/rate-limits (NOT /api/v1/...).
# RBAC: AdminUser extractor → admin 200. Response = RateLimitDashboardResponse:
#   { total_clients, total_requests, total_blocked, total_allowed, entries:[] }
# Why: the admin 200 + envelope-shape decode fence. This file is skipped by
# run-all.sh when {{admin_token}} is empty (seed via scripts/seed_test_admin.sql),
# so the read-only RBAC fence in 62_rate_limits.hurl stays green in admin-less envs.

# --- 1. GET rate-limits as admin — 200 + dashboard envelope ---
GET {{base_url}}/api/admin/rate-limits
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.total_clients" isNumber
jsonpath "$.total_requests" isNumber
jsonpath "$.total_blocked" isNumber
jsonpath "$.total_allowed" isNumber
jsonpath "$.entries" isCollection