# Tier 1 — Categories. Documents known P1: GET /categories returns 500
# (errors.database_error). Asserts the 500 to surface the regression
# deterministically rather than letting it slip silently.


# --- 1. Categories list — currently 500 (DB error) ---
GET {{base_url}}/api/v1/categories
Authorization: Bearer {{access_token}}
HTTP 500
[Asserts]
jsonpath "$.error" exists
