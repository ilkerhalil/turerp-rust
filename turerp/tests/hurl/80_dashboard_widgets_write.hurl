# 80_dashboard_widgets_write — WRITE-FENCE for dashboard widget configs.
#
# Covers POST /dashboard/widgets (admin) + GET list (auth) + DELETE
# /dashboard/widgets/{id} (admin). The read-only 36_dashboard.hurl only
# hits the (empty) widgets list; the create + delete Row decode paths
# (DashboardWidgetConfig with WidgetType enum + WidgetPosition nested
# object + DashboardFilter) are never exercised.
#
# Why: the create response and list decode carry the WidgetType enum
# (serde default = PascalCase: "Kpi", "LineChart", ...) and the nested
# WidgetPosition object — a decode bug there stays hidden until a widget
# is seeded. This fence self-seeds a throwaway widget, walks create → list
# → delete, and confirms the 404-on-re-get path.
#
# RBAC: create + delete use AdminUser → testuser ({{access_token}}) gets 403,
# admin ({{admin_token}}) gets 201/204. list uses AuthUser → any authed 200;
# no-auth → 401. run-all.sh SKIPS this file when admin_token is empty.
#
# ASSUMPTION: WidgetType has NO #[serde(rename_all)] → serde serializes the
# variant names in PascalCase: "Kpi", "LineChart", "BarChart", "PieChart",
# "Table", "Gauge". The lowercase strings are only the Display/FromStr impls
# (query-string / CLI), NOT JSON bodies. Sending "kpi" will 400. Verified
# against src/domain/dashboard/model.rs.
# ASSUMPTION: create + delete are idempotent across runs (no unique natural
# key on widget title), so re-runs won't 409. The DELETE is a hard delete
# (204, no soft-delete/restore/destroy routes for widgets), so step 4's
# re-get is 404.
# ASSUMPTION: filter is Option<DashboardFilter> with all-optional fields,
# so null is accepted; position is required (x/y/w/h all i32).
# ASSUMPTION: save_widget returns DashboardWidgetConfig with id (i64),
# tenant_id, widget_type, title, position, filter, created_at, updated_at.

# --- 1. POST widget (admin) → 201; capture widget_id ---
POST {{base_url}}/api/v1/dashboard/widgets
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "widget_type": "Kpi",
  "title": "Hurl Revenue KPI",
  "position": { "x": 0, "y": 0, "w": 4, "h": 2 },
  "filter": null
}
```
HTTP 201
[Captures]
widget_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.tenant_id" isNumber
jsonpath "$.widget_type" == "Kpi"
jsonpath "$.title" == "Hurl Revenue KPI"
jsonpath "$.position.x" isNumber
jsonpath "$.position.y" isNumber
jsonpath "$.position.w" isNumber
jsonpath "$.position.h" isNumber
jsonpath "$.created_at" exists
jsonpath "$.updated_at" exists

# --- 2. POST second widget with a filter (admin) → 201; exercise DashboardFilter decode ---
POST {{base_url}}/api/v1/dashboard/widgets
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "widget_type": "LineChart",
  "title": "Hurl Sales Chart",
  "position": { "x": 4, "y": 0, "w": 8, "h": 4 },
  "filter": {
    "date_from": "2026-01-01T00:00:00Z",
    "date_to": "2026-12-31T23:59:59Z",
    "company_id": 1,
    "branch_id": null,
    "product_category": null
  }
}
```
HTTP 201
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.widget_type" == "LineChart"
jsonpath "$.title" == "Hurl Sales Chart"
jsonpath "$.filter.date_from" exists
jsonpath "$.filter.company_id" isNumber

# --- 3. GET list (auth) — DECODE FENCE: bare Vec, self-seeded rows present ---
GET {{base_url}}/api/v1/dashboard/widgets
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection
jsonpath "$[?(@.id=={{widget_id}})].title" contains "Hurl Revenue KPI"
jsonpath "$[?(@.id=={{widget_id}})].widget_type" contains "Kpi"

# --- 4. DELETE widget (admin) → 204 (hard delete) ---
DELETE {{base_url}}/api/v1/dashboard/widgets/{{widget_id}}
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 5. GET list — deleted row gone (no re-get-by-id route exists; list is the fence) ---
GET {{base_url}}/api/v1/dashboard/widgets
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection
jsonpath "$[?(@.id=={{widget_id}})]" not exists

# --- 6. DELETE non-existent widget → 404 ---
DELETE {{base_url}}/api/v1/dashboard/widgets/999999
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 7. RBAC: POST widget as regular user → 403 (AdminUser) ---
POST {{base_url}}/api/v1/dashboard/widgets
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "widget_type": "BarChart",
  "title": "forbidden",
  "position": { "x": 0, "y": 0, "w": 4, "h": 2 },
  "filter": null
}
```
HTTP 403

# --- 8. RBAC: DELETE widget as regular user → 403 (AdminUser) ---
DELETE {{base_url}}/api/v1/dashboard/widgets/999999
Authorization: Bearer {{access_token}}
HTTP 403

# --- 9. No-auth POST widget → 401 ---
POST {{base_url}}/api/v1/dashboard/widgets
Content-Type: application/json
```
{
  "widget_type": "Kpi",
  "title": "no auth",
  "position": { "x": 0, "y": 0, "w": 4, "h": 2 },
  "filter": null
}
```
HTTP 401

# --- 10. No-auth GET list → 401 ---
GET {{base_url}}/api/v1/dashboard/widgets
HTTP 401