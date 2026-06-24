# Tier 2 — Full-text search. The query endpoint requires a ?q= param:
# 200 with a query, 400 without. index/reindex are POST-only.

# --- 1. Search with query — 200 ---
GET {{base_url}}/api/v1/search?q=deneme
Authorization: Bearer {{access_token}}
HTTP 200

# --- 2. Search without ?q= — 400 (required param missing) ---
GET {{base_url}}/api/v1/search
Authorization: Bearer {{access_token}}
HTTP 400

# --- 3. Reindex — POST-only, GET 404 ---
GET {{base_url}}/api/v1/search/reindex
Authorization: Bearer {{access_token}}
HTTP 404

# --- 4. No auth — 401 ---
GET {{base_url}}/api/v1/search?q=deneme
HTTP 401