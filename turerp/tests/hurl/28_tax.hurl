# Tier 1.5 — Tax (rates + periods). tax_rates is seeded → rates list
# non-empty. tax/periods now 200 (migration 040 added the missing
# deleted_at/deleted_by columns the soft-delete SELECT expects). tax/periods
# is NOT seeded, so the list is empty — but the bug was a missing COLUMN
# (errors on an empty table too), so a 200 + envelope proves the query runs.
# tax/rates/effective requires query params (400 without a valid date).

# --- 1. Tax rates list — seeded, non-empty ---
GET {{base_url}}/api/v1/tax/rates
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber
jsonpath "$.total" > 0
[Captures]
rate_id: jsonpath "$.items[0].id"

# --- 1b. Tax rate by id — 200. Locks in the int4 total_count decode fix
#      (find_by_id used `0 as total_count` = int4, but TaxRateRow.total_count
#      is Option<i64> = int8 → type-oid mismatch → 500; now `0::bigint`).
#      The list path (COUNT(*) OVER, int8) never exercised this, so the
#      get-by-id assert is what fences the decode fix. ---
GET {{base_url}}/api/v1/tax/rates/{{rate_id}}
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.tax_type" exists

# --- 2. Effective rate without a usable date — 400 ---
GET {{base_url}}/api/v1/tax/rates/effective?date=2026-06-24
Authorization: Bearer {{access_token}}
HTTP 400

# --- 3. Tax periods list — 200 (was 500: missing deleted_at/deleted_by cols,
#     fixed by migration 040). Unseeded → empty is fine; 200 proves the query
#     runs. ---
GET {{base_url}}/api/v1/tax/periods
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber

# --- 4. Deleted tax rates — RBAC: regular user gets 403 ---
GET {{base_url}}/api/v1/tax/rates/deleted
Authorization: Bearer {{access_token}}
HTTP 403

# --- 5. No auth — 401 ---
GET {{base_url}}/api/v1/tax/rates
HTTP 401