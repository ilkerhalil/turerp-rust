# Tier 2 — Accounting (accounts + journal-entries + trial-balance).
# Read-leaning smoke-GET fence: list endpoints (paginated envelope),
# accounts-by-type path, by-id 404, POST trial-balance bad body 400,
# /deleted RBAC (admin-only → testuser 403), no-auth 401.
# Why: accounting module had ZERO hurl coverage; lists may be empty unseeded
# but 200 proves the paginated SQL + AccountRow/JournalEntryRow decode runs
# (see memory: list endpoints hide decode bugs when empty).

# --- 1. Accounts list — 200, paginated envelope ---
GET {{base_url}}/api/v1/accounting/accounts
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber

# --- 2. Journal entries list — 200, paginated envelope ---
GET {{base_url}}/api/v1/accounting/journal-entries
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber

# --- 3. Accounts by type (valid enum "Asset") — 200, bare array ---
#    Handler returns Vec<Account> directly (non-paginated), so assert $ isCollection.
GET {{base_url}}/api/v1/accounting/accounts/type/Asset
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 4. Accounts by INVALID type — 400 (path enum deserialize fails) ---
GET {{base_url}}/api/v1/accounting/accounts/type/NotAType
Authorization: Bearer {{access_token}}
HTTP 400

# --- 5. Non-existent account by id — 404 ---
GET {{base_url}}/api/v1/accounting/accounts/999999
Authorization: Bearer {{access_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 6. Non-existent journal entry by id — 404 ---
GET {{base_url}}/api/v1/accounting/journal-entries/999999
Authorization: Bearer {{access_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 7. Trial balance with empty body — 400 (TrialBalanceRequest requires
#      period_start + period_end as DateTime; empty body fails JSON decode) ---
POST {{base_url}}/api/v1/accounting/trial-balance
Authorization: Bearer {{access_token}}
Content-Type: application/json
{}
HTTP 400

# --- 8. Trial balance with invalid date strings — 400 (DateTime parse fail) ---
POST {{base_url}}/api/v1/accounting/trial-balance
Authorization: Bearer {{access_token}}
Content-Type: application/json
{
    "period_start": "not-a-date",
    "period_end": "also-not-a-date"
}
HTTP 400

# --- 9. Deleted accounts — RBAC: admin-only (AdminUser) → testuser 403 ---
GET {{base_url}}/api/v1/accounting/accounts/deleted
Authorization: Bearer {{access_token}}
HTTP 403

# --- 10. Deleted journal entries — RBAC: admin-only → testuser 403 ---
GET {{base_url}}/api/v1/accounting/journal-entries/deleted
Authorization: Bearer {{access_token}}
HTTP 403

# --- 11. No auth on accounts list — 401 ---
GET {{base_url}}/api/v1/accounting/accounts
HTTP 401
[Asserts]
jsonpath "$.error" exists

# --- 12. No auth on journal-entries list — 401 ---
GET {{base_url}}/api/v1/accounting/journal-entries
HTTP 401
[Asserts]
jsonpath "$.error" exists

# --- 13. No auth on trial-balance — 401 ---
POST {{base_url}}/api/v1/accounting/trial-balance
Content-Type: application/json
{}
HTTP 401

# --- 14. No auth on deleted accounts — 401 (auth before RBAC) ---
GET {{base_url}}/api/v1/accounting/accounts/deleted
HTTP 401
[Asserts]
jsonpath "$.error" exists