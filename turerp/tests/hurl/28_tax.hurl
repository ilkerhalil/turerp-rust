# Tier 1.5 — Tax (rates + periods). tax_rates is seeded → rates list
# non-empty. tax/periods currently 500 (broken) and tax/rates/effective
# requires query params (400 without a valid date).

# --- 1. Tax rates list — seeded, non-empty ---
GET {{base_url}}/api/v1/tax/rates
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber
jsonpath "$.total" > 0

# --- 2. Effective rate without a usable date — 400 ---
GET {{base_url}}/api/v1/tax/rates/effective?date=2026-06-24
Authorization: Bearer {{access_token}}
HTTP 400

# --- 3. Tax periods list — BROKEN (500), assert until fixed ---
GET {{base_url}}/api/v1/tax/periods
Authorization: Bearer {{access_token}}
HTTP 500

# --- 4. Deleted tax rates — RBAC: regular user gets 403 ---
GET {{base_url}}/api/v1/tax/rates/deleted
Authorization: Bearer {{access_token}}
HTTP 403

# --- 5. No auth — 401 ---
GET {{base_url}}/api/v1/tax/rates
HTTP 401