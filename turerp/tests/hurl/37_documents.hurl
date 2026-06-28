# Tier 2 — Document management. List, /deleted, and /categories all work.
# (/categories previously 404'd because /v1/documents/{id} was registered before
# /v1/documents/categories in actix-web 4 — registration-order matching let the
# dynamic {id} shadow the literal "categories" segment. Fixed by reordering the
# categories resource before {id}.)

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

# --- 3. Document categories — 200 (bare Vec; fixed: no longer shadowed by {id}) ---
GET {{base_url}}/api/v1/documents/categories
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 4. Non-existent document — 404 ---
GET {{base_url}}/api/v1/documents/999999
Authorization: Bearer {{access_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 5. No auth — 401 ---
GET {{base_url}}/api/v1/documents
HTTP 401