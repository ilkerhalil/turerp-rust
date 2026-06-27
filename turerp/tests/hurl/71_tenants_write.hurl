# Tier 2 — Tenants WRITE-FENCE.
#
# Exercises POST /tenants, GET list, GET /{id}, PUT /{id}, DELETE /{id}
# (soft), /{id}/restore, /{id}/destroy. The read-only 08_tenants.hurl fence
# only covers list + by-id. Requires {{admin_token}} (run-all.sh skips
# *_write.hurl when admin is unavailable).
#
# Decode fences: step 2 (paginated Tenant list Row decode), step 3
# (single-row Tenant decode incl. supported_currencies array + db_name).
#
# ASSUMPTION: db_name is generated server-side from subdomain
# (src/domain/tenant/model.rs::generate_db_name) and is NOT required in the
# create body. CreateTenant has no db_name field.
# ASSUMPTION: suite runs against a freshly reset DB. The static subdomain
# `wftest` is globally unique; a second run against the same DB will 409 at
# step 1. Re-run safety relies on DB reset between full-suite runs.
# ASSUMPTION: get_tenant filters soft-deleted rows so step 6 / 10 return 404
# after soft-delete / destroy.

# --- 1. POST create (admin) → 201; capture id ---
POST {{base_url}}/api/v1/tenants
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "name": "WF Tenant",
  "subdomain": "wftest",
  "base_currency": "TRY",
  "supported_currencies": ["TRY", "USD"]
}
```
HTTP 201
[Captures]
created_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.subdomain" == "wftest"
jsonpath "$.base_currency" == "TRY"
jsonpath "$.db_name" exists
jsonpath "$.is_active" == true

# --- 2. GET list (admin) — DECODE FENCE: paginated envelope ---
GET {{base_url}}/api/v1/tenants
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber
jsonpath "$.total" > 0

# --- 3. GET by-id (admin) — DECODE FENCE: single-row Tenant decode ---
GET {{base_url}}/api/v1/tenants/{{created_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{created_id}}
jsonpath "$.name" isString
jsonpath "$.subdomain" == "wftest"
jsonpath "$.db_name" isString
jsonpath "$.base_currency" == "TRY"
jsonpath "$.supported_currencies" isCollection
jsonpath "$.is_active" isBoolean

# --- 4. PUT update (admin) → 200; assert changed name ---
PUT {{base_url}}/api/v1/tenants/{{created_id}}
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{ "name": "WF Tenant Updated" }
```
HTTP 200
[Asserts]
jsonpath "$.name" == "WF Tenant Updated"
jsonpath "$.subdomain" == "wftest"

# --- 5. DELETE soft (admin) → 200 MessageResponse ---
DELETE {{base_url}}/api/v1/tenants/{{created_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.message" exists

# --- 6. GET by-id → 404 (soft-deleted hidden) ---
GET {{base_url}}/api/v1/tenants/{{created_id}}
Authorization: Bearer {{admin_token}}
HTTP 404

# --- 7. POST restore (admin) → 200 ---
POST {{base_url}}/api/v1/tenants/{{created_id}}/restore
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{created_id}}
jsonpath "$.is_active" isBoolean

# --- 8. GET by-id → 200 (restored) ---
GET {{base_url}}/api/v1/tenants/{{created_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{created_id}}

# --- 9. DELETE destroy (admin) → 200 MessageResponse ---
DELETE {{base_url}}/api/v1/tenants/{{created_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.message" exists

# --- 10. GET by-id → 404 (permanently destroyed) ---
GET {{base_url}}/api/v1/tenants/{{created_id}}
Authorization: Bearer {{admin_token}}
HTTP 404

# --- 11. RBAC: POST create with regular user → 403 ---
POST {{base_url}}/api/v1/tenants
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{ "name": "WF RBAC", "subdomain": "wfrbac", "base_currency": "TRY" }
```
HTTP 403

# --- 12. No auth POST → 401 ---
POST {{base_url}}/api/v1/tenants
Content-Type: application/json
```
{ "name": "WF NoAuth", "subdomain": "wfnoauth", "base_currency": "TRY" }
```
HTTP 401