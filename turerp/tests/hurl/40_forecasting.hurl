# Tier 2 — Inventory forecasting. demand/reorder/alerts are POST-only
# (compute endpoints); the only GET is /report.

# --- 1. Forecast report — 200 (GET) ---
GET {{base_url}}/api/v1/forecasting/report
Authorization: Bearer {{access_token}}
HTTP 200

# --- 2. Demand forecast — POST-only, GET 405 ---
GET {{base_url}}/api/v1/forecasting/demand
Authorization: Bearer {{access_token}}
HTTP 405

# --- 3. Reorder suggestions — POST-only, GET 405 ---
GET {{base_url}}/api/v1/forecasting/reorder
Authorization: Bearer {{access_token}}
HTTP 405

# --- 4. Stock alerts — POST-only, GET 405 ---
GET {{base_url}}/api/v1/forecasting/alerts
Authorization: Bearer {{access_token}}
HTTP 405

# --- 5. No auth — 401 ---
GET {{base_url}}/api/v1/forecasting/report
HTTP 401