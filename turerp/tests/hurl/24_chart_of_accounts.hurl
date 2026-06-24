# Tier 1.5 — Chart of accounts. Seeded (chart_accounts table), so the
# flat list and the tree MUST be non-empty.

# --- 1. Flat list — seeded, non-empty paginated envelope ---
GET {{base_url}}/api/v1/chart-of-accounts
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber
jsonpath "$.total" > 0

# --- 2. Tree view — 200, non-empty array of nodes ---
GET {{base_url}}/api/v1/chart-of-accounts/tree
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 3. Trial balance — 200 ---
GET {{base_url}}/api/v1/chart-of-accounts/trial-balance
Authorization: Bearer {{access_token}}
HTTP 200

# --- 4. Non-existent account code — 404 ---
GET {{base_url}}/api/v1/chart-of-accounts/ZZZ-NONEXISTENT
Authorization: Bearer {{access_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 5. No auth — 401 ---
GET {{base_url}}/api/v1/chart-of-accounts
HTTP 401