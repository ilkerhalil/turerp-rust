# Tier 2 — Settings WRITE-FENCE.
#
# Exercises POST /settings, GET (list, uses "data" key not "items"),
# GET /{key} (by-key lookup), PUT /{id} (mutation by id), DELETE /{id}/soft,
# /{id}/restore, /{id}/destroy, plus /bulk and /seed. The read-only
# 16_settings.hurl fence only covers list + by-key. Requires
# {{admin_token}} (run-all.sh skips *_write.hurl when admin unavailable).
#
# Key shape note: GET uses {key}; mutations (PUT/DELETE/soft/restore/destroy)
# use {id}. The create handler overrides tenant_id with the admin's tenant.
#
# Decode fences: step 2 (list "data" envelope decode), step 3 (by-key
# SettingResponse decode incl. masked-sensitive handling).
#
# ASSUMPTION: suite runs against a freshly reset DB. The static key
# `wf.test.setting` is unique per tenant; a second run against the same DB
# will 409 at step 1. Re-run safety relies on DB reset between full-suite
# runs.
# ASSUMPTION: get_by_key filters soft-deleted rows so step 6 / 10 return 404
# after soft-delete / destroy.
# ASSUMPTION: /seed is idempotent (returns 201 with created count, 0 on
# re-seed). If the implementation 409s on existing default keys this step
# will fail — flagged for review.

# --- 1. POST create (admin) → 201; capture id ---
POST {{base_url}}/api/v1/settings
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "key": "wf.test.setting",
  "value": "hello",
  "default_value": "default",
  "data_type": "string",
  "group": "general",
  "description": "hurl write-fence setting",
  "is_sensitive": false,
  "is_editable": true
}
```
HTTP 201
[Captures]
created_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.key" == "wf.test.setting"
jsonpath "$.value" == "hello"
jsonpath "$.data_type" == "string"
jsonpath "$.group" == "general"

# --- 2. GET list (admin) — DECODE FENCE: "data" envelope ---
GET {{base_url}}/api/v1/settings
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.data" isCollection
jsonpath "$.total" isNumber
jsonpath "$.total" > 0

# --- 3. GET by-key (admin) — DECODE FENCE: SettingResponse single-row ---
GET {{base_url}}/api/v1/settings/wf.test.setting
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{created_id}}
jsonpath "$.key" == "wf.test.setting"
jsonpath "$.value" == "hello"
jsonpath "$.data_type" isString
jsonpath "$.group" isString
jsonpath "$.is_sensitive" isBoolean
jsonpath "$.is_editable" isBoolean

# --- 4. PUT update by-id (admin) → 200; assert changed value + description ---
PUT {{base_url}}/api/v1/settings/{{created_id}}
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{ "value": "updated", "description": "updated by write-fence", "is_editable": false }
```
HTTP 200
[Asserts]
jsonpath "$.value" == "updated"
jsonpath "$.description" == "updated by write-fence"
jsonpath "$.is_editable" == false

# --- 5. POST /bulk (admin) → 200; bulk-update by key, assert updated count ---
POST {{base_url}}/api/v1/settings/bulk
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{ "updates": [ { "key": "wf.test.setting", "value": "bulk-set" } ] }
```
HTTP 200
[Asserts]
jsonpath "$.updated" isNumber
jsonpath "$.updated" > 0

# --- 6. GET by-key → assert bulk-applied value ---
GET {{base_url}}/api/v1/settings/wf.test.setting
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.value" == "bulk-set"

# --- 7. DELETE soft by-id (admin) → 204 ---
DELETE {{base_url}}/api/v1/settings/{{created_id}}/soft
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 8. GET by-key → 404 (soft-deleted hidden) ---
GET {{base_url}}/api/v1/settings/wf.test.setting
Authorization: Bearer {{admin_token}}
HTTP 404

# --- 9. POST restore by-id (admin) → 204 ---
POST {{base_url}}/api/v1/settings/{{created_id}}/restore
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 10. GET by-key → 200 (restored) ---
GET {{base_url}}/api/v1/settings/wf.test.setting
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{created_id}}

# --- 11. DELETE destroy by-id (admin) → 204 ---
DELETE {{base_url}}/api/v1/settings/{{created_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 12. GET by-key → 404 (permanently destroyed) ---
GET {{base_url}}/api/v1/settings/wf.test.setting
Authorization: Bearer {{admin_token}}
HTTP 404

# --- 13. POST /seed (admin) → 201; seed default settings for tenant ---
# ASSUMPTION: idempotent — returns 201 with created count (0 on re-seed).
POST {{base_url}}/api/v1/settings/seed
Authorization: Bearer {{admin_token}}
HTTP 201
[Asserts]
jsonpath "$.created" isNumber

# --- 14. RBAC: POST create with regular user → 403 ---
POST {{base_url}}/api/v1/settings
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "key": "wf.rbac.403",
  "value": "x",
  "data_type": "string",
  "group": "general",
  "description": "should be rejected"
}
```
HTTP 403

# --- 15. No auth POST → 401 ---
POST {{base_url}}/api/v1/settings
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "key": "wf.noauth",
  "value": "x",
  "data_type": "string",
  "group": "general",
  "description": "no auth"
}
```
HTTP 401