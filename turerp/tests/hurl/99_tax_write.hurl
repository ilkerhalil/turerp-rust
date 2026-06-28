# 99_tax_write — WRITE-FENCE for the Tax rate + tax period CRUD + calculation
# lifecycle.
#
# Covers (admin-gated writes unless noted):
#   POST   /tax/rates                  create rate
#   GET    /tax/rates                  list (decode fence, paginated envelope)
#   GET    /tax/rates/{id}             by-id (decode fence; fences int4
#                                       total_count → 0::bigint fix from PR #173)
#   PUT    /tax/rates/{id}             update rate
#   POST   /tax/calculate              calculate tax on an amount (AuthUser)
#   POST   /tax/calculate              bad tax_type → 400 (input-validation fence)
#   POST   /tax/calculate-invoice      always 501 NotImplemented (fences stub)
#   DELETE /tax/rates/{id}             soft delete (204)
#   GET    /tax/rates/{id}             404 (soft-deleted filtered)
#   PUT    /tax/rates/{id}/restore     restore
#   GET    /tax/rates/{id}             200
#   DELETE /tax/rates/{id}             soft delete again (set up bulk-restore)
#   POST   /tax/rates/bulk-restore     bulk restore (200, restored >= 1)
#   GET    /tax/rates/{id}             200
#   DELETE /tax/rates/{id}/destroy    hard delete (204)
#   GET    /tax/rates/{id}             404
#   POST   /tax/periods                create period
#   GET    /tax/periods                 list (decode fence, paginated)
#   GET    /tax/periods/{id}          by-id (decode fence; fences int4
#                                       total_count → 0::bigint fix)
#   POST   /tax/periods/{id}/calculate recalc period (Open→Calculated, 200)
#   POST   /tax/periods/{id}/file      file period (Calculated→Filed, 200)
#   DELETE /tax/periods/{id}          soft delete (204)
#   GET    /tax/periods/{id}          404
#   PUT    /tax/periods/{id}/restore  restore (individual, 200)
#   GET    /tax/periods/{id}          200
#   DELETE /tax/periods/{id}          soft delete again (set up bulk-restore)
#   POST   /tax/periods/bulk-restore  bulk restore (200, restored >= 1)
#   GET    /tax/periods/{id}          200
#   DELETE /tax/periods/{id}/destroy  hard delete (204)
#   GET    /tax/periods/{id}          404
#   RBAC:  POST /tax/rates as testuser → 403; no auth → 401.
#
# Why: 28_tax.hurl only hits the seeded rates list + a by-id + an
# effective-rate 400 (missing tax_type) + a periods list (empty) + a
# deleted-list RBAC 403 + 401, so Row decode bugs in the create/by-id/
# update/soft-delete/restore/destroy/calculate/period-calculate/period-file
# paths stay hidden until seeded. This fence self-seeds a throwaway tax
# rate (Damga, unseeded) and a throwaway tax period (KV, unseeded) and
# exercises every lifecycle transition.
#
# FK STRATEGY: tax rates are self-contained (tax_type/rate/effective dates).
# tax periods are self-contained (tax_type/year/month). tax/calculate needs
# an effective rate for (tax_type, date) — we fence the input-validation
# path (400 on bad tax_type) deterministically, AND fence a 200 happy path
# using our self-seeded Damga rate on its own effective_from date (no
# seeded Damga rate exists, so no overlap/404 risk). tax/calculate-invoice
# is a stub that always returns 501 NotImplemented — fenced deterministically.
# No cari/invoice FK is required for any path exercised here.
#
# ASSUMPTION: the seed script (scripts/seed_sample_data.sql) only inserts
# KDV/OIV/BSMV rates for tenant 1 with effective_from = CURRENT_DATE, so
# creating a Damga rate with effective_from = "2020-01-01" does NOT collide
# with the unique index (tenant_id, tax_type, effective_from) WHERE
# deleted_at IS NULL. If a prior run aborted before destroy, re-run will
# 409 on create — drop the leftover (DELETE FROM tax_rates WHERE tax_type
# = 'Damga' AND effective_from = '2020-01-01' AND tenant_id = 1) or pick a
# different effective_from. Same idempotency caveat for the KV period
# (2024, month 1): if a prior run aborted before destroy, drop the leftover
# (DELETE FROM tax_periods WHERE tax_type = 'KV' AND period_year = 2024
# AND period_month = 1 AND tenant_id = 1).
#
# ASSUMPTION: TaxType serializes as the PascalCase variant name (no
# rename_all) — "Damga", "KV", "INVALID" in bodies; response tax_type ==
# "Damga" / "KV". TaxPeriodStatus serializes PascalCase too — "Open",
# "Calculated", "Filed", "Closed". Decimal fields (rate, total_base,
# total_tax, total_deduction, net_tax) are Decimal-as-string — sent as
# strings, asserted with `exists` (tolerant to float/string serde).
#
# ASSUMPTION: calculate_period on an Open period with no detail lines
# succeeds (totals sum to 0, status → Calculated, 200). file_period
# requires status == Calculated, so it is called AFTER calculate_period.
#
# RBAC: writes use AdminUser → testuser ({{access_token}}) gets 403, admin
# ({{admin_token}}) gets 200/201. calculate & calculate-invoice use
# AuthUser (any authed) — fenced with admin_token for determinism.
# run-all.sh SKIPS this file when admin_token is empty.

