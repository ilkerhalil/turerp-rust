# Tier 1.5 — Invoices. Cross-module: invoices integrate with cari and
# accounting. List is the smoke test; outstanding/overdue are common
# queries that exercise different filter paths.


# --- 1. All invoices ---
GET {{base_url}}/api/v1/invoices
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber

# --- 2. Outstanding invoices (filter by status) ---
GET {{base_url}}/api/v1/invoices/outstanding
Authorization: Bearer {{access_token}}
HTTP 200

# --- 3. Invoices with bogus status filter — must not 500 (404 if route
#        not defined for status path, 400 if validated, both acceptable) ---
GET {{base_url}}/api/v1/invoices/status/bogus_status_xyz
Authorization: Bearer {{access_token}}
HTTP 404
