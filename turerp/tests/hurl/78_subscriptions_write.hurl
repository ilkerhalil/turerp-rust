# 78_subscriptions_write — WRITE-FENCE for SaaS subscriptions & plans.
#
# Covers plan CRUD (admin) + subscription CRUD (auth) + the special billing
# endpoints the read-only 49_subscriptions.hurl never exercises:
#   /subscription-plans          POST/PUT/DELETE
#   /subscriptions               POST/PUT/DELETE
#   /subscriptions/{id}/renew     POST
#   /subscriptions/{id}/proration POST
#   /subscriptions/{id}/dunning  GET + POST (retry)
#   /subscriptions/{id}/trial/convert POST
#   /subscriptions/trials        GET  (shadowed by {id} → 404, documented)
#   /subscriptions/{id}/usage    GET + POST
#   /subscriptions/{id}/cancel   POST
#   /subscriptions/due-for-billing GET
#
# Why: 49 only hits the empty list decode. The create/by-id/renew/proration/
# dunning/usage/cancel/trial-convert Row decode paths stay hidden until a
# real subscription is seeded. This fence self-seeds two plans + a trial
# subscription and walks every transition.
#
# RBAC: plan writes use AdminUser → testuser ({{access_token}}) gets 403,
# admin ({{admin_token}}) gets 200/201. Subscription + special endpoints use
# AuthUser → any authenticated user (testuser 200/201); only no-auth → 401.
# run-all.sh SKIPS this file when admin_token is empty.
#
# FK STRATEGY: plan create is self-contained (admin self-seeds two plans,
# captures both ids). customer_id is DISCOVERED via GET-list-capture from
# /caris ($.items[0].id) — the subscription handler does NOT validate
# customer_id existence, but the DB may enforce an FK.
#
# ASSUMPTION: suite runs against a freshly reset DB. Static plan names
# `Hurl Plan A` / `Hurl Plan B` are unique per tenant; a second run against
# the same DB will 409 at step 1/2. Re-run safety relies on DB reset.
# ASSUMPTION: tenant 1 has >=1 cari row for the customer_id capture. If
# /caris is empty, step 3 capture fails — seed a cari first (74_cari_write).
# ASSUMPTION: the subscription is created in `trial` status so trial/convert
# (step 8) succeeds; renew (step 9) runs AFTER convert so next_billing_date is
# set by the conversion, satisfying renew's not-null requirement.
# ASSUMPTION: renew creates a subscription invoice (service.create_subscription
# _invoice is called in renew_subscription), so GET invoices (step 10) yields
# >=1 row whose id feeds the dunning retry POST (step 15). If renew produces
# no invoice, step 10 capture fails — flagged for review.
# ASSUMPTION: GET /subscriptions/trials returns 404 because the literal route
# is registered AFTER /subscriptions/{id} and actix matches the dynamic
# {id} resource first, failing to parse "trials" as i64 → 404 (confirmed by
# 49_subscriptions.hurl step 3). The route exists but is shadowed.
# ASSUMPTION: BillingCycle / SubscriptionStatus use #[serde(rename_all=
# "lowercase")] → JSON bodies send "monthly"/"trial"; plan/subscription
# Response enums are NOT renamed → center_type etc. are PascalCase (n/a here).
# ASSUMPTION: rust_decimal deserializes base_amount from a JSON number; if
# the Decimal serde feature requires a string, step 1 will 400.

# --- 1. POST plan A (admin) → 201; capture plan_id (FK parent) ---
POST {{base_url}}/api/v1/subscription-plans
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "name": "Hurl Plan A",
  "description": "hurl write-fence seeded plan A",
  "billing_cycle": "monthly",
  "base_amount": 100.00,
  "currency": "TRY",
  "is_active": true,
  "included_quantity": 1000,
  "overage_rate": 0.10,
  "tenant_id": {{tenant_id}}
}
```
HTTP 201
[Captures]
plan_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.name" == "Hurl Plan A"
jsonpath "$.billing_cycle" == "monthly"
jsonpath "$.currency" == "TRY"
jsonpath "$.is_active" == true
jsonpath "$.tenant_id" isNumber
jsonpath "$.created_at" exists

# --- 2. POST plan B (admin) → 201; capture plan2_id (proration target) ---
POST {{base_url}}/api/v1/subscription-plans
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "name": "Hurl Plan B",
  "description": "hurl write-fence seeded plan B (proration target)",
  "billing_cycle": "yearly",
  "base_amount": 1000.00,
  "currency": "TRY",
  "is_active": true,
  "tenant_id": {{tenant_id}}
}
```
HTTP 201
[Captures]
plan2_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.billing_cycle" == "yearly"