# --- 1. Create tax rate as admin — 201 (Damga, unseeded → no unique clash) ---
POST {{base_url}}/api/v1/tax/rates
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "tax_type": "Damga",
  "rate": "0.01",
  "effective_from": "2020-01-01",
  "effective_to": null,
  "category": null,
  "description": "Hurl Test Damga Rate",
  "is_default": false
}
```
HTTP 201
[Captures]
rate_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.tenant_id" isNumber
jsonpath "$.tax_type" == "Damga"
jsonpath "$.rate" exists
jsonpath "$.effective_from" exists
jsonpath "$.description" == "Hurl Test Damga Rate"
jsonpath "$.is_default" isBoolean
jsonpath "$.created_at" exists

# --- 2. GET rates list — decode fence: paginated envelope + self-seeded row ---
GET {{base_url}}/api/v1/tax/rates
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber
jsonpath "$.total" > 0
jsonpath "$.items[?(@.id=={{rate_id}})].tax_type" contains "Damga"

# --- 3. GET rate by-id — decode fence on single-row path
#        (also fences the int4 total_count → 0::bigint decode fix, PR #173). ---
GET {{base_url}}/api/v1/tax/rates/{{rate_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{rate_id}}
jsonpath "$.tax_type" == "Damga"
jsonpath "$.rate" exists
jsonpath "$.effective_from" exists
jsonpath "$.description" == "Hurl Test Damga Rate"

# --- 4. PUT update rate — 200, changed field reflects new value ---
PUT {{base_url}}/api/v1/tax/rates/{{rate_id}}
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "description": "Hurl Test Damga Rate (renamed)",
  "is_default": false
}
```
HTTP 200
[Asserts]
jsonpath "$.id" == {{rate_id}}
jsonpath "$.tax_type" == "Damga"
jsonpath "$.description" == "Hurl Test Damga Rate (renamed)"

# --- 5. POST calculate tax — 200 happy path.
#        Our Damga rate covers 2020-01-01 (effective_from = 2020-01-01,
#        effective_to = null) and no seeded Damga rate exists, so the
#        effective-rate lookup resolves to our row → 200. ---
POST {{base_url}}/api/v1/tax/calculate
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "amount": "1000.00",
  "tax_type": "Damga",
  "date": "2020-01-01",
  "inclusive": false
}
```
HTTP 200
[Asserts]
jsonpath "$.base_amount" exists
jsonpath "$.tax_type" == "Damga"
jsonpath "$.rate" exists
jsonpath "$.tax_amount" exists
jsonpath "$.inclusive" isBoolean

# --- 6. POST calculate tax — 400 input-validation fence (bad tax_type).
#        The handler parses tax_type via FromStr; "INVALID" fails →
#        ApiError::Validation → 400. This is the deterministic validation
#        fence (per task: "fence the input-validation path (400 on bad body)"). ---
POST {{base_url}}/api/v1/tax/calculate
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "amount": "1000.00",
  "tax_type": "INVALID",
  "date": "2020-01-01",
  "inclusive": false
}
```
HTTP 400
[Asserts]
jsonpath "$.error" exists

# --- 7. POST calculate-invoice — 501 NotImplemented (stub).
#        calculate_invoice_tax always returns 501 with a MessageResponse
#        (tax.invoice_calculation_not_implemented). Fences the stub so a
#        future implementation flips this assertion to 200. ---
POST {{base_url}}/api/v1/tax/calculate-invoice
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "invoice_id": 1
}
```
HTTP 501
[Asserts]
jsonpath "$.message" exists

# --- 8. DELETE rate (soft) — 204 ---
DELETE {{base_url}}/api/v1/tax/rates/{{rate_id}}
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 9. GET rate by-id after soft-delete — 404 ---
GET {{base_url}}/api/v1/tax/rates/{{rate_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 10. PUT restore rate — 200 (individual restore) ---
PUT {{base_url}}/api/v1/tax/rates/{{rate_id}}/restore
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{rate_id}}
jsonpath "$.tax_type" == "Damga"

# --- 11. GET rate by-id after restore — 200 ---
GET {{base_url}}/api/v1/tax/rates/{{rate_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{rate_id}}
jsonpath "$.description" == "Hurl Test Damga Rate (renamed)"

# --- 12. DELETE rate (soft) again — 204 (set up bulk-restore) ---
DELETE {{base_url}}/api/v1/tax/rates/{{rate_id}}
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 13. POST bulk-restore rates — 200, restored >= 1 ---
POST {{base_url}}/api/v1/tax/rates/bulk-restore
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "ids": [{{rate_id}}]
}
```
HTTP 200
[Asserts]
jsonpath "$.restored" isNumber
jsonpath "$.restored" > 0
jsonpath "$.items" isCollection
jsonpath "$.failed" isCollection

