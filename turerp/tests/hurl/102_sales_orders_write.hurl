# 102_sales_orders_write — WRITE-FENCE for the Sales Order mutation surface.
#
# Covers: POST create → GET list (decode) → GET by-id (decode)
#         → PUT /{id}/status → GET /status/{status} → line mutations
#         (DELETE soft / GET lines/deleted / PUT restore / DELETE destroy)
#         → DELETE soft → GET 404 → PUT restore → GET 200 → DELETE /destroy
#         → GET 404.
#
# Why: the read-only 29_sales.hurl only hits the (empty) list + 404s, so
# Row decode bugs in the create/by-id/status/line/lifecycle paths stay
# hidden until seeded. This fence self-seeds an order + lines and exercises
# every lifecycle transition.
#
# RBAC: writes (create/update-status/delete/restore/destroy/line-mutations)
# use the AdminUser extractor → testuser ({{access_token}}) gets 403, admin
# ({{admin_token}}) gets 200/201. Reads use AuthUser. No-auth → 401.
# run-all.sh SKIPS this file when admin_token is empty.
#
# FK STRATEGY: caris + products are seeded (seed_sample_data.sql). GET-capture
# a cari_id (GET /caris items[0].id) and a product_id (GET /products
# items[0].id). The order create returns SalesOrderResponse with a `lines`
# array, so line ids are captured from $.lines[N].id (no separate POST for
# lines — lines are created inline with the order).
#
# ASSUMPTION: suite runs against a freshly-reset DB. Re-run safety relies
# on DB reset between full-suite runs (no static unique code is sent —
# order_number is server-generated).
# ASSUMPTION: cari_id / product_id from GET /caris / /products are non-empty
# (seed_sample_data seeds them; per 13_crm_customers.hurl + 23_products.hurl
# $.total > 0). If a seed is absent the order create FK-fails — flagged.
# ASSUMPTION: rust_decimal serializes Decimal as a JSON string ("100.00")
# in both request and response (rust_decimal features=["serde"], no
# serde-float). Send amounts as strings.
# ASSUMPTION: SalesOrderStatus has no #[serde(rename_all)] → PascalCase
# variants ("Draft", "PendingApproval", "Approved", "InProgress",
# "Shipped", "Delivered", "Cancelled", "OnHold"). PUT /status body sends
# the variant as-is.
# ASSUMPTION: PUT /{id}/status returns the raw SalesOrder entity (NO lines
# array) — we assert status + id but do NOT assert a lines field exists.
# ASSUMPTION: create returns status (some default, likely "Draft"); we
# assert it is a string, not an exact value, to avoid coupling to default.
# ASSUMPTION: line destroy permanently removes the line; the order still
# exists with remaining lines. Subsequent order soft-delete cascades to
# the remaining (non-destroyed) lines; restore brings them back; the
# destroyed line stays gone.

# --- 1. RBAC: regular user POST create → 403 ---
POST {{base_url}}/api/v1/sales/orders
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "cari_id": 1,
  "order_date": "2026-06-01T10:00:00Z",
  "lines": [
    {"description": "rbac reject", "quantity": "1", "unit_price": "1", "tax_rate": "0", "discount_rate": "0"}
  ]
}
```
HTTP 403

# --- 2. RBAC: no-auth POST create → 401 ---
POST {{base_url}}/api/v1/sales/orders
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "cari_id": 1,
  "order_date": "2026-06-01T10:00:00Z",
  "lines": [
    {"description": "noauth", "quantity": "1", "unit_price": "1", "tax_rate": "0", "discount_rate": "0"}
  ]
}
```
HTTP 401

# --- 3. GET /caris — capture a seeded cari_id ---
GET {{base_url}}/api/v1/caris
Authorization: Bearer {{admin_token}}
HTTP 200
[Captures]
cari_id: jsonpath "$.items[0].id"
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber
jsonpath "$.total" > 0

# --- 4. GET /products — capture a seeded product_id ---
GET {{base_url}}/api/v1/products
Authorization: Bearer {{admin_token}}
HTTP 200
[Captures]
product_id: jsonpath "$.items[0].id"
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber
jsonpath "$.total" > 0

