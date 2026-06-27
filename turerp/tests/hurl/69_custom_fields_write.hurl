# 69_custom_fields_write — WRITE-FENCE for the Custom Field CRUD lifecycle.
#
# Covers: POST create → GET list (decode fence) → GET by-id (decode fence)
#         → PUT update → DELETE soft (200 MessageResponse) → GET by-id 404
#         → POST /restore (200 MessageResponse) → GET by-id 200
#         → DELETE /destroy (204) → GET by-id 404.
#
# Why: 35_custom_fields.hurl only asserts the RBAC privilege-escalation fence
# (testuser 403, no-auth 401) — the create/update/soft-delete/restore/destroy
# decode paths are never exercised, and deleted_by audit trail is never
# checked. This fence self-seeds a throwaway custom field definition and
# walks every lifecycle transition.
#
# Route shape notes (custom_fields differs from the other modules):
#   - list is admin-only (testuser 403) and returns a bare Vec, not paginated
#   - soft delete:  DELETE /custom-fields/{id}        → 200 MessageResponse
#   - restore:       POST   /custom-fields/{id}/restore → 200 MessageResponse
#   - destroy:       DELETE /custom-fields/{id}/destroy → 204 NoContent
# (restore is POST, not PUT — see src/api/v1/custom_fields.rs.)
#
# RBAC: every route (incl. list) uses AdminUser → testuser ({{access_token}})
# gets 403, admin ({{admin_token}}) gets 200/201. No auth → 401.
# run-all.sh SKIPS this file when admin_token is empty.
#
# ASSUMPTION: field_name "hurl_test_field" is unique per (tenant, module).
# If a prior run aborted before destroy, re-run will 409 — drop the leftover
# row or pick a different field_name. module must be a valid CustomFieldModule
# (cari|invoice|stock|sales|purchase|hr|accounting|project|manufacturing|crm|
# asset|product); field_type must be a valid CustomFieldType
# (string|number|date|boolean|select). The handler overwrites tenant_id from
# the admin token, so the body's tenant_id is ignored.

# --- 1. Create custom field as admin — 201 ---
POST {{base_url}}/api/v1/custom-fields
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "module": "product",
  "field_name": "hurl_test_field",
  "field_label": "Hurl Test Field",
  "field_type": "string",
  "required": false,
  "options": [],
  "sort_order": 0,
  "tenant_id": 1
}
```
HTTP 201
[Captures]
created_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.module" == "product"
jsonpath "$.field_name" == "hurl_test_field"
jsonpath "$.field_label" == "Hurl Test Field"
jsonpath "$.field_type" == "string"
jsonpath "$.required" isBoolean
jsonpath "$.is_active" isBoolean
jsonpath "$.tenant_id" isNumber
jsonpath "$.created_at" exists

# --- 2. GET list — decode fence: top-level Vec + self-seeded item present ---
GET {{base_url}}/api/v1/custom-fields
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection
jsonpath "$[?(@.id=={{created_id}})].field_name" contains "hurl_test_field"

# --- 3. GET by-id — decode fence on single-row path ---
GET {{base_url}}/api/v1/custom-fields/{{created_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{created_id}}
jsonpath "$.module" == "product"
jsonpath "$.field_name" == "hurl_test_field"
jsonpath "$.field_label" == "Hurl Test Field"
jsonpath "$.field_type" == "string"
jsonpath "$.required" isBoolean
jsonpath "$.is_active" isBoolean

# --- 4. PUT update — 200, changed fields reflect new values ---
PUT {{base_url}}/api/v1/custom-fields/{{created_id}}
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "field_label": "Hurl Test Field (renamed)",
  "required": true,
  "options": ["a", "b"],
  "sort_order": 5,
  "is_active": false
}
```
HTTP 200
[Asserts]
jsonpath "$.id" == {{created_id}}
jsonpath "$.field_label" == "Hurl Test Field (renamed)"
jsonpath "$.required" == true
jsonpath "$.is_active" == false

# --- 5. DELETE soft — 200 (MessageResponse) ---
DELETE {{base_url}}/api/v1/custom-fields/{{created_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.message" exists

# --- 6. GET by-id after soft-delete — 404 (soft-deleted filtered out) ---
GET {{base_url}}/api/v1/custom-fields/{{created_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 7. POST /restore — 200 (MessageResponse) ---
POST {{base_url}}/api/v1/custom-fields/{{created_id}}/restore
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.message" exists

# --- 8. GET by-id after restore — 200 ---
GET {{base_url}}/api/v1/custom-fields/{{created_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{created_id}}
jsonpath "$.field_label" == "Hurl Test Field (renamed)"
jsonpath "$.required" == true

# --- 9. DELETE /destroy — 204 (hard delete) ---
DELETE {{base_url}}/api/v1/custom-fields/{{created_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 10. GET by-id after destroy — 404 ---
GET {{base_url}}/api/v1/custom-fields/{{created_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 11. RBAC: POST create as regular user — 403 ---
POST {{base_url}}/api/v1/custom-fields
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "module": "product",
  "field_name": "forbidden_field",
  "field_label": "should be forbidden",
  "field_type": "string",
  "required": false,
  "options": [],
  "sort_order": 0,
  "tenant_id": 1
}
```
HTTP 403

# --- 12. RBAC: POST create with no auth — 401 ---
POST {{base_url}}/api/v1/custom-fields
Content-Type: application/json
```
{
  "module": "product",
  "field_name": "noauth_field",
  "field_label": "should be unauth",
  "field_type": "string",
  "required": false,
  "options": [],
  "sort_order": 0,
  "tenant_id": 1
}
```
HTTP 401