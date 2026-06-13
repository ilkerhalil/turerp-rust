# Tier 1.5 — Stock. Warehouses endpoint is gated by core.stock.warehouses
# (default-disabled). Issue #152 fixed the underlying WarehouseRow
# decode (phantom company_id column) so warehouses now returns 200
# when the flag is enabled.


# --- 1. Warehouses list — fixed by #152 (handler SQL bug) ---
GET {{base_url}}/api/v1/stock/warehouses
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber

# --- 2. Non-existent warehouse — still 404 (no row) ---
GET {{base_url}}/api/v1/stock/warehouses/999999
Authorization: Bearer {{access_token}}
HTTP 404
