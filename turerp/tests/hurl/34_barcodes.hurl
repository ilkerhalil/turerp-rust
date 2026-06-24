# Tier 2 — Barcodes. List works, generate is POST-only (404 on GET),
# per-entity lookup 404 when no barcode is registered for the entity.

# --- 1. Barcodes list — 200 ---
GET {{base_url}}/api/v1/barcodes
Authorization: Bearer {{access_token}}
HTTP 200

# --- 2. Barcode generate — POST-only, GET 404 ---
GET {{base_url}}/api/v1/barcodes/generate
Authorization: Bearer {{access_token}}
HTTP 404

# --- 3. Barcode for an entity that has none — 404 ---
GET {{base_url}}/api/v1/barcodes/product/999999
Authorization: Bearer {{access_token}}
HTTP 404

# --- 4. No auth — 401 ---
GET {{base_url}}/api/v1/barcodes
HTTP 401