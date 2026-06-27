# 66_currencies_write — WRITE-FENCE for the Currency CRUD lifecycle.
#
# Currency is keyed by {code} (not {id}) on every mutation route, and its
# soft-delete/restore/destroy routes are suffixed /soft, /restore, /destroy
# (not the bare DELETE). Covers:
#   POST create → GET list (decode fence) → GET by-code (decode fence)
#   → PUT update → DELETE /{code}/soft → GET by-code 404
#   → POST /{code}/restore → GET by-code 200
#   → DELETE /{code}/destroy → GET by-code 404.
#
# Why: 06_currencies.hurl only hits the list + the seeded USD by-code, so
# Row decode bugs in the create/update/soft-delete/restore/destroy paths
# stay hidden. This fence self-seeds a throwaway currency (code TST) and
# exercises every lifecycle transition + the deleted_by audit trail.
#
# RBAC: writes use AdminUser → testuser ({{access_token}}) gets 403, admin
# ({{admin_token}}) gets 200/201. No auth → 401.
# run-all.sh SKIPS this file when admin_token is empty.
#
# ASSUMPTION: currency code "TST" is not already in use under tenant 1. If a
# prior run aborted before destroy, re-run will 409 on create — drop the
# leftover row (DELETE FROM currencies WHERE code='TST' AND tenant_id=1) or
# pick a different code. The handler uppercases the code before insert, so
# "TST" is sent uppercase.

# --- 1. Create currency as admin — 201 ---
POST {{base_url}}/api/v1/currencies
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "code": "TST",
  "name": "Hurl Test Currency",
  "symbol": "₮",
  "decimal_places": 2,
  "is_active": true,
  "is_base": false
}
```
HTTP 201
[Captures]
created_code: jsonpath "$.code"
[Asserts]
jsonpath "$.code" == "TST"
jsonpath "$.name" == "Hurl Test Currency"
jsonpath "$.symbol" exists
jsonpath "$.decimal_places" isNumber
jsonpath "$.is_active" isBoolean
jsonpath "$.is_base" isBoolean
jsonpath "$.tenant_id" isNumber
jsonpath "$.created_at" exists

# --- 2. GET list — decode fence: items decode + self-seeded total > 0 ---
GET {{base_url}}/api/v1/currencies
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber
jsonpath "$.total" > 0
jsonpath "$.items[?(@.code=='TST')].name" contains "Hurl Test Currency"

# --- 3. GET by-code — decode fence on single-row path ---
GET {{base_url}}/api/v1/currencies/TST
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.code" == "TST"
jsonpath "$.name" == "Hurl Test Currency"
jsonpath "$.symbol" exists
jsonpath "$.decimal_places" == 2
jsonpath "$.is_active" isBoolean
jsonpath "$.is_base" isBoolean

# --- 4. PUT update by-code — 200, changed field reflects new value ---
PUT {{base_url}}/api/v1/currencies/TST
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "name": "Hurl Test Currency (renamed)",
  "is_active": false
}
```
HTTP 200
[Asserts]
jsonpath "$.code" == "TST"
jsonpath "$.name" == "Hurl Test Currency (renamed)"
jsonpath "$.is_active" == false

# --- 5. DELETE /{code}/soft — 204 ---
DELETE {{base_url}}/api/v1/currencies/TST/soft
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 6. GET by-code after soft-delete — 404 (soft-deleted filtered out) ---
GET {{base_url}}/api/v1/currencies/TST
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 7. POST /{code}/restore — 204 ---
POST {{base_url}}/api/v1/currencies/TST/restore
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 8. GET by-code after restore — 200 ---
GET {{base_url}}/api/v1/currencies/TST
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.code" == "TST"
jsonpath "$.name" == "Hurl Test Currency (renamed)"

# --- 9. DELETE /{code}/destroy — 204 (hard delete) ---
DELETE {{base_url}}/api/v1/currencies/TST/destroy
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 10. GET by-code after destroy — 404 ---
GET {{base_url}}/api/v1/currencies/TST
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 11. RBAC: POST create as regular user — 403 ---
POST {{base_url}}/api/v1/currencies
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "code": "TST",
  "name": "should be forbidden",
  "symbol": "₮",
  "decimal_places": 2,
  "is_active": true,
  "is_base": false
}
```
HTTP 403

# --- 12. RBAC: POST create with no auth — 401 ---
POST {{base_url}}/api/v1/currencies
Content-Type: application/json
```
{
  "code": "TST",
  "name": "should be unauth",
  "symbol": "₮",
  "decimal_places": 2,
  "is_active": true,
  "is_base": false
}
```
HTTP 401