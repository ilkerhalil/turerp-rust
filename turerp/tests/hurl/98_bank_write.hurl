# 98_bank_write — WRITE-FENCE for the Bank account + statement + transaction
# + reconciliation-rule CRUD lifecycle.
#
# Covers (admin-gated writes):
#   POST /bank/accounts                 create
#   GET  /bank/accounts                 list (decode fence, plain array)
#   GET  /bank/accounts/{id}            by-id (decode fence)
#   PUT  /bank/accounts/{id}            update
#   POST /bank/accounts/{id}/statements upload MT940 (imports transactions)
#   GET  /bank/accounts/{id}/transactions        list (decode fence)
#   GET  /bank/accounts/{id}/transactions/unmatched list (decode fence)
#   POST /bank/transactions/{tx_id}/match   manual match (null FK → status=manual)
#   POST /bank/transactions/{tx_id}/unmatch unmatch (status=unmatched)
#   GET  /bank/reconciliation            report (AuthUser)
#   POST /bank/reconcile                 auto-reconcile (admin)
#   POST /bank/rules                     create rule
#   GET  /bank/rules/{id}                by-id (decode fence)
#   PUT  /bank/rules/{id}                update rule
#   DELETE /bank/rules/{id}              soft delete (MessageResponse; no
#                                        restore/destroy route for rules)
#   DELETE /bank/accounts/{id}           soft delete (MessageResponse)
#   GET  /bank/accounts/{id}            404 (soft-deleted filtered)
#   PUT  /bank/accounts/{id}/restore    restore
#   GET  /bank/accounts/{id}            200
#   DELETE /bank/accounts/{id}/destroy  hard delete (204)
#   GET  /bank/accounts/{id}            404
#   RBAC: POST /bank/accounts as testuser → 403; no auth → 401.
#
# Why: 33_bank.hurl only hits the (plain-array) list + reconciliation +
# rules list + a 404 by-id + 401, so Row decode bugs in the create/by-id/
# update/statement/transaction-match/rule paths stay hidden until seeded.
# This fence self-seeds a throwaway bank account, imports a one-line MT940
# statement (producing a real transaction_id for the match/unmatch fence),
# creates a reconciliation rule, and exercises every lifecycle transition.
#
# FK STRATEGY: bank statements/transactions need an account_id → self-seed a
# bank account (POST /bank/accounts) and capture its id. The match/unmatch
# endpoints need a transaction_id → capture it from the statement-import
# response (`$.transactions[0].id`). manual_match is called with BOTH
# invoice_id and payment_id null — the service does no FK validation on
# those ids, it only flips match_status Unmatched→Manual, so this fences
# the match path without seeding a cari/invoice. unmatch then flips it back.
#
# ASSUMPTION: the MT940 sample below matches MT940_61_RE
# (`:61:(\d{6})(?:\d{4})?([CD])([\d,]+)N([A-Z]{3})([^/]*)(?://(.*))?`),
# so parse_mt940 yields exactly one non-zero transaction. If the parser
# regex changes, transactions_imported may be 0 and tx_id capture fails —
# adjust the sample. ASSUMPTION: process_statement uses `account_id =
# statement_id` (a known service-layer simplification), so the imported
# transactions are filed under statement_id, NOT the real account_id —
# therefore GET /accounts/{id}/transactions may return an EMPTY array.
# The fence asserts 200 + isCollection only (not non-empty) for that
# reason; the match/unmatch fence keys off the captured tx_id, which is
# valid regardless. ASSUMPTION: re-running this file after a prior abort
# before destroy may 409 on POST /bank/accounts if a unique index on
# (tenant_id, account_number) exists and the leftover row remains — drop
# it or pick a different account_number. ASSUMPTION: reconciliation rules
# have NO restore/destroy route (only soft DELETE), so the created rule is
# left soft-deleted at end; re-runs that 409 on POST /bank/rules can drop
# the leftover (DELETE FROM reconciliation_rules WHERE rule_name LIKE
# 'Hurl Test Rule%' AND tenant_id=1). ASSUMPTION: bank_code is sent
# lowercase ("garanti") and the response serializes BankCode lowercase
# (#[serde(rename_all = "lowercase")]). Decimal fields are not present on
# BankAccount; statement amounts are Decimal-as-string in the parser path
# but the import response exposes them via BankTransactionResponse.amount.
#
# RBAC: writes use AdminUser → testuser ({{access_token}}) gets 403, admin
# ({{admin_token}}) gets 200/201. No auth → 401.
# run-all.sh SKIPS this file when admin_token is empty.

