# Tier 2 — Chart of Accounts WRITE-FENCE.
#
# Exercises POST /chart-of-accounts, GET list, GET /{code}, PUT /{code},
# DELETE /{code} (soft), POST /{code}/restore, DELETE /{code}/destroy,
# plus GET /{code}/children and POST /{code}/recalculate. Uses {code} (not
# {id}) for all mutations and lookups. The read-only 24_chart_of_accounts.hurl
# fence only covers list / tree / trial-balance / 404. Requires
# {{admin_token}} (run-all.sh skips *_write.hurl when admin unavailable).
#
# Decode fences: step 2 (paginated ChartAccountResponse list decode incl.
# Decimal balance), step 3 (single-row decode incl. group / account_type
# enums).
#
# ASSUMPTION: suite runs against a freshly reset DB. The static account code
# `WF999` is unique per tenant and chosen not to collide with seeded chart
# accounts (100/101/...); a second run against the same DB will 409 at
# step 1. Re-run safety relies on DB reset between full-suite runs.
# ASSUMPTION: get_account_by_code filters soft-deleted rows so step 6 / 10
# return 404 after soft-delete / destroy.
# ASSUMPTION: account_type uses PascalCase variants
# (Asset/Liability/Equity/Revenue/Expense) and group uses the Turkish
# PascalCase names (DonenVarliklar/.../GiderTablosu) per
# src/domain/chart_of_accounts/model.rs Display/FromStr impls.

# --- 1. POST create (admin) → 201; capture id ---
POST {{base_url}}/api/v1/chart-of-accounts
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "code": "WF999",
  "name": "WF Write-Fence Account",
  "group": "GelirTablosu",
  "parent_code": null,
  "account_type": "Revenue",
  "allow_posting": true
}
```
HTTP 201
[Captures]
created_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.code" == "WF999"
jsonpath "$.group" == "GelirTablosu"
jsonpath "$.account_type" == "Revenue"
jsonpath "$.allow_posting" == true

# --- 2. GET list (admin) — DECODE FENCE: paginated ChartAccountResponse envelope ---
GET {{base_url}}/api/v1/chart-of-accounts
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber
jsonpath "$.total" > 0

# --- 3. GET by-code (admin) — DECODE FENCE: single-row ChartAccountResponse ---
GET {{base_url}}/api/v1/chart-of-accounts/WF999
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{created_id}}
jsonpath "$.code" == "WF999"
jsonpath "$.name" isString
jsonpath "$.group" == "GelirTablosu"
jsonpath "$.account_type" == "Revenue"
jsonpath "$.balance" exists
jsonpath "$.is_active" isBoolean
jsonpath "$.allow_posting" isBoolean

# --- 4. PUT update by-code (admin) → 200; assert changed name + is_active ---
PUT {{base_url}}/api/v1/chart-of-accounts/WF999
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{ "name": "WF Updated", "is_active": false }
```
HTTP 200
[Asserts]
jsonpath "$.name" == "WF Updated"
jsonpath "$.is_active" == false

# --- 5. GET /{code}/children (auth) → 200; bare array of children (empty ok) ---
GET {{base_url}}/api/v1/chart-of-accounts/WF999/children
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 6. DELETE soft by-code (admin) → 200 MessageResponse ---
DELETE {{base_url}}/api/v1/chart-of-accounts/WF999
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.message" exists

# --- 7. GET by-code → 404 (soft-deleted hidden) ---
GET {{base_url}}/api/v1/chart-of-accounts/WF999
Authorization: Bearer {{admin_token}}
HTTP 404

# --- 8. POST restore by-code (admin) → 200 MessageResponse ---
POST {{base_url}}/api/v1/chart-of-accounts/WF999/restore
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.message" exists

# --- 9. GET by-code → 200 (restored) ---
GET {{base_url}}/api/v1/chart-of-accounts/WF999
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.code" == "WF999"

# --- 10. POST /{code}/recalculate (admin) → 200; balance recalc returns account ---
POST {{base_url}}/api/v1/chart-of-accounts/WF999/recalculate
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.code" == "WF999"
jsonpath "$.balance" exists

# --- 11. DELETE destroy by-code (admin) → 204 ---
DELETE {{base_url}}/api/v1/chart-of-accounts/WF999/destroy
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 12. GET by-code → 404 (permanently destroyed) ---
GET {{base_url}}/api/v1/chart-of-accounts/WF999
Authorization: Bearer {{admin_token}}
HTTP 404

# --- 13. RBAC: POST create with regular user → 403 ---
POST {{base_url}}/api/v1/chart-of-accounts
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "code": "WF999RBAC",
  "name": "RBAC reject",
  "group": "GelirTablosu",
  "parent_code": null,
  "account_type": "Revenue",
  "allow_posting": true
}
```
HTTP 403

# --- 14. No auth POST → 401 ---
POST {{base_url}}/api/v1/chart-of-accounts
Content-Type: application/json
```
{
  "code": "WF999NOAUTH",
  "name": "no auth",
  "group": "GelirTablosu",
  "parent_code": null,
  "account_type": "Revenue",
  "allow_posting": true
}
```
HTTP 401