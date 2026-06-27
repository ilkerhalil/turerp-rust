# Tier 2 — e-Defter Blockchain (hash-chain, merkle-tree, verify, hash-state).
# Routes live at /api/v1/edefter/periods/{id}/<op> (configured under /api).
# RBAC map (from src/api/v1/edefter_blockchain.rs):
#   GET  hash-chain    → AuthUser (ALWAYS 200, even for unknown period:
#                         service returns Ok(empty) → {period_id,count:0,entries:[]})
#   GET  hash-state    → AuthUser (200 when state exists; 404 when none for period)
#   POST merkle-tree  → AdminUser (testuser 403; admin 200/400)
#   POST verify       → AdminUser (testuser 403; admin 200/404)
# Why: edefter_blockchain module had ZERO hurl coverage. We fence the
# non-existent-period path (999999) on the GET routes, the admin-only
# RBAC 403 on the POST routes, and no-auth 401 across all four.

# --- 1. GET hash-chain on non-existent period — 200 (handler returns
#      200 with {period_id, count:0, entries:[]} for an unknown period,
#      NOT 404 — verified in src/api/v1/edefter_blockchain.rs:35) ---
GET {{base_url}}/api/v1/edefter/periods/999999/hash-chain
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.period_id" == 999999
jsonpath "$.count" isNumber
jsonpath "$.entries" isCollection

# --- 2. GET hash-chain without auth — 401 ---
GET {{base_url}}/api/v1/edefter/periods/999999/hash-chain
HTTP 401
[Asserts]
jsonpath "$.error" exists

# --- 3. GET hash-state on non-existent period — 404 (handler returns
#      NotFound with { "error": "Hash state not found for this period" }) ---
GET {{base_url}}/api/v1/edefter/periods/999999/hash-state
Authorization: Bearer {{access_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 4. GET hash-state without auth — 401 ---
GET {{base_url}}/api/v1/edefter/periods/999999/hash-state
HTTP 401
[Asserts]
jsonpath "$.error" exists

# --- 5. POST merkle-tree — RBAC: admin-only (AdminUser) → testuser 403 ---
POST {{base_url}}/api/v1/edefter/periods/999999/merkle-tree
Authorization: Bearer {{access_token}}
HTTP 403

# --- 6. POST merkle-tree without auth — 401 ---
POST {{base_url}}/api/v1/edefter/periods/999999/merkle-tree
HTTP 401
[Asserts]
jsonpath "$.error" exists

# --- 7. POST verify — RBAC: admin-only (AdminUser) → testuser 403 ---
POST {{base_url}}/api/v1/edefter/periods/999999/verify
Authorization: Bearer {{access_token}}
HTTP 403

# --- 8. POST verify without auth — 401 ---
POST {{base_url}}/api/v1/edefter/periods/999999/verify
HTTP 401
[Asserts]
jsonpath "$.error" exists