# --- 3. GET /caris — FK capture: customer_id for subscription create ---
GET {{base_url}}/api/v1/caris
Authorization: Bearer {{admin_token}}
HTTP 200
[Captures]
customer_id: jsonpath "$.items[0].id"
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber
jsonpath "$.total" > 0

# --- 4. POST subscription (auth) — trial status, capture sub_id ---
POST {{base_url}}/api/v1/subscriptions
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "customer_id": {{customer_id}},
  "plan_id": {{plan_id}},
  "start_date": "2026-01-01",
  "end_date": null,
  "status": "trial",
  "auto_renew": true,
  "next_billing_date": "2026-02-01",
  "trial_start_date": "2026-01-01",
  "trial_end_date": "2026-01-15",
  "tenant_id": {{tenant_id}}
}
```
HTTP 201
[Captures]
sub_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.status" == "trial"
jsonpath "$.auto_renew" == true
jsonpath "$.plan_id" isNumber
jsonpath "$.customer_id" isNumber
jsonpath "$.start_date" exists
jsonpath "$.trial_start_date" exists
jsonpath "$.tenant_id" isNumber

# --- 5. GET by-id — DECODE FENCE: single-row SubscriptionResponse ---
GET {{base_url}}/api/v1/subscriptions/{{sub_id}}
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{sub_id}}
jsonpath "$.status" == "trial"
jsonpath "$.plan_id" isNumber
jsonpath "$.customer_id" isNumber
jsonpath "$.next_billing_date" exists
jsonpath "$.created_at" exists

# --- 6. GET list — DECODE FENCE: bare Vec (not paginated) ---
GET {{base_url}}/api/v1/subscriptions
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 7. PUT update — 200; auto_renew flipped to false ---
PUT {{base_url}}/api/v1/subscriptions/{{sub_id}}
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{ "auto_renew": false }
```
HTTP 200
[Asserts]
jsonpath "$.id" == {{sub_id}}
jsonpath "$.auto_renew" == false

# --- 8. POST trial/convert — 200 TrialConversionResult (trial → active) ---
POST {{base_url}}/api/v1/subscriptions/{{sub_id}}/trial/convert
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.subscription_id" == {{sub_id}}
jsonpath "$.previous_status" == "trial"
jsonpath "$.new_status" == "active"
jsonpath "$.next_billing_date" exists
jsonpath "$.billing_start_date" exists

# --- 9. POST renew — 200 (next_billing_date set by convert) ---
POST {{base_url}}/api/v1/subscriptions/{{sub_id}}/renew
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{sub_id}}
jsonpath "$.status" == "active"
jsonpath "$.last_billed_at" exists

# --- 10. GET invoices — capture invoice_id for dunning retry ---
GET {{base_url}}/api/v1/subscriptions/{{sub_id}}/invoices
Authorization: Bearer {{access_token}}
HTTP 200
[Captures]
invoice_id: jsonpath "$[0].id"
[Asserts]
jsonpath "$" isCollection
jsonpath "$[0].subscription_id" == {{sub_id}}
jsonpath "$[0].status" exists
jsonpath "$[0].amount" exists

# --- 11. POST proration — 200 ProrationResult (plan_id → plan2_id) ---
POST {{base_url}}/api/v1/subscriptions/{{sub_id}}/proration
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{ "new_plan_id": {{plan2_id}}, "effective_date": "2026-06-30" }
```
HTTP 200
[Asserts]
jsonpath "$.original_amount" exists
jsonpath "$.prorated_amount" exists
jsonpath "$.unused_days" isNumber
jsonpath "$.total_days" isNumber
jsonpath "$.refund_or_charge" exists
jsonpath "$.direction" exists

# --- 12. POST usage — 201 UsageRecordResponse ---
POST {{base_url}}/api/v1/subscriptions/{{sub_id}}/usage
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "quantity": 500,
  "unit": "api_calls",
  "billing_period_start": "2026-06-01",
  "billing_period_end": "2026-06-30"
}
```
HTTP 201
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.subscription_id" == {{sub_id}}
jsonpath "$.quantity" == 500
jsonpath "$.unit" == "api_calls"
jsonpath "$.record_type" exists

