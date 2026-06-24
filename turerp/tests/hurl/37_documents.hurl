# Tier 2 — Document management. List and /deleted work; /categories
# currently 404 (route registered but no GET list returns rows — fence).

# --- 1. Documents list — 200 (paginated envelope) ---
GET {{base_url}}/api/v1/documents
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber

# --- 2. Deleted documents — 200 (documents/deleted is NOT admin-gated) ---
GET {{base_url}}/api/v1/documents/deleted
Authorization: Bearer {{access_token}}
HTTP 200

# --- 3. Document categories — 404 (route exists but returns 404; fence) ---
GET {{base_url}}/api/v1/documents/categories
Authorization: Bearer {{access_token}}
HTTP 404

# --- 4. Non-existent document — 404 ---
GET {{base_url}}/api/v1/documents/999999
Authorization: Bearer {{access_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 5. No auth — 401 ---
GET {{base_url}}/api/v1/documents
HTTP 401