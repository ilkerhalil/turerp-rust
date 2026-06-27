# Tier 1 — Resilience: ADMIN happy paths (split from 63_resilience.hurl).
# NOTE: routes mounted at /api/resilience/* (NOT /api/v1/...).
# RBAC: AdminUser → admin 200/400. Skipped by run-all.sh when {{admin_token}}
# is empty (seed via scripts/seed_test_admin.sql); the read-only RBAC fence in
# 63_resilience.hurl stays green in admin-less envs.

# --- 1. GET circuit-breakers as admin — 200 + envelope ---
GET {{base_url}}/api/resilience/circuit-breakers
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.circuit_breakers" isCollection

# --- 2. GET retry-stats as admin — 200 + retry_stats object present ---
GET {{base_url}}/api/resilience/retry-stats
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.retry_stats" exists

# --- 3. POST reset valid service ("gib") as admin — 200 + message/service ---
POST {{base_url}}/api/resilience/circuit-breakers/gib/reset
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.message" isString
jsonpath "$.service" == "gib"

# --- 4. POST reset unknown service as admin — 400 (BadRequest) ---
POST {{base_url}}/api/resilience/circuit-breakers/nonexistent_service/reset
Authorization: Bearer {{admin_token}}
HTTP 400
[Asserts]
jsonpath "$.error" exists