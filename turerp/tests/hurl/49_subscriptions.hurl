# Tier 2 — SaaS subscriptions & plans. Both list endpoints are currently
# 500 (broken — follow-up to #152); /trials is 404. Assert the broken
# status so the bugs stay visible until fixed.

# --- 1. Subscription plans — BROKEN (500), assert until fixed ---
GET {{base_url}}/api/v1/subscription-plans
Authorization: Bearer {{access_token}}
HTTP 500

# --- 2. Subscriptions — BROKEN (500), assert until fixed ---
GET {{base_url}}/api/v1/subscriptions
Authorization: Bearer {{access_token}}
HTTP 500

# --- 3. Trials — 404 (no GET at that path) ---
GET {{base_url}}/api/v1/subscriptions/trials
Authorization: Bearer {{access_token}}
HTTP 404

# --- 4. No auth — 401 ---
GET {{base_url}}/api/v1/subscriptions
HTTP 401