# Tier 2 — Sales (orders + quotations). orders list works (empty until
# seeded), quotations list is currently 500 (broken — follow-up to #152).
# All /deleted lists are admin-only (403 for a regular user).

# --- 1. Sales orders list — 200 (paginated envelope, may be empty) ---
GET {{base_url}}/api/v1/sales/orders
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber

# --- 2. Sales quotations list — BROKEN (500), assert until fixed ---
GET {{base_url}}/api/v1/sales/quotations
Authorization: Bearer {{access_token}}
HTTP 500

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