# Tier 2 — e-Fatura / e-Arsiv (Turkish e-invoicing). The list roots ARE
# registered as GET, but their Query struct fails to deserialize (live body:
# "Query deserialize error: invalid type: map, expected option" — reproduces
# even with ?page=1) → 400 on any GET. That is a real query-deser bug, fenced
# here; flip to 200 + non-empty when fixed. Per-id and status lookups return
# 404 for a missing document.

# --- 1. e-Fatura root — 400 (Query deserialize bug, fenced) ---
GET {{base_url}}/api/v1/efatura
Authorization: Bearer {{access_token}}
HTTP 400

# --- 2. e-Arsiv root — 400 (Query deserialize bug, fenced) ---
GET {{base_url}}/api/v1/earchive
Authorization: Bearer {{access_token}}
HTTP 400

# --- 3. Non-existent e-Fatura by id — 404 ---
GET {{base_url}}/api/v1/efatura/999999
Authorization: Bearer {{access_token}}
HTTP 404

# --- 4. e-Fatura status by unknown uuid — 404 ---
GET {{base_url}}/api/v1/efatura/status/nonexistent-uuid
Authorization: Bearer {{access_token}}
HTTP 404

# --- 5. Non-existent e-Arsiv by id — 404 ---
GET {{base_url}}/api/v1/earchive/999999
Authorization: Bearer {{access_token}}
HTTP 404

# --- 6. No auth — 401 ---
GET {{base_url}}/api/v1/efatura/999999
HTTP 401