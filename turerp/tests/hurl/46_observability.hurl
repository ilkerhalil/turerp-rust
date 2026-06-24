# Tier 2 — Observability (health, SLI/SLO, alerts). Read endpoints are
# open to a regular user. slos returns a plain JSON array.

# --- 1. Observability health — 200 ---
GET {{base_url}}/api/v1/observability/health
Authorization: Bearer {{access_token}}
HTTP 200

# --- 2. SLOs — 200, plain array ---
GET {{base_url}}/api/v1/observability/slos
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 3. SLIs — 200 ---
GET {{base_url}}/api/v1/observability/slis
Authorization: Bearer {{access_token}}
HTTP 200

# --- 4. Alerts — 200 ---
GET {{base_url}}/api/v1/observability/alerts
Authorization: Bearer {{access_token}}
HTTP 200

# --- 5. Alert rules — 200 ---
GET {{base_url}}/api/v1/observability/alert-rules
Authorization: Bearer {{access_token}}
HTTP 200

# --- 6. Aspire dashboard URL — 200 ---
GET {{base_url}}/api/v1/observability/dashboard
Authorization: Bearer {{access_token}}
HTTP 200

# --- 7. No auth — 401 ---
GET {{base_url}}/api/v1/observability/slos
HTTP 401