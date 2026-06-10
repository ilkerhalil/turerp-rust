# Tier 1.5 — Accounting. Chart of accounts is the root of the
# double-entry ledger. List and tree structure are core happy paths.


# --- 1. Chart of accounts — flat list ---
GET {{base_url}}/api/v1/chart-of-accounts
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber

# --- 2. Account tree — nested view (also validates JSON tree parsing) ---
GET {{base_url}}/api/v1/chart-of-accounts/tree
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" exists