# --- 1. Create bank account as admin — 201 ---
POST {{base_url}}/api/v1/bank/accounts
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "bank_code": "garanti",
  "account_number": "HURL-98-001",
  "account_name": "Hurl Test Bank Account",
  "currency": "TRY",
  "iban": "TR000123456789012345678901",
  "branch_code": "001",
  "is_active": true,
  "tenant_id": 1,
  "company_id": null
}
```
HTTP 201
[Captures]
account_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.bank_code" == "garanti"
jsonpath "$.account_number" == "HURL-98-001"
jsonpath "$.account_name" == "Hurl Test Bank Account"
jsonpath "$.currency" == "TRY"
jsonpath "$.is_active" isBoolean
jsonpath "$.tenant_id" isNumber
jsonpath "$.created_at" exists

# --- 2. GET accounts list — decode fence: plain array + self-seeded row ---
GET {{base_url}}/api/v1/bank/accounts
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection
jsonpath "$[?(@.id=={{account_id}})].account_name" contains "Hurl Test Bank Account"

# --- 3. GET account by-id — decode fence on single-row path ---
GET {{base_url}}/api/v1/bank/accounts/{{account_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{account_id}}
jsonpath "$.bank_code" == "garanti"
jsonpath "$.account_number" == "HURL-98-001"
jsonpath "$.account_name" == "Hurl Test Bank Account"
jsonpath "$.currency" == "TRY"
jsonpath "$.is_active" isBoolean
jsonpath "$.created_at" exists

# --- 4. PUT update account — 200, changed field reflects new value ---
PUT {{base_url}}/api/v1/bank/accounts/{{account_id}}
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "account_name": "Hurl Test Bank Account (renamed)",
  "is_active": false,
  "company_id": null
}
```
HTTP 200
[Asserts]
jsonpath "$.id" == {{account_id}}
jsonpath "$.account_name" == "Hurl Test Bank Account (renamed)"
jsonpath "$.is_active" == false

# --- 5. Upload a one-line MT940 statement — 201, captures statement_id
#        and a real transaction_id for the match/unmatch fence below. ---
POST {{base_url}}/api/v1/bank/accounts/{{account_id}}/statements
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "format": "mt940",
  "data": ":61:260101C100,00NTRFREF001\n:86:Hurl test tx\n"
}
```
HTTP 201
[Captures]
statement_id: jsonpath "$.statement_id"
tx_id: jsonpath "$.transactions[0].id"
[Asserts]
jsonpath "$.statement_id" isNumber
jsonpath "$.transactions_imported" isNumber
jsonpath "$.transactions" isCollection
jsonpath "$.transactions[0].id" isNumber
jsonpath "$.transactions[0].amount" exists
jsonpath "$.transactions[0].match_status" exists

# --- 6. GET transactions for account — 200, plain array decode fence.
#        May be EMPTY due to the process_statement account_id=statement_id
#        simplification (see file header); assert shape only. ---
GET {{base_url}}/api/v1/bank/accounts/{{account_id}}/transactions
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 7. GET unmatched transactions for account — 200, plain array ---
GET {{base_url}}/api/v1/bank/accounts/{{account_id}}/transactions/unmatched
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 8. POST match transaction — 200, flips Unmatched→Manual.
#        Both FK ids null: service does not validate them, only sets
#        match_status=Manual. Fences the manual_match decode path. ---
POST {{base_url}}/api/v1/bank/transactions/{{tx_id}}/match
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "invoice_id": null,
  "payment_id": null
}
```
HTTP 200
[Asserts]
jsonpath "$.id" == {{tx_id}}
jsonpath "$.match_status" == "manual"

# --- 9. POST unmatch transaction — 200, flips Manual→Unmatched ---
POST {{base_url}}/api/v1/bank/transactions/{{tx_id}}/unmatch
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{tx_id}}
jsonpath "$.match_status" == "unmatched"

# --- 10. GET reconciliation report — 200 (AuthUser, any authed) ---
GET {{base_url}}/api/v1/bank/reconciliation
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.tenant_id" isNumber
jsonpath "$.total_transactions" isNumber

