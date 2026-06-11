# Tier 1.5 — Stock. Warehouses endpoint is now gated (core.stock.warehouses,
# default-disabled). The gate fires BEFORE the underlying handler runs,
# so both /stock/warehouses and /stock/warehouses/{id} return 404 with
# the flag off. PR 2 will fix the underlying handler AND the operator
# will flip the flag on (these are tracked together).


# --- 1. Warehouses list — currently 404 (gated; PR 2 fixes handler) ---
GET {{base_url}}/api/v1/stock/warehouses
Authorization: Bearer {{access_token}}
HTTP 404

# --- 2. Non-existent warehouse — also 404 (gate fires before handler) ---
GET {{base_url}}/api/v1/stock/warehouses/999999
Authorization: Bearer {{access_token}}
HTTP 404
