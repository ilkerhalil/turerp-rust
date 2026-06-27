# 94_manufacturing_boms_write — WRITE-FENCE for the Bill of Materials CRUD
# lifecycle + BOM lines + by-product lookup.
#
# Covers: POST /manufacturing/boms → GET /boms/deleted (decode fence) →
#         GET /boms/{id} (decode fence) → DELETE /boms/{id} (soft) →
#         GET /boms/{id} 404 → PUT /boms/{id}/restore → GET /boms/{id} 200 →
#         DELETE /boms/{id}/destroy → GET /boms/{id} 404.
#         Plus: GET /boms/{bom_id}/lines (decode fence, bare Vec) →
#         POST /boms/lines (add a line) → GET /boms/{bom_id}/lines (non-empty)
#         → GET /boms/product/{product_id} (decode fence, bare Vec).
#
# Why: the read-only 17_manufacturing.hurl only fences the gated (404) path.
# BOM Row decode bugs in create/by-id/lines/by-product/soft-delete/restore/
# destroy stay hidden until a BOM is seeded. This fence self-seeds a BOM and
# exercises every lifecycle transition plus the BOM-line sub-resource.
#
# RBAC: writes (create/soft-delete/restore/destroy/list-deleted/add-line) use
# the AdminUser extractor → testuser ({{access_token}}) gets 403, admin
# ({{admin_token}}) gets 200/201. Reads (get-by-id, get-lines, by-product) use
# AuthUser → any authenticated user. No auth → 401. run-all.sh SKIPS this
# file when admin_token is empty.
#
# ASSUMPTION: the tier2.manufacturing feature flag is ON in this env; the
# read-only 17_manufacturing.hurl fences the gated (404) path. This file
# exercises the un-gated admin path, so it requires the flag enabled (same
# convention as 77_shifts_write.hurl).
# ASSUMPTION: GET /products is seeded non-empty by seed_sample_data.sql
# (P001/P002/P003) so items[0].id and items[1].id resolve — the BOM needs a
# product_id and the BOM line needs a distinct component_product_id.
# ASSUMPTION: rust_decimal serializes Decimal fields as JSON strings
# (quantity, scrap_percentage) — matches the cari write-fence convention.
# ASSUMPTION: soft_delete returns 200 MessageResponse; restore returns 200
# BillOfMaterials; destroy returns 204. After soft-delete GET by-id 404s;
# after destroy GET by-id 404s.
# ASSUMPTION: create_bom injects tenant_id from admin_user (handler overrides
# the body value), so the body tenant_id is informational only.
# RE-RUN SAFETY: the BOM is hard-destroyed at step 12, which frees the
# NON-partial unique slot bills_of_materials(product_id, version, tenant_id)
# (003:723) — so re-running with version "WF-BOM-1.0" does NOT 409. The
# product parents are NOT created here (taken from GET /products), so there
# is no parent cleanup to perform.

# --- 0. FK capture: GET /products → product_id + component_product_id ---
GET {{base_url}}/api/v1/products
Authorization: Bearer {{admin_token}}
HTTP 200
[Captures]
product_id: jsonpath "$.items[0].id"
component_product_id: jsonpath "$.items[1].id"
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber
jsonpath "$.total" > 0