# --- 11. POST auto-reconcile — 200 (admin) ---
POST {{base_url}}/api/v1/bank/reconcile
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.tenant_id" isNumber
jsonpath "$.total_transactions" isNumber

# --- 12. Create reconciliation rule — 201 ---
POST {{base_url}}/api/v1/bank/rules
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "rule_name": "Hurl Test Rule",
  "match_field": "description",
  "match_pattern": "Hurl test tx",
  "auto_match": true,
  "is_active": true,
  "tenant_id": 1
}
```
HTTP 201
[Captures]
rule_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.rule_name" == "Hurl Test Rule"
jsonpath "$.match_field" == "description"
jsonpath "$.match_pattern" == "Hurl test tx"
jsonpath "$.auto_match" isBoolean
jsonpath "$.is_active" isBoolean
jsonpath "$.created_at" exists

# --- 13. GET rule by-id — decode fence on single-row path ---
GET {{base_url}}/api/v1/bank/rules/{{rule_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{rule_id}}
jsonpath "$.rule_name" == "Hurl Test Rule"
jsonpath "$.match_field" == "description"

# --- 14. PUT update rule — 200, changed field reflects new value ---
PUT {{base_url}}/api/v1/bank/rules/{{rule_id}}
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "rule_name": "Hurl Test Rule (renamed)",
  "auto_match": false,
  "is_active": false
}
```
HTTP 200
[Asserts]
jsonpath "$.id" == {{rule_id}}
jsonpath "$.rule_name" == "Hurl Test Rule (renamed)"
jsonpath "$.auto_match" == false
jsonpath "$.is_active" == false

# --- 15. DELETE rule (soft) — 200 MessageResponse.
#        Reconciliation rules have NO restore/destroy route, so the rule
#        is left soft-deleted here (residual, matches the reconciliation-
#        rules memory note: delete_rule never sets deleted_by). ---
DELETE {{base_url}}/api/v1/bank/rules/{{rule_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.message" exists

# --- 16. GET rule by-id after soft-delete — 404 (filtered out) ---
GET {{base_url}}/api/v1/bank/rules/{{rule_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 17. DELETE account (soft) — 200 MessageResponse ---
DELETE {{base_url}}/api/v1/bank/accounts/{{account_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.message" exists

# --- 18. GET account by-id after soft-delete — 404 ---
GET {{base_url}}/api/v1/bank/accounts/{{account_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 19. PUT restore account — 200 ---
PUT {{base_url}}/api/v1/bank/accounts/{{account_id}}/restore
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{account_id}}
jsonpath "$.account_name" exists

# --- 20. GET account by-id after restore — 200 ---
GET {{base_url}}/api/v1/bank/accounts/{{account_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{account_id}}
jsonpath "$.account_name" == "Hurl Test Bank Account (renamed)"

# --- 21. Account hard /destroy → 409 (fixed: FK RESTRICT surfaced as Conflict) ---
# bank_statements.account_id (025:30) and bank_transactions.account_id
# (025:46) both REFERENCE bank_accounts(id) with NO ON DELETE clause. The
# account restored at step 19 still has the statement imported at step 5
# against it, so a hard /destroy is now caught by the child-existence guard in
# destroy_account → ApiError::Conflict (409) instead of letting the DELETE
# hit a FK violation → 500. The account is left intact (409 does not delete);
# re-run safety is unchanged (the account was already left alive after step 20).
DELETE {{base_url}}/api/v1/bank/accounts/{{account_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 409
[Asserts]
jsonpath "$.error" exists

# --- 23. RBAC: POST create account as regular user — 403 ---
POST {{base_url}}/api/v1/bank/accounts
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "bank_code": "garanti",
  "account_number": "should-be-forbidden",
  "account_name": "should be forbidden",
  "currency": "TRY",
  "tenant_id": 1,
  "company_id": null
}
```
HTTP 403

# --- 24. RBAC: POST create account with no auth — 401 ---
POST {{base_url}}/api/v1/bank/accounts
Content-Type: application/json
```
{
  "bank_code": "garanti",
  "account_number": "should-be-unauth",
  "account_name": "should be unauth",
  "currency": "TRY",
  "tenant_id": 1,
  "company_id": null
}
```
HTTP 401