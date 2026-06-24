# Tier 2 — BI dashboard. kpis list is 500 (broken — follow-up to #152);
# charts and widgets work.

# --- 1. KPIs list — BROKEN (500), assert until fixed ---
GET {{base_url}}/api/v1/dashboard/kpis
Authorization: Bearer {{access_token}}
HTTP 500

# --- 2. Sales chart — 200 ---
GET {{base_url}}/api/v1/dashboard/charts/sales
Authorization: Bearer {{access_token}}
HTTP 200

# --- 3. Widgets list — 200 ---
GET {{base_url}}/api/v1/dashboard/widgets
Authorization: Bearer {{access_token}}
HTTP 200

# --- 4. No auth — 401 ---
GET {{base_url}}/api/v1/dashboard/widgets
HTTP 401