# Tier 1.5 — Companies (inter-company profile). Seeded (companies table),
# list MUST be non-empty.

# --- 1. Companies list — seeded, non-empty ---
GET {{base_url}}/api/v1/companies
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber
jsonpath "$.total" > 0

# --- 2. Non-existent company — 404 ---
GET {{base_url}}/api/v1/companies/999999
Authorization: Bearer {{access_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 3. Consolidated report — 200 (cross-company aggregate) ---
GET {{base_url}}/api/v1/companies/consolidated-report
Authorization: Bearer {{access_token}}
HTTP 200

# --- 4. No auth — 401 ---
GET {{base_url}}/api/v1/companies
HTTP 401