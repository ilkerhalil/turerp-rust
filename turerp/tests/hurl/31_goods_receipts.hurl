# Tier 2 — Goods receipts. The collection root is POST-only (create a
# receipt) — GET returns 405. Reads are by id, by order, or via /deleted.
# /deleted is admin-only (403).

# --- 1. Root collection GET — 405 (only POST is registered) ---
GET {{base_url}}/api/v1/goods-receipts
Authorization: Bearer {{access_token}}
HTTP 405

# --- 2. Get-by-id — 404 (was 500: goods_receipts missing company_id, fixed
#        by migration 042; find_by_id now runs and returns NotFound). ---
GET {{base_url}}/api/v1/goods-receipts/999999
Authorization: Bearer {{access_token}}
HTTP 404

# --- 3. Receipts for a non-existent order — 200 with empty array (was 500:
#        same missing-company_id bug on find_by_order). ---
GET {{base_url}}/api/v1/goods-receipts/order/999999
Authorization: Bearer {{access_token}}
HTTP 200

# --- 4. Deleted receipts — RBAC: regular user 403 ---
GET {{base_url}}/api/v1/goods-receipts/deleted
Authorization: Bearer {{access_token}}
HTTP 403

# --- 5. No auth on root — 401 (auth before method check) ---
GET {{base_url}}/api/v1/goods-receipts
HTTP 401