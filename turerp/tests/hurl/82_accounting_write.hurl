# 82_accounting_write — WRITE-FENCE for the Accounting CRUD + journal-entry
# lifecycle.
#
# Covers:
#   /accounting/accounts  — POST create → GET list (decode) → GET by-id
#                           (decode) → DELETE soft → GET 404 → PUT restore
#                           → GET 200 → DELETE /destroy → GET 404.
#   /accounting/journal-entries — POST create (lines w/ account_id FK)
#                           → GET list (decode) → GET by-id (decode)
#                           → POST /{id}/post (Draft→Posted)
#                           → POST /{id}/void (Posted→Voided)
#                           → DELETE soft → GET 404 → PUT restore
#                           → GET 200 → DELETE /destroy → GET 404.
#   /accounting/trial-balance — POST (authed) → 200 TrialBalance envelope.
#
# Why: the read-only 12_accounting_accounts.hurl / 58_accounting.hurl only
# hit the (empty) list + by-id 404, so Row decode bugs in the create/post/
# void/soft-delete/restore/destroy paths stay hidden until seeded. This
# fence self-seeds two accounts + a balanced journal entry, then
# exercises every lifecycle transition.
#
# RBAC: writes (create account, create journal entry, post, void,
# soft-delete, restore, destroy) use AdminUser → testuser
# ({{access_token}}) gets 403, admin ({{admin_token}}) gets 200/201. No
# auth → 401. trial-balance uses AuthUser (any authed 200). run-all.sh
# SKIPS this file when admin_token is empty.
#
# FK STRATEGY: journal-entry lines need account_id. We self-seed TWO
# accounts (account_a, account_b) before creating the entry and reference
# both captured ids — never hardcode FK ids.
#
# ASSUMPTION: rust_decimal::Decimal deserializes from a JSON string
# (matches 74_cari_write.hurl credit_limit style); debit/credit sent as
# "100.00". ASSUMPTION: account code is unique per tenant; a second run
# against the same DB will 409 at step 1/2 — re-run safety relies on DB
# reset between full-suite runs. ASSUMPTION: company_id=1 is seeded for
# tenant 1 (create_account does NOT inject company_id, only tenant_id).
# ASSUMPTION: POST /post then POST /void is the valid transition order
# (Draft→Posted→Voided); voiding a Draft may 400 — flagged for review.

# --- 1. Create account A (admin) → 201; capture id ---
POST {{base_url}}/api/v1/accounting/accounts
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "company_id": 1,
  "code": "WF-CASH-1001",
  "name": "Hurl Test Cash",
  "account_type": "Asset",
  "sub_type": "CurrentAsset",
  "parent_id": null,
  "allow_transaction": true
}
```
HTTP 201
[Captures]
account_a_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.code" == "WF-CASH-1001"
jsonpath "$.account_type" == "Asset"
jsonpath "$.sub_type" == "CurrentAsset"
jsonpath "$.company_id" isNumber
jsonpath "$.is_active" == true
jsonpath "$.allow_transaction" == true
jsonpath "$.created_at" exists

# --- 2. Create account B (admin) → 201; capture id ---
POST {{base_url}}/api/v1/accounting/accounts
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "company_id": 1,
  "code": "WF-REV-4001",
  "name": "Hurl Test Revenue",
  "account_type": "Revenue",
  "sub_type": "OperatingRevenue",
  "parent_id": null,
  "allow_transaction": true
}
```
HTTP 201
[Captures]
account_b_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.account_type" == "Revenue"

# --- 3. GET accounts list — DECODE FENCE: paginated envelope ---
GET {{base_url}}/api/v1/accounting/accounts
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber
jsonpath "$.total" > 0
jsonpath "$.items[?(@.id=={{account_a_id}})].code" contains "WF-CASH-1001"

