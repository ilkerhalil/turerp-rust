# 103_sales_quotations_write — WRITE-FENCE for the Sales Quotation surface.
#
# Covers: POST create → GET list (decode) → GET by-id (decode)
#         → PUT /{id}/status → GET /status/{status} → line mutations
#         (DELETE soft / GET lines/deleted / PUT restore / DELETE destroy)
#         → POST /{id}/convert (idempotent) → DELETE soft → GET 404
#         → PUT restore → GET 200 → DELETE /destroy → GET 404.
#
# Why: the read-only 29_sales.hurl only hits the (empty) list + 404s.
# Quotation create/by-id/status/convert/line/lifecycle decode paths stay
# hidden until seeded. This fence self-seeds a quotation + lines and
# exercises every transition including the convert-to-order path.
#
# RBAC: writes use AdminUser → testuser 403 / admin 200/201. Reads use
# AuthUser. No-auth → 401. run-all.sh SKIPS this file when admin_token
# is empty.
#
# FK STRATEGY: GET-capture cari_id (GET /caris items[0].id) + product_id
# (GET /products items[0].id). Quotation create returns QuotationResponse
# with a `lines` array → line ids captured from $.lines[N].id.
#
# ASSUMPTION: suite runs against a freshly-reset DB (no static unique
# code sent; quotation_number is server-generated).
# ASSUMPTION: cari_id / product_id seeded non-empty (seed_sample_data.sql).
# ASSUMPTION: rust_decimal serializes Decimal as a JSON string ("100.00").
# ASSUMPTION: QuotationStatus has no #[serde(rename_all)] → PascalCase
# variants ("Draft","Sent","UnderReview","Accepted","Rejected","Expired",
# "ConvertedToOrder"). valid_until must be a FUTURE RFC3339 timestamp
# (quotation validation: valid_until > now).
# ASSUMPTION: PUT /{id}/status returns the raw Quotation entity (NO lines
# array) — we assert status + id but not a lines field.
# ASSUMPTION: POST /{id}/convert takes NO body; returns SalesOrderResponse
# (with lines). On a quotation with sales_order_id already set, convert is
# idempotent and returns the existing linked order. The convert service
# links the quotation to the order (sales_order_id) but does NOT flip the
# quotation status to ConvertedToOrder (that is an explicit status the
# caller can set via PUT /status); we assert sales_order_id, not status.
# ASSUMPTION: company_id is REQUIRED on CreateQuotation (no serde default),
# so the body must include it.
# ASSUMPTION: soft-delete works on a quotation in any status (the service
# does not guard on status); we soft-delete after convert.

# --- 1. RBAC: regular user POST create → 403 ---
POST {{base_url}}/api/v1/sales/quotations
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "company_id": 1,
  "cari_id": 1,
  "valid_until": "2027-01-15T10:30:00Z",
  "lines": [
    {"description": "rbac", "quantity": "1", "unit_price": "1", "tax_rate": "0", "discount_rate": "0"}
  ]
}
```
HTTP 403

# --- 2. RBAC: no-auth POST create → 401 ---
POST {{base_url}}/api/v1/sales/quotations
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "company_id": 1,
  "cari_id": 1,
  "valid_until": "2027-01-15T10:30:00Z",
  "lines": [
    {"description": "noauth", "quantity": "1", "unit_price": "1", "tax_rate": "0", "discount_rate": "0"}
  ]
}
```
HTTP 401

# --- 3. GET /caris — capture cari_id ---
GET {{base_url}}/api/v1/caris
Authorization: Bearer {{admin_token}}
HTTP 200
[Captures]
cari_id: jsonpath "$.items[0].id"
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" > 0

# --- 4. GET /products — capture product_id ---
GET {{base_url}}/api/v1/products
Authorization: Bearer {{admin_token}}
HTTP 200
[Captures]
product_id: jsonpath "$.items[0].id"
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" > 0

