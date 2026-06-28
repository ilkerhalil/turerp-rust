# 100_stock_write — WRITE-FENCE for the Warehouse CRUD lifecycle + RBAC for
# the un-gated /stock/movements and /stock/levels DELETE routes.
#
# Covers (admin-gated writes + auth reads):
#   POST   /stock/warehouses                 create (201) → capture wh_id
#   GET    /stock/warehouses                 list (decode fence, paginated)
#   GET    /stock/warehouses/{id}            by-id (decode fence)
#   PUT    /stock/warehouses/{id}            update
#   GET    /stock/warehouses/deleted         list-deleted (decode fence, bare Vec)
#   DELETE /stock/warehouses/{id}            soft delete (MessageResponse)
#   GET    /stock/warehouses/{id}            404 (soft-deleted hidden)
#   PUT    /stock/warehouses/{id}/restore    restore
#   GET    /stock/warehouses/{id}            200 (restored)
#   DELETE /stock/warehouses/{id}/destroy    hard delete (204)
#   GET    /stock/warehouses/{id}            404 (permanently destroyed)
#   RBAC: POST create as testuser → 403; no auth → 401.
#   RBAC: DELETE /stock/movements/{id} + /stock/levels/... → 403 regular / 401 no-auth.
#
# Why: the read-only 14_stock_items.hurl only fences the (paginated) list + a
# 404 by-id. WarehouseRow decode bugs in the create/by-id/update/list-deleted/
# soft-delete/restore/destroy paths stay hidden until a warehouse is seeded.
# This fence self-seeds a throwaway warehouse and exercises every transition.
#
# FLAG: the `core.stock.warehouses` broken-endpoint gate (issue #152-5) was
# seeded DISABLED in 036 and is flipped ON by migration 047 once the handler is
# fixed. The handler had TWO root causes, both fixed in this PR:
#   (a) WarehouseRow declares `total_count: Option<i64>` but create/find_by_id/
#       find_by_tenant/update/find_deleted SELECT|RETURNING did not include it
#       → sqlx FromRow "column total_count does not exist" → 500 on GET/POST.
#       Fixed by selecting `NULL::bigint AS total_count` on those paths.
#   (b) warehouses had a `update_warehouses_updated_at BEFORE UPDATE` trigger
#       (003:100) calling `NEW.updated_at = NOW()` but NO updated_at column →
#       "record NEW has no field updated_at" → 500 on PUT/soft-delete/restore.
#       Fixed by migration 047 adding `updated_at TIMESTAMPTZ DEFAULT NOW()`.
# With the flag ON, the GlobalGate (src/middleware/gate.rs) passes the
# /stock/warehouses segment through to the handler+extractor.
#
# MIDDLEWARE ORDER (main.rs .wrap stack — last wrap is outermost, runs first):
#   JwtAuth (outermost) → GlobalGate → handler(+extractors).
# So: no token → JwtAuth 401. Valid token → GlobalGate (flag ON → pass). Writes
# then hit AdminUser → testuser 403, admin 200/201. Reads hit AuthUser → any
# authed 200.
#
# RE-RUN SAFETY: the warehouse is hard-destroyed at its final lifecycle step,
# which frees the NON-partial unique slot warehouses(code, tenant_id)
# (idx_warehouses_code_tenant, 003) — so re-running with code "HURLWH-100" does
# NOT 409. The destroy is a plain DELETE; a freshly-created warehouse has no
# stock_levels/stock_movements children, so no FK RESTRICT blocks it.
# (destroy now has a 409 guard: a warehouse WITH stock_levels/movements
# returns 409 instead of 500 — same class as the bank/invoice destroy fix.
# This fence only exercises the childless → 204 path.)
# ASSUMPTION: create_warehouse injects tenant_id from admin_user (handler
# overrides the body value), so the body tenant_id is informational only.
# ASSUMPTION: WarehouseResponse serializes company_id (hardcoded 0 by
# From<WarehouseRow> — a pre-existing phantom, not asserted here).
# run-all.sh SKIPS this file when admin_token is empty.

