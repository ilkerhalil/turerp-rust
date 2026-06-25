# Tier 2 — Bank integration. accounts list works (plain JSON array,
# not the paginated envelope), reconciliation works, rules list is 200
# (was 500 — reconciliation_rules lacked deleted_at/deleted_by; fixed by
# migration 044).

# --- 1. Bank accounts list — 200, plain array body ---
GET {{base_url}}/api/v1/bank/accounts
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 2. Bank reconciliation — 200 ---
GET {{base_url}}/api/v1/bank/reconciliation
Authorization: Bearer {{access_token}}
HTTP 200

# --- 3. Bank rules list — 200 (was 500: reconciliation_rules table
#        lacked the deleted_at/deleted_by columns that ReconciliationRuleRow
#        SELECTs and filters on; fixed by migration 044. Plain JSON array.) ---
GET {{base_url}}/api/v1/bank/rules
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 4. Non-existent bank account — 404 ---
GET {{base_url}}/api/v1/bank/accounts/999999
Authorization: Bearer {{access_token}}
HTTP 404

# --- 5. No auth — 401 ---
GET {{base_url}}/api/v1/bank/accounts
HTTP 401