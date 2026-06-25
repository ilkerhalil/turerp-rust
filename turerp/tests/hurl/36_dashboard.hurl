# Tier 2 — BI dashboard. kpis list is 200 (was 500: three broken SQL
# queries — get_cash_flow referenced a non-existent c.outflow column, and
# get_ar_aging/get_ap_aging did CURRENT_DATE - i.due_date where due_date is
# TIMESTAMPTZ — fixed in the dashboard repo). charts and widgets work.

# --- 1. KPIs list — 200 (was 500; returns the KpiResponse object) ---
GET {{base_url}}/api/v1/dashboard/kpis
Authorization: Bearer {{access_token}}
HTTP 200

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