# --- 13. GET usage — 200 bare Vec decode ---
GET {{base_url}}/api/v1/subscriptions/{{sub_id}}/usage
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 14. GET dunning — 200 bare Vec (DunningEntryResponse decode; may be empty) ---
GET {{base_url}}/api/v1/subscriptions/{{sub_id}}/dunning
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 15. POST dunning retry — 200 DunningEntryResponse (uses captured invoice_id) ---
POST {{base_url}}/api/v1/subscriptions/{{sub_id}}/dunning
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{ "invoice_id": {{invoice_id}} }
```
HTTP 200
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.subscription_id" == {{sub_id}}
jsonpath "$.invoice_id" == {{invoice_id}}
jsonpath "$.attempt_number" isNumber
jsonpath "$.status" exists
jsonpath "$.created_at" exists

# --- 16. POST cancel — 200 CancellationResult (cancel at period end) ---
POST {{base_url}}/api/v1/subscriptions/{{sub_id}}/cancel
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{ "cancel_immediately": false, "reason": "hurl write-fence cleanup" }
```
HTTP 200
[Asserts]
jsonpath "$.subscription_id" == {{sub_id}}
jsonpath "$.status" == "cancelled"
jsonpath "$.cancelled_at" exists

# --- 17. GET due-for-billing — 200 bare Vec decode ---
GET {{base_url}}/api/v1/subscriptions/due-for-billing?date=2026-12-31
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 18. GET /subscriptions/trials — 404 (route shadowed by {id}; see header) ---
GET {{base_url}}/api/v1/subscriptions/trials
Authorization: Bearer {{access_token}}
HTTP 404
# Path-shadow 404 (/{id} registered before /trials) is actix plain-text, NOT
# the app JSON envelope — jsonpath "$.error" would fail "Invalid JSON body".
# Assert status only (matches sibling 49_subscriptions.hurl).

# --- 19. PUT plan A — 200; rename ---
PUT {{base_url}}/api/v1/subscription-plans/{{plan_id}}
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{ "name": "Hurl Plan A (renamed)", "is_active": false }
```
HTTP 200
[Asserts]
jsonpath "$.id" == {{plan_id}}
jsonpath "$.name" == "Hurl Plan A (renamed)"
jsonpath "$.is_active" == false

# --- 20. DELETE subscription — 200 MessageResponse (terminal soft-delete) ---
DELETE {{base_url}}/api/v1/subscriptions/{{sub_id}}
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.message" exists

# --- 21. GET by-id after delete — 404 ---
GET {{base_url}}/api/v1/subscriptions/{{sub_id}}
Authorization: Bearer {{access_token}}
HTTP 404

# --- 22. DELETE plan A — 200 MessageResponse ---
DELETE {{base_url}}/api/v1/subscription-plans/{{plan_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.message" exists

# --- 23. GET plan A by-id after delete — 404 (soft-deleted hidden) ---
GET {{base_url}}/api/v1/subscription-plans/{{plan_id}}
Authorization: Bearer {{admin_token}}
HTTP 404

# --- 24. DELETE plan B — 200 (cleanup so re-run doesn't 409 on plan B) ---
DELETE {{base_url}}/api/v1/subscription-plans/{{plan2_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.message" exists

# --- 25. RBAC: POST plan as regular user — 403 (AdminUser) ---
POST {{base_url}}/api/v1/subscription-plans
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{ "name": "forbidden", "billing_cycle": "monthly", "base_amount": 1.00, "tenant_id": {{tenant_id}} }
```
HTTP 403

# --- 26. No-auth POST plan — 401 ---
POST {{base_url}}/api/v1/subscription-plans
Content-Type: application/json
```
{ "name": "noauth", "billing_cycle": "monthly", "base_amount": 1.00, "tenant_id": {{tenant_id}} }
```
HTTP 401

# --- 27. No-auth POST subscription — 401 ---
POST {{base_url}}/api/v1/subscriptions
Content-Type: application/json
```
{ "customer_id": 1, "plan_id": 1, "start_date": "2026-01-01", "tenant_id": {{tenant_id}} }
```
HTTP 401