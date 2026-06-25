# Tier 2 — Purchase (orders + requests). orders + requests lists work
# (requests unseeded → empty). requests list was 500 (broken — the
# purchase_requests table was missing company_id, which migration 023 added
# to 10 sibling tables but skipped; fixed by migration 042). /deleted lists
# are admin-only (403).

# --- 1. Purchase orders list — 200 (paginated envelope) ---
GET {{base_url}}/api/v1/purchase-orders
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber

# --- 2. Purchase requests list — 200 (was 500: purchase_requests missing
#     company_id, fixed by migration 042). Unseeded → empty is fine; 200 +
#     envelope proves the query runs. ---
GET {{base_url}}/api/v1/purchase-requests
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber

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