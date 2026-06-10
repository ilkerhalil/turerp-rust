# Tier 1.5 — Stock. Warehouses endpoint is currently broken (500,
# see issue #152). Asserted to keep the regression visible.


# --- 1. Warehouses list — currently 500 (P1) ---
GET {{base_url}}/api/v1/stock/warehouses
Authorization: Bearer {{access_token}}
HTTP 500

# --- 2. Non-existent warehouse — currently 500 (same DB error, see #152) ---
GET {{base_url}}/api/v1/stock/warehouses/999999
Authorization: Bearer {{access_token}}
HTTP 500
