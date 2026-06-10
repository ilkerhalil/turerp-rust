# Tier 1.5 — CRM / cari. The "cari" entity represents customers and
# vendors in Turkish ERP convention. List endpoint is the smoke test.


# --- 1. All cari (customers + vendors) ---
GET {{base_url}}/api/v1/caris
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber

# --- 2. Cari by type (customer) ---
GET {{base_url}}/api/v1/caris/type/customer
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection

# --- 3. Non-existent cari — 404 ---
GET {{base_url}}/api/v1/caris/999999
Authorization: Bearer {{access_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists
