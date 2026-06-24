# Tier 1.5 — Exchange rates (currency submodule). Seeded
# (exchange_rates table), list MUST be non-empty.

# --- 1. Exchange rates list — seeded, non-empty ---
GET {{base_url}}/api/v1/exchange-rates
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber
jsonpath "$.total" > 0

# --- 2. No auth — 401 ---
GET {{base_url}}/api/v1/exchange-rates
HTTP 401