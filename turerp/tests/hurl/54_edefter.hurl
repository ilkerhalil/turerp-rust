# Tier 2 — e-Defter (electronic ledger). The /periods root IS registered as
# GET, but its Query struct fails to deserialize (live body: "Query
# deserialize error: invalid type: map, expected option" — reproduces even
# with ?page=1) → 400 on any GET. Real query-deser bug, fenced here; flip to
# 200 + non-empty when fixed. Per-period GET returns 404 for a missing id.

# --- 1. Periods root — 400 (Query deserialize bug, fenced) ---
GET {{base_url}}/api/v1/edefter/periods
Authorization: Bearer {{access_token}}
HTTP 400

# --- 2. Non-existent period — 404 ---
GET {{base_url}}/api/v1/edefter/periods/999999
Authorization: Bearer {{access_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 3. No auth — 401 ---
GET {{base_url}}/api/v1/edefter/periods/999999
HTTP 401