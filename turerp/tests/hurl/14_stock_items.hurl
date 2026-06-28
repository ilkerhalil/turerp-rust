# Tier 1.5 — Stock. The /stock/warehouses broken-endpoint gate
# (core.stock.warehouses, issue #152-5) was seeded disabled in 036 and is
# flipped ON by migration 047 once the handler is fixed. The handler had two
# root causes (both fixed in the 047 PR): WarehouseRow.total_count was not
# selected by create/find_by_id/find_by_tenant/update/find_deleted (FromRow
# decode 500), and a BEFORE UPDATE trigger referenced a missing updated_at
# column (UPDATE 500). With the flag ON the list returns 200.


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