# --- 4. GET account by-id — DECODE FENCE: single-row Account ---
GET {{base_url}}/api/v1/accounting/accounts/{{account_a_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{account_a_id}}
jsonpath "$.code" == "WF-CASH-1001"
jsonpath "$.account_type" == "Asset"
jsonpath "$.deleted_at" exists
jsonpath "$.created_at" exists

# --- 5. Create journal entry (admin) → 201; balanced lines reference both accounts ---
POST {{base_url}}/api/v1/accounting/journal-entries
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "company_id": 1,
  "date": "2026-06-27T00:00:00Z",
  "description": "Hurl write-fence journal entry",
  "reference": "WF-JE-001",
  "created_by": {{user_id}},
  "lines": [
    {
      "account_id": {{account_a_id}},
      "cost_center_id": null,
      "debit": "100.00",
      "credit": "0.00",
      "description": "debit cash",
      "reference": null
    },
    {
      "account_id": {{account_b_id}},
      "cost_center_id": null,
      "debit": "0.00",
      "credit": "100.00",
      "description": "credit revenue",
      "reference": null
    }
  ]
}
```
HTTP 201
[Captures]
je_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.entry_number" isString
jsonpath "$.status" == "Draft"
jsonpath "$.total_debit" exists
jsonpath "$.total_credit" exists
jsonpath "$.created_by" isNumber
jsonpath "$.company_id" isNumber

# --- 6. GET journal entries list — DECODE FENCE: paginated envelope ---
GET {{base_url}}/api/v1/accounting/journal-entries
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber
jsonpath "$.total" > 0
jsonpath "$.items[?(@.id=={{je_id}})].status" contains "Draft"

# --- 7. GET journal entry by-id — DECODE FENCE: single-row JournalEntry ---
GET {{base_url}}/api/v1/accounting/journal-entries/{{je_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{je_id}}
jsonpath "$.description" == "Hurl write-fence journal entry"
jsonpath "$.status" == "Draft"
jsonpath "$.deleted_at" exists

# --- 8. POST /post (admin) → 200; status transitions Draft → Posted ---
POST {{base_url}}/api/v1/accounting/journal-entries/{{je_id}}/post
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{je_id}}
jsonpath "$.status" == "Posted"
jsonpath "$.posted_at" exists

# --- 9. POST /void (admin) → 200; status transitions Posted → Voided ---
POST {{base_url}}/api/v1/accounting/journal-entries/{{je_id}}/void
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{je_id}}
jsonpath "$.status" == "Voided"

# --- 10. DELETE journal entry soft (admin) → 200 MessageResponse ---
DELETE {{base_url}}/api/v1/accounting/journal-entries/{{je_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.message" exists

# --- 11. GET journal entry by-id after soft-delete → 404 ---
GET {{base_url}}/api/v1/accounting/journal-entries/{{je_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 12. PUT restore journal entry (admin) → 200 ---
PUT {{base_url}}/api/v1/accounting/journal-entries/{{je_id}}/restore
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{je_id}}
jsonpath "$.status" exists

# --- 13. GET journal entry by-id after restore → 200 ---
GET {{base_url}}/api/v1/accounting/journal-entries/{{je_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{je_id}}

# --- 14. DELETE journal entry /destroy (admin) → 204 ---
DELETE {{base_url}}/api/v1/accounting/journal-entries/{{je_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 15. GET journal entry by-id after destroy → 404 ---
GET {{base_url}}/api/v1/accounting/journal-entries/{{je_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 16. POST trial-balance (authed) → 200 TrialBalance envelope ---
POST {{base_url}}/api/v1/accounting/trial-balance
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "period_start": "2026-01-01T00:00:00Z",
  "period_end": "2026-12-31T23:59:59Z"
}
```
HTTP 200
[Asserts]
jsonpath "$.period_start" exists
jsonpath "$.period_end" exists
jsonpath "$.accounts" isCollection
jsonpath "$.total_debits" exists
jsonpath "$.total_credits" exists

# --- 17. DELETE account A soft (admin) → 200 MessageResponse ---
DELETE {{base_url}}/api/v1/accounting/accounts/{{account_a_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.message" exists

# --- 18. GET account A by-id after soft-delete → 404 ---
GET {{base_url}}/api/v1/accounting/accounts/{{account_a_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 19. PUT restore account A (admin) → 200 ---
PUT {{base_url}}/api/v1/accounting/accounts/{{account_a_id}}/restore
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{account_a_id}}
jsonpath "$.code" == "WF-CASH-1001"

# --- 20. GET account A by-id after restore → 200 ---
GET {{base_url}}/api/v1/accounting/accounts/{{account_a_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{account_a_id}}

# --- 21. DELETE account A /destroy (admin) → 204 ---
DELETE {{base_url}}/api/v1/accounting/accounts/{{account_a_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 22. GET account A by-id after destroy → 404 ---
GET {{base_url}}/api/v1/accounting/accounts/{{account_a_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 22b. Cleanup: soft-delete account B (re-run safety) ---
# chart_accounts(tenant_id, code) is a PARTIAL unique index (009:21, WHERE
# deleted_at IS NULL), so soft-deleting account_b frees its (tenant_id, code)
# slot for the next run. The main journal entry (and its lines) are already
# hard-destroyed at step 14, so account_b has no journal_lines children here;
# we still use soft-delete (not hard /destroy) because it is FK-safe (a plain
# UPDATE of deleted_at — never 500s on any residual FK reference) and is
# sufficient to free the partial-unique slot. Hard /destroy is unnecessary
# and would 500 if any other seeded row references account_b.
DELETE {{base_url}}/api/v1/accounting/accounts/{{account_b_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.message" exists

# --- 23. RBAC: POST create account as regular user → 403 ---
POST {{base_url}}/api/v1/accounting/accounts
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "company_id": 1,
  "code": "WF-RBAC-403",
  "name": "should be forbidden",
  "account_type": "Asset",
  "sub_type": "CurrentAsset",
  "parent_id": null,
  "allow_transaction": true
}
```
HTTP 403

# --- 24. RBAC: POST create journal entry as regular user → 403 ---
POST {{base_url}}/api/v1/accounting/journal-entries
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "company_id": 1,
  "date": "2026-06-27T00:00:00Z",
  "description": "should be forbidden",
  "reference": null,
  "created_by": {{user_id}},
  "lines": [
    { "account_id": {{account_b_id}}, "cost_center_id": null, "debit": "1.00", "credit": "0.00", "description": null, "reference": null },
    { "account_id": {{account_b_id}}, "cost_center_id": null, "debit": "0.00", "credit": "1.00", "description": null, "reference": null }
  ]
}
```
HTTP 403

# --- 25. RBAC: POST create account with no auth → 401 ---
POST {{base_url}}/api/v1/accounting/accounts
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "company_id": 1,
  "code": "WF-NOAUTH",
  "name": "should be unauth",
  "account_type": "Asset",
  "sub_type": "CurrentAsset",
  "parent_id": null,
  "allow_transaction": true
}
```
HTTP 401