# --- 5. Admin POST create (2 lines) → 201; capture order_id + both line ids ---
POST {{base_url}}/api/v1/sales/orders
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "cari_id": {{cari_id}},
  "order_date": "2026-06-01T10:00:00Z",
  "delivery_date": "2026-06-15T10:00:00Z",
  "notes": "hurl fence order",
  "lines": [
    {"product_id": {{product_id}}, "description": "Hurl fence line 0", "quantity": "2", "unit_price": "100.00", "tax_rate": "18", "discount_rate": "0"},
    {"description": "Hurl fence line 1 (no product)", "quantity": "1", "unit_price": "50.00", "tax_rate": "18", "discount_rate": "0"}
  ]
}
```
HTTP 201
[Captures]
order_id: jsonpath "$.id"
line0_id: jsonpath "$.lines[0].id"
line1_id: jsonpath "$.lines[1].id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.order_number" isString
jsonpath "$.cari_id" == {{cari_id}}
jsonpath "$.status" isString
jsonpath "$.order_date" exists
jsonpath "$.lines" isCollection
jsonpath "$.lines[0].id" isNumber
jsonpath "$.lines[1].id" isNumber
jsonpath "$.lines[0].line_total" exists
jsonpath "$.company_id" isNumber

# --- 6. GET list (admin) — DECODE FENCE: paginated SalesOrder entity items ---
GET {{base_url}}/api/v1/sales/orders
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber
jsonpath "$.total" > 0
jsonpath "$.items[?(@.id=={{order_id}})].order_number" exists

# --- 7. GET by-id (admin) — DECODE FENCE: SalesOrderResponse with lines ---
GET {{base_url}}/api/v1/sales/orders/{{order_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{order_id}}
jsonpath "$.cari_id" == {{cari_id}}
jsonpath "$.status" isString
jsonpath "$.subtotal" exists
jsonpath "$.tax_amount" exists
jsonpath "$.total_amount" exists
jsonpath "$.lines" isCollection
jsonpath "$.lines[0].id" == {{line0_id}}
jsonpath "$.lines[1].id" == {{line1_id}}
jsonpath "$.company_id" isNumber

# --- 8. PUT /{id}/status "Approved" (admin) → 200 raw SalesOrder entity (no lines) ---
PUT {{base_url}}/api/v1/sales/orders/{{order_id}}/status
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{"status": "Approved"}
```
HTTP 200
[Asserts]
jsonpath "$.id" == {{order_id}}
jsonpath "$.status" == "Approved"
jsonpath "$.cari_id" == {{cari_id}}

# --- 9. GET /status/Approved — paginated; our order present ---
GET {{base_url}}/api/v1/sales/orders/status/Approved
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.items[?(@.id=={{order_id}})].status" contains "Approved"

# --- 10. Line: DELETE soft (admin) → 200 MessageResponse ---
DELETE {{base_url}}/api/v1/sales/orders/{{order_id}}/lines/{{line0_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.message" exists

# --- 11. Line: GET /orders/{id}/lines/deleted (admin) — bare Vec with line0 ---
GET {{base_url}}/api/v1/sales/orders/{{order_id}}/lines/deleted
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection
jsonpath "$[0].id" == {{line0_id}}

# --- 12. Line: PUT restore (admin) → 200 SalesOrderLine ---
PUT {{base_url}}/api/v1/sales/orders/{{order_id}}/lines/{{line0_id}}/restore
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{line0_id}}
jsonpath "$.order_id" == {{order_id}}

# --- 13. Line: DELETE destroy (admin) → 204 (permanent) ---
DELETE {{base_url}}/api/v1/sales/orders/{{order_id}}/lines/{{line0_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 14. GET /orders/deleted (admin) — bare Vec decode fence (empty pre-delete) ---
GET {{base_url}}/api/v1/sales/orders/deleted
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 15. RBAC: regular user DELETE order → 403 ---
DELETE {{base_url}}/api/v1/sales/orders/{{order_id}}
Authorization: Bearer {{access_token}}
HTTP 403

# --- 16. DELETE soft (admin) → 200 MessageResponse ---
DELETE {{base_url}}/api/v1/sales/orders/{{order_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.message" exists

# --- 17. GET by-id → 404 (soft-deleted hidden) ---
GET {{base_url}}/api/v1/sales/orders/{{order_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 18. PUT restore (admin) → 200 SalesOrderResponse ---
PUT {{base_url}}/api/v1/sales/orders/{{order_id}}/restore
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{order_id}}
jsonpath "$.cari_id" == {{cari_id}}
jsonpath "$.lines" isCollection

# --- 19. GET by-id after restore → 200 (line1 still present; line0 destroyed) ---
GET {{base_url}}/api/v1/sales/orders/{{order_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{order_id}}
jsonpath "$.lines" isCollection
jsonpath "$.lines[?(@.id=={{line1_id}})]" exists

# --- 20. DELETE /destroy (admin) → 204 ---
DELETE {{base_url}}/api/v1/sales/orders/{{order_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 21. GET by-id after destroy → 404 ---
GET {{base_url}}/api/v1/sales/orders/{{order_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 22. RBAC: no-auth GET by-id → 401 ---
GET {{base_url}}/api/v1/sales/orders/999999
HTTP 401