# --- 5. Admin POST create (2 lines, future valid_until) → 201; capture ids ---
POST {{base_url}}/api/v1/sales/quotations
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "company_id": 1,
  "cari_id": {{cari_id}},
  "valid_until": "2027-01-15T10:30:00Z",
  "notes": "hurl fence quotation",
  "terms": "Payment within 30 days",
  "lines": [
    {"product_id": {{product_id}}, "description": "Hurl quote line 0", "quantity": "2", "unit_price": "100.00", "tax_rate": "18", "discount_rate": "0"},
    {"description": "Hurl quote line 1", "quantity": "1", "unit_price": "50.00", "tax_rate": "18", "discount_rate": "0"}
  ]
}
```
HTTP 201
[Captures]
quotation_id: jsonpath "$.id"
line0_id: jsonpath "$.lines[0].id"
line1_id: jsonpath "$.lines[1].id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.quotation_number" isString
jsonpath "$.cari_id" == {{cari_id}}
jsonpath "$.status" isString
jsonpath "$.valid_until" exists
jsonpath "$.subtotal" exists
jsonpath "$.total_amount" exists
jsonpath "$.lines" isCollection
jsonpath "$.lines[0].id" isNumber
jsonpath "$.lines[1].id" isNumber
jsonpath "$.company_id" isNumber

# --- 6. GET list (admin) — DECODE FENCE: paginated Quotation entity items ---
GET {{base_url}}/api/v1/sales/quotations
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber
jsonpath "$.total" > 0
jsonpath "$.items[?(@.id=={{quotation_id}})].quotation_number" exists

# --- 7. GET by-id (admin) — DECODE FENCE: QuotationResponse with lines ---
GET {{base_url}}/api/v1/sales/quotations/{{quotation_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{quotation_id}}
jsonpath "$.cari_id" == {{cari_id}}
jsonpath "$.status" isString
jsonpath "$.valid_until" exists
jsonpath "$.lines" isCollection
jsonpath "$.lines[0].id" == {{line0_id}}
jsonpath "$.lines[1].id" == {{line1_id}}
jsonpath "$.company_id" isNumber
jsonpath "$.sales_order_id" exists

# --- 8. PUT /{id}/status "Sent" (admin) → 200 raw Quotation entity (no lines) ---
PUT {{base_url}}/api/v1/sales/quotations/{{quotation_id}}/status
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{"status": "Sent"}
```
HTTP 200
[Asserts]
jsonpath "$.id" == {{quotation_id}}
jsonpath "$.status" == "Sent"

# --- 9. GET /status/Sent — paginated; our quotation present ---
GET {{base_url}}/api/v1/sales/quotations/status/Sent
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.items[?(@.id=={{quotation_id}})].status" contains "Sent"

# --- 10. Line: DELETE soft (admin) → 200 MessageResponse ---
DELETE {{base_url}}/api/v1/sales/quotations/{{quotation_id}}/lines/{{line0_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.message" exists

# --- 11. Line: GET /quotations/{id}/lines/deleted (admin) — bare Vec with line0 ---
GET {{base_url}}/api/v1/sales/quotations/{{quotation_id}}/lines/deleted
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection
jsonpath "$[0].id" == {{line0_id}}

# --- 12. Line: PUT restore (admin) → 200 QuotationLine ---
PUT {{base_url}}/api/v1/sales/quotations/{{quotation_id}}/lines/{{line0_id}}/restore
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{line0_id}}
jsonpath "$.quotation_id" == {{quotation_id}}

# --- 13. Line: DELETE destroy (admin) → 204 (permanent) ---
DELETE {{base_url}}/api/v1/sales/quotations/{{quotation_id}}/lines/{{line0_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 14. GET by-id → 200 (lines now has 1 = line1; line0 destroyed) ---
GET {{base_url}}/api/v1/sales/quotations/{{quotation_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{quotation_id}}
jsonpath "$.lines" isCollection
jsonpath "$.lines[?(@.id=={{line1_id}})]" exists

# --- 15. POST /{id}/convert (admin, no body) → 200 SalesOrderResponse; capture order_id ---
POST {{base_url}}/api/v1/sales/quotations/{{quotation_id}}/convert
Authorization: Bearer {{admin_token}}
HTTP 200
[Captures]
converted_order_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.order_number" isString
jsonpath "$.cari_id" == {{cari_id}}
jsonpath "$.lines" isCollection

# --- 16. GET quotation by-id → sales_order_id now linked to converted order ---
GET {{base_url}}/api/v1/sales/quotations/{{quotation_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{quotation_id}}
jsonpath "$.sales_order_id" == {{converted_order_id}}

# --- 17. POST /{id}/convert again → idempotent, returns the SAME order ---
POST {{base_url}}/api/v1/sales/quotations/{{quotation_id}}/convert
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{converted_order_id}}

# --- 18. GET /quotations/deleted (admin) — bare Vec decode fence (empty pre-delete) ---
GET {{base_url}}/api/v1/sales/quotations/deleted
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 19. RBAC: regular user DELETE quotation → 403 ---
DELETE {{base_url}}/api/v1/sales/quotations/{{quotation_id}}
Authorization: Bearer {{access_token}}
HTTP 403

# --- 20. DELETE soft (admin) → 200 MessageResponse ---
DELETE {{base_url}}/api/v1/sales/quotations/{{quotation_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.message" exists

# --- 21. GET by-id → 404 (soft-deleted hidden) ---
GET {{base_url}}/api/v1/sales/quotations/{{quotation_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 22. PUT restore (admin) → 200 QuotationResponse ---
PUT {{base_url}}/api/v1/sales/quotations/{{quotation_id}}/restore
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{quotation_id}}
jsonpath "$.cari_id" == {{cari_id}}
jsonpath "$.lines" isCollection

# --- 23. GET by-id after restore → 200 ---
GET {{base_url}}/api/v1/sales/quotations/{{quotation_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{quotation_id}}

# --- 24. DELETE /destroy (admin) → 204 ---
DELETE {{base_url}}/api/v1/sales/quotations/{{quotation_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 25. GET by-id after destroy → 404 ---
GET {{base_url}}/api/v1/sales/quotations/{{quotation_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 26. RBAC: no-auth GET by-id → 401 ---
GET {{base_url}}/api/v1/sales/quotations/999999
HTTP 401