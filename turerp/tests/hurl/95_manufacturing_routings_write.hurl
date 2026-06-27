# 95_manufacturing_routings_write — WRITE-FENCE for the Routing CRUD
# lifecycle + routing operations + by-product lookup + material-requirements.
#
# Covers: POST /manufacturing/routings → GET /routings/deleted (decode fence)
#         → GET /routings/{id} (decode fence) → DELETE /routings/{id} (soft)
#         → GET /routings/{id} 404 → PUT /routings/{id}/restore →
#         GET /routings/{id} 200 → DELETE /routings/{id}/destroy →
#         GET /routings/{id} 404.
#         Plus: POST /routings/operations (add operation) →
#         GET /routings/product/{product_id} (decode fence, bare Vec) →
#         GET /material-requirements/{product_id}?quantity=10 (decode fence).
#
# Why: the read-only 17_manufacturing.hurl only fences the gated (404) path.
# Routing Row decode bugs in create/by-id/by-product/operations/material-
# requirements/soft-delete/restore/destroy stay hidden until a routing is
# seeded. This fence self-seeds a routing and exercises every transition.
#
# RBAC: writes (create/soft-delete/restore/destroy/list-deleted/add-operation)
# use the AdminUser extractor → testuser ({{access_token}}) gets 403, admin
# ({{admin_token}}) gets 200/201. Reads (get-by-id, by-product, material-
# requirements) use AuthUser → any authenticated user. No auth → 401.
# run-all.sh SKIPS this file when admin_token is empty.
#
# ASSUMPTION: the tier2.manufacturing feature flag is ON in this env; the
# read-only 17_manufacturing.hurl fences the gated (404) path. This file
# exercises the un-gated admin path, so it requires the flag enabled (same
# convention as 77_shifts_write.hurl).
# ASSUMPTION: GET /products is seeded non-empty by seed_sample_data.sql
# (P001/P002/P003) so items[0].id resolves as the routing's product_id.
# ASSUMPTION: rust_decimal serializes Decimal fields as JSON strings
# (setup_hours, run_hours) — matches the cari write-fence convention.
# ASSUMPTION: create_routing injects tenant_id from admin_user (handler
# overrides the body value), so the body tenant_id is informational only.
# RE-RUN SAFETY: the routing is hard-destroyed at its final step, which frees
# the NON-partial unique slot routings(product_id, version, tenant_id)
# (003:757) — so re-running with version "WF-RT-1.0" does NOT 409. The
# product parent is NOT created here (taken from GET /products), so there is
# no parent cleanup to perform.
# ASSUMPTION: GET /material-requirements/{product_id}?quantity=10 returns a
# bare Vec (may be empty if no BOM is linked to the product); the decode
# fence asserts 200 + isCollection, not non-emptiness.

# --- 0. FK capture: GET /products → product_id ---
GET {{base_url}}/api/v1/products
Authorization: Bearer {{admin_token}}
HTTP 200
[Captures]
product_id: jsonpath "$.items[0].id"
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber
jsonpath "$.total" > 0

# --- 1. POST create routing (admin) → 201; capture routing_id ---
POST {{base_url}}/api/v1/manufacturing/routings
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "product_id": {{product_id}},
  "version": "WF-RT-1.0",
  "is_active": true,
  "is_primary": true
}
```
HTTP 201
[Captures]
routing_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.tenant_id" isNumber
jsonpath "$.product_id" == {{product_id}}
jsonpath "$.version" == "WF-RT-1.0"
jsonpath "$.is_active" isBoolean
jsonpath "$.is_primary" isBoolean
jsonpath "$.created_at" exists

# --- 2. GET /routings/deleted (admin) — DECODE FENCE: bare Vec<Routing> ---
GET {{base_url}}/api/v1/manufacturing/routings/deleted
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 3. GET /routings/{id} (admin) — DECODE FENCE: single-row Routing ---
GET {{base_url}}/api/v1/manufacturing/routings/{{routing_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{routing_id}}
jsonpath "$.product_id" == {{product_id}}
jsonpath "$.version" == "WF-RT-1.0"
jsonpath "$.is_active" isBoolean
jsonpath "$.is_primary" isBoolean
jsonpath "$.deleted_at" exists
jsonpath "$.created_at" exists

# --- 4. POST /routings/operations (admin) → 201; add an operation ---
POST {{base_url}}/api/v1/manufacturing/routings/operations
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "routing_id": {{routing_id}},
  "sequence": 10,
  "operation_name": "WF Assembly",
  "work_center_id": null,
  "setup_hours": "0.50",
  "run_hours": "2.00",
  "description": "hurl-seeded routing operation"
}
```
HTTP 201
[Captures]
routing_op_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.routing_id" == {{routing_id}}
jsonpath "$.sequence" isNumber
jsonpath "$.operation_name" == "WF Assembly"
jsonpath "$.setup_hours" exists
jsonpath "$.run_hours" exists

# --- 5. GET /routings/product/{product_id} (auth) — DECODE FENCE: bare Vec, self-seeded row present ---
GET {{base_url}}/api/v1/manufacturing/routings/product/{{product_id}}
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection
jsonpath "$[?(@.id=={{routing_id}})].version" contains "WF-RT-1.0"

# --- 6. GET /material-requirements/{product_id}?quantity=10 (auth) — DECODE FENCE ---
GET {{base_url}}/api/v1/manufacturing/material-requirements/{{product_id}}?quantity=10
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 7. DELETE soft (admin) → 200 MessageResponse ---
DELETE {{base_url}}/api/v1/manufacturing/routings/{{routing_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.message" exists

# --- 8. GET by-id → 404 (soft-deleted hidden) ---
GET {{base_url}}/api/v1/manufacturing/routings/{{routing_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 9. PUT restore (admin) → 200 Routing ---
PUT {{base_url}}/api/v1/manufacturing/routings/{{routing_id}}/restore
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{routing_id}}
jsonpath "$.version" == "WF-RT-1.0"
jsonpath "$.deleted_at" exists

# --- 10. GET by-id → 200 (restored) ---
GET {{base_url}}/api/v1/manufacturing/routings/{{routing_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{routing_id}}

# --- 11. DELETE destroy (admin) → 204 ---
DELETE {{base_url}}/api/v1/manufacturing/routings/{{routing_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 12. GET by-id → 404 (permanently destroyed) ---
GET {{base_url}}/api/v1/manufacturing/routings/{{routing_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 13. RBAC: POST create with regular user → 403 ---
POST {{base_url}}/api/v1/manufacturing/routings
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "product_id": {{product_id}},
  "version": "WF-RBAC-403",
  "is_active": true,
  "is_primary": false
}
```
HTTP 403

# --- 14. RBAC: POST create with no auth → 401 ---
POST {{base_url}}/api/v1/manufacturing/routings
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "product_id": {{product_id}},
  "version": "WF-NOAUTH",
  "is_active": true,
  "is_primary": false
}
```
HTTP 401