# Tier 1.5 — Products (catalog). Seeded by scripts/seed_sample_data.sql,
# so the list MUST be non-empty — that is what flushes out ProductRow
# decode bugs (see memory: list endpoints hide decode bugs when empty).

# --- 1. All products — seeded, expect non-empty envelope ---
GET {{base_url}}/api/v1/products
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber
jsonpath "$.total" > 0

# --- 2. Product search with query param — 200 (needs ?q=) ---
GET {{base_url}}/api/v1/products/search?q=A
Authorization: Bearer {{access_token}}
HTTP 200

# --- 3. Product search without ?q= — 400 (required param missing) ---
GET {{base_url}}/api/v1/products/search
Authorization: Bearer {{access_token}}
HTTP 400

# --- 4. Non-existent product — 404 ---
GET {{base_url}}/api/v1/products/999999
Authorization: Bearer {{access_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 5. Variant lookup for a product that has none — 404 ---
GET {{base_url}}/api/v1/products/999999/variants
Authorization: Bearer {{access_token}}
HTTP 404

# --- 6. List without auth — 401 ---
GET {{base_url}}/api/v1/products
HTTP 401
[Asserts]
jsonpath "$.error" exists