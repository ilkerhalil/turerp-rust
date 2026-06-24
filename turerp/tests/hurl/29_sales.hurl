# Tier 2 — Sales (orders + quotations). orders + quotations lists work
# (empty until seeded). quotations list was 500 (broken — the quotations
# table was missing company_id, which migration 023 added to 10 sibling
# tables but skipped quotations; fixed by migration 041). All /deleted
# lists are admin-only (403 for a regular user).

# --- 1. Sales orders list — 200 (paginated envelope, may be empty) ---
GET {{base_url}}/api/v1/sales/orders
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber

# --- 2. Sales quotations list — 200 (was 500: quotations table missing
#     company_id, which 023 added to 10 sibling tables but skipped; fixed
#     by migration 041). Unseeded → empty is fine; 200 proves the query runs. ---
GET {{base_url}}/api/v1/sales/quotations
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber

# --- 3. Non-existent sales order — 404 ---
GET {{base_url}}/api/v1/sales/orders/999999
Authorization: Bearer {{access_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 4. Deleted orders — RBAC: regular user 403 ---
GET {{base_url}}/api/v1/sales/orders/deleted
Authorization: Bearer {{access_token}}
HTTP 403

# --- 5. Deleted quotations — RBAC: regular user 403 ---
GET {{base_url}}/api/v1/sales/quotations/deleted
Authorization: Bearer {{access_token}}
HTTP 403

# --- 6. No auth — 401 ---
GET {{base_url}}/api/v1/sales/orders
HTTP 401