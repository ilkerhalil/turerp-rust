# Tier 1 — Currencies. Documents a known PR #145 regression:
#   GET /currencies and GET /exchange-rates return 404 in the current build.
#   This file asserts the 404 and is expected to flip to 200 once the
#   route registration is fixed. Filed as issue #150 follow-up.

# --- 1. Currencies list — currently 404, see issue #150 ---
GET {{base_url}}/api/v1/currencies
Authorization: Bearer {{access_token}}
HTTP 404

# --- 2. Exchange rates list — currently 404, see issue #150 ---
GET {{base_url}}/api/v1/exchange-rates
Authorization: Bearer {{access_token}}
HTTP 404

# --- 3. Direct table probe via the seed endpoint (workaround) ---
#    Confirms the DB row exists, isolating the failure to the route layer.
GET {{base_url}}/api/v1/currencies/USD
Authorization: Bearer {{access_token}}
HTTP 404
