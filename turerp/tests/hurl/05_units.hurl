# Tier 1 — Units. Same P1 as categories: GET /units returns 500.
# Asserted to make the regression deterministic.


# --- 1. Units list — currently 500 (DB error) ---
GET {{base_url}}/api/v1/units
Authorization: Bearer {{access_token}}
HTTP 500
[Asserts]
jsonpath "$.error" exists