# --- 1. POST create BOM (admin) → 201; capture bom_id ---
POST {{base_url}}/api/v1/manufacturing/boms
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "product_id": {{product_id}},
  "version": "WF-BOM-1.0",
  "is_active": true,
  "is_primary": true,
  "valid_from": null,
  "valid_to": null
}
```
HTTP 201
[Captures]
bom_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.tenant_id" isNumber
jsonpath "$.product_id" == {{product_id}}
jsonpath "$.version" == "WF-BOM-1.0"
jsonpath "$.is_active" isBoolean
jsonpath "$.is_primary" isBoolean
jsonpath "$.created_at" exists

# --- 2. GET /boms/deleted (admin) — DECODE FENCE: bare Vec<BillOfMaterials> ---
GET {{base_url}}/api/v1/manufacturing/boms/deleted
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 3. GET /boms/{id} (admin) — DECODE FENCE: single-row BillOfMaterials ---
GET {{base_url}}/api/v1/manufacturing/boms/{{bom_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{bom_id}}
jsonpath "$.product_id" == {{product_id}}
jsonpath "$.version" == "WF-BOM-1.0"
jsonpath "$.is_active" isBoolean
jsonpath "$.is_primary" isBoolean
jsonpath "$.deleted_at" exists
jsonpath "$.created_at" exists

# --- 4. GET /boms/{bom_id}/lines (auth) — DECODE FENCE: bare Vec (empty before line added) ---
GET {{base_url}}/api/v1/manufacturing/boms/{{bom_id}}/lines
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 5. POST /boms/lines (admin) → 201; add a component line ---
POST {{base_url}}/api/v1/manufacturing/boms/lines
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "bom_id": {{bom_id}},
  "component_product_id": {{component_product_id}},
  "quantity": "5.00",
  "unit_id": null,
  "scrap_percentage": "2.00",
  "is_optional": false,
  "notes": "hurl-seeded line"
}
```
HTTP 201
[Captures]
bom_line_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.bom_id" == {{bom_id}}
jsonpath "$.component_product_id" == {{component_product_id}}
jsonpath "$.quantity" exists
jsonpath "$.scrap_percentage" exists
jsonpath "$.is_optional" isBoolean

# --- 6. GET /boms/{bom_id}/lines (auth) — now non-empty, decode fence ---
GET {{base_url}}/api/v1/manufacturing/boms/{{bom_id}}/lines
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection
jsonpath "$[0].id" == {{bom_line_id}}
jsonpath "$[0].bom_id" == {{bom_id}}
jsonpath "$[0].component_product_id" == {{component_product_id}}

# --- 7. GET /boms/product/{product_id} (auth) — DECODE FENCE: bare Vec, self-seeded row present ---
GET {{base_url}}/api/v1/manufacturing/boms/product/{{product_id}}
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection
jsonpath "$[?(@.id=={{bom_id}})].version" contains "WF-BOM-1.0"

# --- 8. DELETE soft (admin) → 200 MessageResponse ---
DELETE {{base_url}}/api/v1/manufacturing/boms/{{bom_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.message" exists

# --- 9. GET by-id → 404 (soft-deleted hidden) ---
GET {{base_url}}/api/v1/manufacturing/boms/{{bom_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 10. PUT restore (admin) → 200 BillOfMaterials ---
PUT {{base_url}}/api/v1/manufacturing/boms/{{bom_id}}/restore
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{bom_id}}
jsonpath "$.version" == "WF-BOM-1.0"
jsonpath "$.deleted_at" exists

# --- 11. GET by-id → 200 (restored) ---
GET {{base_url}}/api/v1/manufacturing/boms/{{bom_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{bom_id}}

# --- 12. DELETE destroy (admin) → 204 ---
DELETE {{base_url}}/api/v1/manufacturing/boms/{{bom_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 13. GET by-id → 404 (permanently destroyed) ---
GET {{base_url}}/api/v1/manufacturing/boms/{{bom_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 14. RBAC: POST create with regular user → 403 ---
POST {{base_url}}/api/v1/manufacturing/boms
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "product_id": {{product_id}},
  "version": "WF-RBAC-403",
  "is_active": true,
  "is_primary": false,
  "valid_from": null,
  "valid_to": null
}
```
HTTP 403

# --- 15. RBAC: POST create with no auth → 401 ---
POST {{base_url}}/api/v1/manufacturing/boms
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "product_id": {{product_id}},
  "version": "WF-NOAUTH",
  "is_active": true,
  "is_primary": false,
  "valid_from": null,
  "valid_to": null
}
```
HTTP 401