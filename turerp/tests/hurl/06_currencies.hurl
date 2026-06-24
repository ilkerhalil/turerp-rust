# Tier 1 — Currencies. PR #145 broke the route registration; the
# configure() used web::scope("/currencies") mounted under /api,
# exposing GET /api/currencies (not /api/v1/currencies). Issue #152
# flipped the prefix to /v1/... to align with the utoipa paths and
# the GlobalGate rule, so the route is now reachable at the v1 path.

# --- 1. Currencies list — fixed by #152 ---
GET {{base_url}}/api/v1/currencies
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber

# --- 2. Exchange rates list — fixed by #152 ---
GET {{base_url}}/api/v1/exchange-rates
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber

# --- 3. Specific currency — fixed by #152, seeded by scripts/seed_sample_data.sql ---
# USD is seeded for tenant 1 (base currency), so the handler now returns
# the row. The route itself was unreachable before #152 (missing-route
# 404); after the fix + seed it returns 200 with the currency object.
GET {{base_url}}/api/v1/currencies/USD
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.code" == "USD"
jsonpath "$.is_base" isBoolean
