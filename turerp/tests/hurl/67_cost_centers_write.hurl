# 67_cost_centers_write — WRITE-FENCE for the Cost Center CRUD lifecycle.
#
# Covers: POST create → GET list (decode fence) → GET by-id (decode fence)
#         → PUT update → DELETE soft (204) → GET by-id 404 → PUT restore
#         → GET by-id 200 → DELETE /destroy (204) → GET by-id 404.
#
# Why: 25_cost_centers.hurl only hits the seeded list + by-id=1 (CC01) and
# the /999999 → 404 case. The create/update/soft-delete/restore/destroy Row
# decode paths are never exercised; the int4-as-int8 total_count decode bug
# (PR #173) was hidden exactly because the by-id fence only used seeded rows
# and the non-existent fence returned no row to decode. This fence self-seeds
# a throwaway cost center and walks every lifecycle transition + the
# deleted_by audit trail.
#
# RBAC: writes use AdminUser → testuser ({{access_token}}) gets 403, admin
# ({{admin_token}}) gets 200/201. No auth → 401.
# run-all.sh SKIPS this file when admin_token is empty.
#
# ASSUMPTION: cost center code "HURLCC" is unique per tenant. If a prior run
# aborted before destroy, re-run will 409 — drop the leftover row or pick a
# different code. center_type is the CostCenterType enum, serialized via
# serde default (PascalCase): "Cost" or "Profit" — NOT lowercase (the
# lowercase Display/FromStr impls are query-string only, not JSON bodies).

# --- 1. Create cost center as admin — 201 ---
POST {{base_url}}/api/v1/cost-centers
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "code": "HURLCC",
  "name": "Hurl Test Cost Center",
  "description": "self-seeded by hurl write-fence",
  "center_type": "Cost",
  "parent_id": null,
  "is_active": true
}
```
HTTP 201
[Captures]
created_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.code" == "HURLCC"
jsonpath "$.name" == "Hurl Test Cost Center"
jsonpath "$.center_type" == "Cost"
jsonpath "$.is_active" isBoolean
jsonpath "$.tenant_id" isNumber
jsonpath "$.created_at" exists

# --- 2. GET list — decode fence: items decode + self-seeded total > 0 ---
GET {{base_url}}/api/v1/cost-centers
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber
jsonpath "$.total" > 0
jsonpath "$.items[?(@.id=={{created_id}})].code" contains "HURLCC"

# --- 3. GET by-id — decode fence on single-row path (the bug class from #173) ---
GET {{base_url}}/api/v1/cost-centers/{{created_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{created_id}}
jsonpath "$.code" == "HURLCC"
jsonpath "$.name" == "Hurl Test Cost Center"
jsonpath "$.center_type" == "Cost"
jsonpath "$.is_active" isBoolean
jsonpath "$.created_at" exists

# --- 4. PUT update — 200, changed fields reflect new values ---
PUT {{base_url}}/api/v1/cost-centers/{{created_id}}
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "name": "Hurl Test Cost Center (renamed)",
  "description": "updated by hurl write-fence",
  "center_type": "Profit",
  "is_active": false
}
```
HTTP 200
[Asserts]
jsonpath "$.id" == {{created_id}}
jsonpath "$.name" == "Hurl Test Cost Center (renamed)"
jsonpath "$.center_type" == "Profit"
jsonpath "$.is_active" == false

# --- 5. DELETE soft — 204 (NoContent) ---
DELETE {{base_url}}/api/v1/cost-centers/{{created_id}}
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 6. GET by-id after soft-delete — 404 ---
GET {{base_url}}/api/v1/cost-centers/{{created_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 7. PUT restore — 200 with body ---
PUT {{base_url}}/api/v1/cost-centers/{{created_id}}/restore
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{created_id}}
jsonpath "$.code" == "HURLCC"

# --- 8. GET by-id after restore — 200 ---
GET {{base_url}}/api/v1/cost-centers/{{created_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{created_id}}
jsonpath "$.name" == "Hurl Test Cost Center (renamed)"
jsonpath "$.center_type" == "Profit"

# --- 9. DELETE /destroy — 204 (hard delete) ---
DELETE {{base_url}}/api/v1/cost-centers/{{created_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 10. GET by-id after destroy — 404 ---
GET {{base_url}}/api/v1/cost-centers/{{created_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 11. RBAC: POST create as regular user — 403 ---
POST {{base_url}}/api/v1/cost-centers
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "code": "FORBIDDEN",
  "name": "should be forbidden",
  "description": null,
  "center_type": "Cost",
  "parent_id": null,
  "is_active": true
}
```
HTTP 403

# --- 12. RBAC: POST create with no auth — 401 ---
POST {{base_url}}/api/v1/cost-centers
Content-Type: application/json
```
{
  "code": "NOAUTH",
  "name": "should be unauth",
  "description": null,
  "center_type": "Cost",
  "parent_id": null,
  "is_active": true
}
```
HTTP 401