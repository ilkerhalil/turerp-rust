# 100_stock_write — GATE + RBAC FENCE for the Stock mutation surface.
#
# Scope (reduced from a full lifecycle fence): the /stock/warehouses routes
# are gated behind the `core.stock.warehouses` feature flag (issue #152
# broken-endpoint gate), so this file fences the GATE behavior (404 when the
# flag is disabled) for all auth states, plus the RBAC layer (403 regular /
# 401 no-auth) for the un-gated /stock/movements and /stock/levels DELETE
# routes. The full warehouse/movement/level CRUD lifecycle is deferred.
#
# Why minimal: the GlobalGate (src/middleware/gate.rs, registered in main.rs
# at the App level) short-circuits /api/v1/stock/warehouses with 404 when
# `core.stock.warehouses` is disabled — and that flag is seeded DISABLED
# (migrations/036_flag_seed_defaults.sql:22, "flip to on after #152-5 fix").
# Even when the flag is ON the warehouse handler is 500/404-broken (#152), so
# the "assume flag ON" convention used by manufacturing (96/97) does NOT
# apply here. There is no way to create/list a warehouse to obtain a
# warehouse_id, and the movement + level create handlers require a
# warehouse_id FK → the movement/level CRUD lifecycle is unfenceable until
# #152-5 is fixed and the flag flipped on. This file documents the gate
# (404) so the broken-endpoint status is visible in CI instead of silently
# uncovered, and fences the RBAC layer that the gate hides for warehouses.
#
# MIDDLEWARE ORDER (main.rs .wrap stack — last wrap is outermost, runs first):
#   JwtAuth (outermost) → GlobalGate → handler(+extractors).
# So: no token → JwtAuth 401 (before the gate). Valid token of ANY role →
# GlobalGate 404 (flag disabled, before the handler/extractor). The AdminUser
# extractor therefore NEVER runs for /stock/warehouses → there is no 403 for
# warehouses (testuser gets 404, same as admin).
#
# /stock/movements and /stock/levels are NOT gated (the gate rule is the
# exact segment /api/v1/stock/warehouses — segment-aware prefix matching,
# gate.rs:254-260, so /stock/movements does NOT match). Their DELETE routes
# use the AdminUser extractor → testuser 403, no-auth 401 (JwtAuth). We fence
# those with a non-existent id (999999) — the extractor 403/401 fires before
# any DB lookup, so the id need not exist.
#
# run-all.sh SKIPS this file when admin_token is empty (needed for the admin
# gate-404 assertions to distinguish 401-vs-404).

# === /stock/warehouses — GATE fence (flag disabled → 404 for any authed role) ===

# --- 1. No-auth POST /stock/warehouses → 401 (JwtAuth runs before the gate) ---
POST {{base_url}}/api/v1/stock/warehouses
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "company_id": 1,
  "code": "HURLWH-NOAUTH",
  "name": "should be unauth",
  "address": null
}
```
HTTP 401
[Asserts]
jsonpath "$.error" exists

# --- 2. Regular user POST /stock/warehouses → 404 (gate disabled; NOT 403) ---
# Authed token passes JwtAuth, then GlobalGate sees core.stock.warehouses
# disabled → 404 JSON envelope {"error":"Not found"} (gate.rs:317). The
# AdminUser extractor never runs, so this is 404 (not the 403 a normal admin
# route would return for a regular user).
POST {{base_url}}/api/v1/stock/warehouses
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "company_id": 1,
  "code": "HURLWH-RBAC",
  "name": "should be forbidden by gate",
  "address": null
}
```
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 3. Admin POST /stock/warehouses → 404 (gate disabled; NOT 201) ---
# Same gate short-circuit as step 2 — admin role does not bypass the flag.
# The 201 (create) path is unreachable until #152-5 is fixed + flag flipped on.
POST {{base_url}}/api/v1/stock/warehouses
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "company_id": 1,
  "code": "HURLWH",
  "name": "Hurl Test Warehouse",
  "address": "Istanbul, TR"
}
```
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 4. No-auth GET /stock/warehouses → 401 ---
GET {{base_url}}/api/v1/stock/warehouses
HTTP 401
[Asserts]
jsonpath "$.error" exists

# --- 5. Regular user GET /stock/warehouses → 404 (gate disabled) ---
GET {{base_url}}/api/v1/stock/warehouses
Authorization: Bearer {{access_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# === /stock/movements — RBAC fence (NOT gated; AdminUser extractor) ===

# --- 6. Regular user DELETE /stock/movements/{id} → 403 (AdminUser first) ---
# /stock/movements is not gate-matched, so JwtAuth passes → handler → AdminUser
# extractor 403 (role=user) before any DB lookup. id 999999 need not exist.
DELETE {{base_url}}/api/v1/stock/movements/999999
Authorization: Bearer {{access_token}}
HTTP 403
[Asserts]
jsonpath "$.error" exists

# --- 7. No-auth DELETE /stock/movements/{id} → 401 ---
DELETE {{base_url}}/api/v1/stock/movements/999999
HTTP 401
[Asserts]
jsonpath "$.error" exists

# === /stock/levels — RBAC fence (NOT gated; AdminUser extractor) ===

# --- 8. Regular user DELETE /stock/levels/warehouse/{wid}/product/{pid} → 403 ---
DELETE {{base_url}}/api/v1/stock/levels/warehouse/999999/product/999999
Authorization: Bearer {{access_token}}
HTTP 403
[Asserts]
jsonpath "$.error" exists

# --- 9. No-auth DELETE /stock/levels/warehouse/{wid}/product/{pid} → 401 ---
DELETE {{base_url}}/api/v1/stock/levels/warehouse/999999/product/999999
HTTP 401
[Asserts]
jsonpath "$.error" exists