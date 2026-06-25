# Tier 2 — SaaS subscriptions & plans. Both list endpoints are 200 (were
# 500 — subscription_plans and subscriptions, created in migration 026,
# lacked the deleted_at/deleted_by soft-delete columns AND the feature
# columns the Row structs SELECT: subscription_plans missed
# included_quantity/overage_rate; subscriptions missed
# trial_start_date/trial_end_date. Fixed by migration 045, mirroring the
# 040/044 soft-delete pattern). /trials is 404 (no GET at that path).

# --- 1. Subscription plans — 200 (was 500; plain JSON array) ---
GET {{base_url}}/api/v1/subscription-plans
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 2. Subscriptions — 200 (was 500; plain JSON array) ---
GET {{base_url}}/api/v1/subscriptions
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 3. Trials — 404 (no GET at that path) ---
GET {{base_url}}/api/v1/subscriptions/trials
Authorization: Bearer {{access_token}}
HTTP 404

# --- 4. No auth — 401 ---
GET {{base_url}}/api/v1/subscriptions
HTTP 401