# --- 14. GET rate by-id after bulk-restore — 200 ---
GET {{base_url}}/api/v1/tax/rates/{{rate_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{rate_id}}

# --- 15. DELETE rate /destroy — 204 (hard delete) ---
DELETE {{base_url}}/api/v1/tax/rates/{{rate_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 16. GET rate by-id after destroy — 404 ---
GET {{base_url}}/api/v1/tax/rates/{{rate_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 17. Create tax period as admin — 201 (KV, unseeded → no clash) ---
POST {{base_url}}/api/v1/tax/periods
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "tax_type": "KV",
  "period_year": 2024,
  "period_month": 1
}
```
HTTP 201
[Captures]
period_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.tenant_id" isNumber
jsonpath "$.tax_type" == "KV"
jsonpath "$.period_year" == 2024
jsonpath "$.period_month" == 1
jsonpath "$.status" == "Open"
jsonpath "$.total_base" exists
jsonpath "$.total_tax" exists
jsonpath "$.net_tax" exists
jsonpath "$.created_at" exists

# --- 18. GET periods list — decode fence: paginated envelope + self-seeded row ---
GET {{base_url}}/api/v1/tax/periods
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber
jsonpath "$.items[?(@.id=={{period_id}})].tax_type" contains "KV"

# --- 19. GET period by-id — decode fence on single-row path
#         (also fences the int4 total_count → 0::bigint decode fix). ---
GET {{base_url}}/api/v1/tax/periods/{{period_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{period_id}}
jsonpath "$.tax_type" == "KV"
jsonpath "$.period_year" == 2024
jsonpath "$.period_month" == 1
jsonpath "$.status" == "Open"

# --- 20. POST period calculate — 200 (Open→Calculated; no detail lines → 0 totals) ---
POST {{base_url}}/api/v1/tax/periods/{{period_id}}/calculate
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{period_id}}
jsonpath "$.status" == "Calculated"
jsonpath "$.total_base" exists
jsonpath "$.net_tax" exists

# --- 21. POST period file — 200 (Calculated→Filed) ---
POST {{base_url}}/api/v1/tax/periods/{{period_id}}/file
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{period_id}}
jsonpath "$.status" == "Filed"
jsonpath "$.filed_at" exists

# --- 22. DELETE period (soft) — 204 ---
DELETE {{base_url}}/api/v1/tax/periods/{{period_id}}
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 23. GET period by-id after soft-delete — 404 ---
GET {{base_url}}/api/v1/tax/periods/{{period_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 24. PUT restore period — 200 (individual restore) ---
PUT {{base_url}}/api/v1/tax/periods/{{period_id}}/restore
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{period_id}}
jsonpath "$.tax_type" == "KV"

# --- 25. GET period by-id after restore — 200 ---
GET {{base_url}}/api/v1/tax/periods/{{period_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{period_id}}
jsonpath "$.status" == "Filed"

# --- 26. DELETE period (soft) again — 204 (set up bulk-restore) ---
DELETE {{base_url}}/api/v1/tax/periods/{{period_id}}
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 27. POST bulk-restore periods — 200, restored >= 1 ---
POST {{base_url}}/api/v1/tax/periods/bulk-restore
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "ids": [{{period_id}}]
}
```
HTTP 200
[Asserts]
jsonpath "$.restored" isNumber
jsonpath "$.restored" > 0
jsonpath "$.items" isCollection
jsonpath "$.failed" isCollection

# --- 28. GET period by-id after bulk-restore — 200 ---
GET {{base_url}}/api/v1/tax/periods/{{period_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{period_id}}

# --- 29. DELETE period /destroy — 204 (hard delete) ---
DELETE {{base_url}}/api/v1/tax/periods/{{period_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 30. GET period by-id after destroy — 404 ---
GET {{base_url}}/api/v1/tax/periods/{{period_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 31. RBAC: POST create rate as regular user — 403 ---
POST {{base_url}}/api/v1/tax/rates
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "tax_type": "Damga",
  "rate": "0.01",
  "effective_from": "2020-01-01",
  "effective_to": null,
  "category": null,
  "description": "should be forbidden",
  "is_default": false
}
```
HTTP 403

# --- 32. RBAC: POST create rate with no auth — 401 ---
POST {{base_url}}/api/v1/tax/rates
Content-Type: application/json
```
{
  "tax_type": "Damga",
  "rate": "0.01",
  "effective_from": "2020-01-01",
  "effective_to": null,
  "category": null,
  "description": "should be unauth",
  "is_default": false
}
```
HTTP 401