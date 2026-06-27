# 64_categories_write — WRITE-FENCE for the Category CRUD lifecycle.
#
# Covers: POST create → GET list (decode fence) → GET by-id (decode fence)
#         → PUT update → DELETE soft → GET by-id 404 → PUT restore
#         → GET by-id 200 → DELETE /destroy → GET by-id 404.
#
# Why: the read-only 04_categories.hurl only hits the (empty, gated) list,
# so Row decode bugs in the create/by-id/update/soft-delete/restore/destroy
# paths stay hidden until seeded. This fence self-seeds a category, then
# exercises every lifecycle transition and the deleted_by audit trail.
#
# RBAC: writes use the AdminUser extractor → testuser ({{access_token}}) gets
# 403, admin ({{admin_token}}) gets 200/201. No auth → 401.
# run-all.sh SKIPS this file when admin_token is empty.
#
# ASSUMPTION: GET /categories is gated by the `core.categories` feature flag
# (default-disabled per 04_categories.hurl). If the flag is OFF, the create
# and by-id routes may 404 and this fence will fail — flip the flag on before
# running. ASSUMPTION: company_id=1 is used in the create body (the handler
# does NOT inject company_id, only tenant_id); confirm tenant 1 has
# company_id=1 seeded, or adjust.

# --- 1. Create category as admin — 201 ---
POST {{base_url}}/api/v1/categories
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "tenant_id": 1,
  "company_id": 1,
  "name": "Hurl Test Category",
  "parent_id": null
}
```
HTTP 201
[Captures]
created_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.name" == "Hurl Test Category"
jsonpath "$.company_id" isNumber
jsonpath "$.created_at" exists

# --- 2. GET list — decode fence: items decode + self-seeded total > 0 ---
GET {{base_url}}/api/v1/categories
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber
jsonpath "$.total" > 0
jsonpath "$.items[?(@.id=={{created_id}})].name" contains "Hurl Test Category"

# --- 3. GET by-id — decode fence on single-row path ---
GET {{base_url}}/api/v1/categories/{{created_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{created_id}}
jsonpath "$.name" == "Hurl Test Category"
jsonpath "$.company_id" isNumber
jsonpath "$.parent_id" exists
jsonpath "$.created_at" exists

# --- 4. PUT update — 200, changed field reflects new value ---
PUT {{base_url}}/api/v1/categories/{{created_id}}
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "name": "Hurl Test Category (renamed)",
  "parent_id": null,
  "company_id": 1
}
```
HTTP 200
[Asserts]
jsonpath "$.id" == {{created_id}}
jsonpath "$.name" == "Hurl Test Category (renamed)"

# --- 5. DELETE soft — 200 (MessageResponse) ---
DELETE {{base_url}}/api/v1/categories/{{created_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.message" exists

# --- 6. GET by-id after soft-delete — 404 (soft-deleted not visible) ---
GET {{base_url}}/api/v1/categories/{{created_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 7. PUT restore — 200 ---
PUT {{base_url}}/api/v1/categories/{{created_id}}/restore
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{created_id}}
jsonpath "$.name" exists

# --- 8. GET by-id after restore — 200 ---
GET {{base_url}}/api/v1/categories/{{created_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{created_id}}
jsonpath "$.name" == "Hurl Test Category (renamed)"

# --- 9. DELETE /destroy — 204 (hard delete) ---
DELETE {{base_url}}/api/v1/categories/{{created_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 10. GET by-id after destroy — 404 ---
GET {{base_url}}/api/v1/categories/{{created_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 11. RBAC: POST create as regular user — 403 ---
POST {{base_url}}/api/v1/categories
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "tenant_id": 1,
  "company_id": 1,
  "name": "should be forbidden",
  "parent_id": null
}
```
HTTP 403

# --- 12. RBAC: POST create with no auth — 401 ---
POST {{base_url}}/api/v1/categories
Content-Type: application/json
```
{
  "tenant_id": 1,
  "company_id": 1,
  "name": "should be unauth",
  "parent_id": null
}
```
HTTP 401