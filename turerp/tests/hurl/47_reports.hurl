# Tier 2 — Report generation. The bare /reports root returns a list
# (plain JSON array of generated reports); /queue is not a GET route
# (404). /generate is POST-only.

# --- 1. Reports list — 200, plain array ---
GET {{base_url}}/api/v1/reports
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 2. Reports queue — 404 (no GET at that path) ---
GET {{base_url}}/api/v1/reports/queue
Authorization: Bearer {{access_token}}
HTTP 404

# --- 3. Generate — POST-only, GET 404 ---
GET {{base_url}}/api/v1/reports/generate
Authorization: Bearer {{access_token}}
HTTP 404

# --- 4. No auth — 401 ---
GET {{base_url}}/api/v1/reports
HTTP 401