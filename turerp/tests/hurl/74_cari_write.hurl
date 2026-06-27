# Tier 2 — Cari (Customer/Vendor) WRITE-FENCE.
#
# Exercises POST /caris, GET list, GET /{id}, PUT /{id}, DELETE /{id}
# (soft), PUT /{id}/restore, DELETE /{id}/destroy. The read-only
# 13_crm_customers.hurl fence only covers list + by-type + 404. Requires
# {{admin_token}} (run-all.sh skips *_write.hurl when admin unavailable).
#
# Decode fences: step 2 (paginated CariResponse list decode incl. Decimal
# credit_limit / current_balance), step 3 (single-row decode incl. enum
# cari_type / status).
#
# ASSUMPTION: suite runs against a freshly reset DB. The static cari code
# `WF-CARI-001` is unique per tenant; a second run against the same DB will
# 409 at step 1. Re-run safety relies on DB reset between full-suite runs.
# ASSUMPTION: company_id=1 exists for tenant 1 (seeded by seed_sample_data).
# If company 1 is absent the create will FK-fail — flagged for review.
# ASSUMPTION: created_by={{user_id}} is a valid user row (testuser, tenant
# 1) so any FK on created_by holds.
# ASSUMPTION: get_cari filters soft-deleted rows so step 6 / 10 return 404
# after soft-delete / destroy.
# ASSUMPTION: rust_decimal deserializes from a JSON string ("1000.00") for
# credit_limit. If the Decimal serde feature is configured to require a
# number, step 1 will 400 — flagged for review.

# --- 1. POST create (admin) → 201; capture id ---
POST {{base_url}}/api/v1/caris
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "code": "WF-CARI-001",
  "name": "WF Cari Customer",
  "cari_type": "customer",
  "tax_number": "1234567890",
  "email": "wfcari@example.com",
  "phone": "5551234567",
  "credit_limit": "1000.00",
  "default_currency": "TRY",
  "tenant_id": {{tenant_id}},
  "company_id": 1,
  "created_by": {{user_id}}
}
```
HTTP 201
[Captures]
created_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.code" == "WF-CARI-001"
jsonpath "$.cari_type" == "customer"
jsonpath "$.status" == "active"

# --- 2. GET list (admin) — DECODE FENCE: paginated CariResponse envelope ---
GET {{base_url}}/api/v1/caris
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber
jsonpath "$.total" > 0

# --- 3. GET by-id (admin) — DECODE FENCE: single-row CariResponse ---
GET {{base_url}}/api/v1/caris/{{created_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{created_id}}
jsonpath "$.code" == "WF-CARI-001"
jsonpath "$.name" isString
jsonpath "$.cari_type" == "customer"
jsonpath "$.credit_limit" exists
jsonpath "$.current_balance" exists
jsonpath "$.default_currency" == "TRY"
jsonpath "$.status" == "active"
jsonpath "$.tenant_id" isNumber

# --- 4. PUT update by-id (admin) → 200; assert changed name + status ---
PUT {{base_url}}/api/v1/caris/{{created_id}}
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{ "name": "WF Cari Updated", "status": "passive" }
```
HTTP 200
[Asserts]
jsonpath "$.name" == "WF Cari Updated"
jsonpath "$.status" == "passive"

# --- 5. DELETE soft (admin) → 200 MessageResponse ---
DELETE {{base_url}}/api/v1/caris/{{created_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.message" exists

# --- 6. GET by-id → 404 (soft-deleted hidden) ---
GET {{base_url}}/api/v1/caris/{{created_id}}
Authorization: Bearer {{admin_token}}
HTTP 404

# --- 7. PUT restore (admin) → 200 CariResponse ---
PUT {{base_url}}/api/v1/caris/{{created_id}}/restore
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{created_id}}
jsonpath "$.code" == "WF-CARI-001"

# --- 8. GET by-id → 200 (restored) ---
GET {{base_url}}/api/v1/caris/{{created_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{created_id}}

# --- 9. DELETE destroy (admin) → 204 ---
DELETE {{base_url}}/api/v1/caris/{{created_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 10. GET by-id → 404 (permanently destroyed) ---
GET {{base_url}}/api/v1/caris/{{created_id}}
Authorization: Bearer {{admin_token}}
HTTP 404

# --- 11. RBAC: POST create with regular user → 403 ---
POST {{base_url}}/api/v1/caris
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "code": "WF-RBAC-403",
  "name": "RBAC reject",
  "cari_type": "customer",
  "tenant_id": {{tenant_id}},
  "company_id": 1,
  "created_by": {{user_id}}
}
```
HTTP 403

# --- 12. No auth POST → 401 ---
POST {{base_url}}/api/v1/caris
Content-Type: application/json
```
{
  "code": "WF-NOAUTH",
  "name": "no auth",
  "cari_type": "customer",
  "tenant_id": {{tenant_id}},
  "company_id": 1,
  "created_by": {{user_id}}
}
```
HTTP 401