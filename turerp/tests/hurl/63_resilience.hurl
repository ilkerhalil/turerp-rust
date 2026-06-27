# Tier 1 — Resilience (circuit-breakers + retry-stats + reset).
# NOTE: routes mounted at /api/resilience/* (NOT /api/v1/...).
# RBAC (from src/api/v1/resilience.rs): all three handlers take AdminUser →
# testuser 403, admin 200/400, no-auth 401.
#   GET  /api/resilience/circuit-breakers           → 200 { circuit_breakers:[] }
#   GET  /api/resilience/retry-stats                → 200 { retry_stats:{...} }
#   POST /api/resilience/circuit-breakers/{svc}/reset → 200 { message, service }
#       unknown service → 400
# Valid services: gib, email, sms, bank, webhook, file_storage.
# Why: resilience module had ZERO hurl coverage. This file fences the
# admin-only RBAC boundary (403 for testuser) and no-auth 401 on all three
# routes — runs in every environment (no admin token needed). The admin
# happy paths (200) + the unknown-service 400 branch live in
# 63_resilience_admin.hurl, skipped by run-all.sh when no admin_token.

# --- 1. GET circuit-breakers as testuser — RBAC 403 ---
GET {{base_url}}/api/resilience/circuit-breakers
Authorization: Bearer {{access_token}}
HTTP 403

# --- 2. GET circuit-breakers without auth — 401 ---
GET {{base_url}}/api/resilience/circuit-breakers
HTTP 401
[Asserts]
jsonpath "$.error" exists

# --- 3. GET retry-stats as testuser — RBAC 403 ---
GET {{base_url}}/api/resilience/retry-stats
Authorization: Bearer {{access_token}}
HTTP 403

# --- 4. GET retry-stats without auth — 401 ---
GET {{base_url}}/api/resilience/retry-stats
HTTP 401
[Asserts]
jsonpath "$.error" exists

# --- 5. POST reset as testuser — RBAC 403 ---
POST {{base_url}}/api/resilience/circuit-breakers/gib/reset
Authorization: Bearer {{access_token}}
HTTP 403

# --- 6. POST reset without auth — 401 ---
POST {{base_url}}/api/resilience/circuit-breakers/gib/reset
HTTP 401
[Asserts]
jsonpath "$.error" exists