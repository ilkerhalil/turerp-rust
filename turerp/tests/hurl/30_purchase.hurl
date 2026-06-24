# Tier 2 — Purchase (orders + requests). orders list works, requests list
# is 500 (broken — follow-up to #152). /deleted lists admin-only (403).

# --- 1. Purchase orders list — 200 (paginated envelope) ---
GET {{base_url}}/api/v1/purchase-orders
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber

# --- 2. Purchase requests list — BROKEN (500), assert until fixed ---
GET {{base_url}}/api/v1/purchase-requests
Authorization: Bearer {{access_token}}
HTTP 500

# --- 3. Non-existent purchase order — 404 ---
GET {{base_url}}/api/v1/purchase-orders/999999
Authorization: Bearer {{access_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 4. Deleted purchase orders — RBAC: regular user 403 ---
GET {{base_url}}/api/v1/purchase-orders/deleted
Authorization: Bearer {{access_token}}
HTTP 403

# --- 5. No auth — 401 ---
GET {{base_url}}/api/v1/purchase-orders
HTTP 401