# --- 1. Admin POST create warehouse → 201; capture wh_id ---
POST {{base_url}}/api/v1/stock/warehouses
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "company_id": 1,
  "code": "HURLWH-100",
  "name": "Hurl Test Warehouse",
  "address": "Istanbul, TR"
}
```
HTTP 201
[Captures]
wh_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.tenant_id" isNumber
jsonpath "$.company_id" isNumber
jsonpath "$.code" == "HURLWH-100"
jsonpath "$.name" == "Hurl Test Warehouse"
jsonpath "$.address" == "Istanbul, TR"
jsonpath "$.is_active" isBoolean
jsonpath "$.created_at" exists

# --- 2. GET warehouses list (auth) — DECODE FENCE: paginated, self-seeded row present ---
GET {{base_url}}/api/v1/stock/warehouses
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber
jsonpath "$.items[?(@.id=={{wh_id}})].code" contains "HURLWH-100"

# --- 3. GET warehouse by-id (auth) — DECODE FENCE: single-row Warehouse ---
GET {{base_url}}/api/v1/stock/warehouses/{{wh_id}}
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{wh_id}}
jsonpath "$.code" == "HURLWH-100"
jsonpath "$.name" == "Hurl Test Warehouse"
jsonpath "$.address" == "Istanbul, TR"
jsonpath "$.is_active" isBoolean
jsonpath "$.created_at" exists

# --- 4. PUT update warehouse (admin) → 200; name + is_active reflect new values ---
PUT {{base_url}}/api/v1/stock/warehouses/{{wh_id}}
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "name": "Hurl Test Warehouse (renamed)",
  "is_active": false
}
```
HTTP 200
[Asserts]
jsonpath "$.id" == {{wh_id}}
jsonpath "$.name" == "Hurl Test Warehouse (renamed)"
jsonpath "$.is_active" == false

# --- 5. GET /warehouses/deleted (admin) — DECODE FENCE: bare Vec<WarehouseResponse> ---
GET {{base_url}}/api/v1/stock/warehouses/deleted
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 6. DELETE soft (admin) → 200 MessageResponse ---
DELETE {{base_url}}/api/v1/stock/warehouses/{{wh_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.message" exists

# --- 7. GET by-id → 404 (soft-deleted hidden by `deleted_at IS NULL` filter) ---
GET {{base_url}}/api/v1/stock/warehouses/{{wh_id}}
Authorization: Bearer {{access_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 8. PUT restore (admin) → 200 WarehouseResponse ---
PUT {{base_url}}/api/v1/stock/warehouses/{{wh_id}}/restore
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{wh_id}}
jsonpath "$.name" == "Hurl Test Warehouse (renamed)"

# --- 9. GET by-id → 200 (restored) ---
GET {{base_url}}/api/v1/stock/warehouses/{{wh_id}}
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{wh_id}}

# --- 10. DELETE destroy (admin) → 204 (hard delete; fresh warehouse has no stock children) ---
# destroy guards against FK RESTRICT (stock_levels/stock_movements.warehouse_id)
# → 409 if children exist; this freshly-created warehouse has none → 204.
DELETE {{base_url}}/api/v1/stock/warehouses/{{wh_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 11. GET by-id → 404 (permanently destroyed) ---
GET {{base_url}}/api/v1/stock/warehouses/{{wh_id}}
Authorization: Bearer {{access_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# === RBAC: /stock/warehouses write (AdminUser extractor) ===

# --- 12. Regular user POST create → 403 (AdminUser first) ---
POST {{base_url}}/api/v1/stock/warehouses
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "company_id": 1,
  "code": "HURLWH-RBAC",
  "name": "should be forbidden",
  "address": null
}
```
HTTP 403
[Asserts]
jsonpath "$.error" exists

# --- 13. No-auth POST create → 401 (JwtAuth before gate/extractor) ---
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

# === /stock/movements — RBAC fence (NOT gated; AdminUser extractor) ===

# --- 14. Regular user DELETE /stock/movements/{id} → 403 ---
# /stock/movements is not gate-matched (gate rule is the exact segment
# /api/v1/stock/warehouses), so JwtAuth passes → AdminUser 403 before any DB
# lookup. id 999999 need not exist.
DELETE {{base_url}}/api/v1/stock/movements/999999
Authorization: Bearer {{access_token}}
HTTP 403
[Asserts]
jsonpath "$.error" exists

# --- 15. No-auth DELETE /stock/movements/{id} → 401 ---
DELETE {{base_url}}/api/v1/stock/movements/999999
HTTP 401
[Asserts]
jsonpath "$.error" exists

# === /stock/levels — RBAC fence (NOT gated; AdminUser extractor) ===

# --- 16. Regular user DELETE /stock/levels/warehouse/{wid}/product/{pid} → 403 ---
DELETE {{base_url}}/api/v1/stock/levels/warehouse/999999/product/999999
Authorization: Bearer {{access_token}}
HTTP 403
[Asserts]
jsonpath "$.error" exists

# --- 17. No-auth DELETE /stock/levels/warehouse/{wid}/product/{pid} → 401 ---
DELETE {{base_url}}/api/v1/stock/levels/warehouse/999999/product/999999
HTTP 401
[Asserts]
jsonpath "$.error" exists