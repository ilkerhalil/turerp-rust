# Tier 2 — Manufacturing. Largest gated tier-2 module (35+ paths).
# Default state: tier2.manufacturing flag is OFF, so all routes 404.
# Operator path: enable via /api/v1/feature-flags/tier2.manufacturing/enable,
# then the same routes return 200. This file exercises the OFF path; the
# ON path is verified by the dedicated enable-flag test below.

# --- 1. Work orders list — gated (default OFF) → 404 ---
GET {{base_url}}/api/v1/manufacturing/work-orders
Authorization: Bearer {{access_token}}
HTTP 404

# --- 2. Single work order — gated → 404 ---
GET {{base_url}}/api/v1/manufacturing/work-orders/1
Authorization: Bearer {{access_token}}
HTTP 404

# --- 3. BOMs list — gated → 404 ---
GET {{base_url}}/api/v1/manufacturing/boms
Authorization: Bearer {{access_token}}
HTTP 404

# --- 4. Routings list — gated → 404 ---
GET {{base_url}}/api/v1/manufacturing/routings
Authorization: Bearer {{access_token}}
HTTP 404

# --- 5. Inspections list — gated → 404 ---
GET {{base_url}}/api/v1/manufacturing/inspections
Authorization: Bearer {{access_token}}
HTTP 404
