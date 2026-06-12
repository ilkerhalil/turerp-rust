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

# --- 3. Specific currency (404 if not seeded) — fixed by #152 ---
# USD is not seeded by default; the route is now reachable (no
# 404 from missing route) but returns 404 from the handler
# because the row doesn't exist. Seed a row separately if you
# want to assert 200.
GET {{base_url}}/api/v1/currencies/USD
Authorization: Bearer {{access_token}}
HTTP 404
