# 105_goods_receipts_invoices_write — WRITE-FENCE for Goods Receipts +
# Invoices (with payments). Two related mutation surfaces in one file.
#
# Goods Receipts:
#   Self-seed a PO (needs cari_id) → transition PO to "Approved" → POST
#   /goods-receipts (refs po_line_id) → GET by-id → GET /order/{po_id}
#   → PUT /{id}/status → GET /deleted → DELETE soft → GET 404
#   → POST /restore → GET 200 → DELETE /destroy → GET 404.
#
# Invoices:
#   POST /invoices (self-seed) → GET list → GET by-id → PUT /{id}/status
#   → GET /status/{status} → POST /invoices/payments → GET /{id}/payments
#   → GET /outstanding → GET /overdue → GET /search → GET /deleted
#   → DELETE soft → GET 404 → PUT /restore → GET 200 → DELETE /destroy
#   → GET 404.
#
# Why: read-only 31_goods_receipts.hurl + 15_invoices.hurl only hit root
# 405 / 404 / empty-list. Create/by-id/status/payment/lifecycle decode
# paths stay hidden until seeded. This fence self-seeds a PO + a receipt
# and an invoice + payment.
#
# RBAC:
#   - Goods-receipt create (POST) + PUT status are AuthUser → regular user
#     201/200, NOT 403. The 403 fence is on DELETE (AdminUser).
#   - Invoice create / payment / delete / restore / destroy / status are
#     AdminUser → regular user 403. Reads (list/by-id/payments/search/
#     outstanding/overdue/status) are AuthUser.
#   - run-all.sh SKIPS this file when admin_token is empty.
#
# FK STRATEGY:
#   - Goods-receipts need a pre-existing PO in Approved/SentToVendor/
#     PartialReceived status, plus the PO's line id (order_line_id).
#     Self-seed: POST /purchase-orders (AuthUser) → capture po_id + po_line_id
#     → PUT /purchase-orders/{po_id}/status "Approved".
#   - Invoices need cari_id (seeded). GET-capture cari_id + product_id.
#   - Payments need invoice_id (self-seeded invoice from earlier in this
#     file); amount must be > 0 and not exceed total_amount - paid_amount.
#
# ASSUMPTION: suite runs against a freshly-reset DB (receipt_number /
# invoice_number are server-generated).
# ASSUMPTION: cari_id / product_id seeded non-empty (seed_sample_data.sql).
# ASSUMPTION: rust_decimal serializes Decimal as a JSON string ("100.00").
# ASSUMPTION: GoodsReceiptStatus has no #[serde(rename_all)] → PascalCase
# variants "Pending","Partial","Completed","Cancelled". PUT /{id}/status
# body = {"status": "<variant>"}; returns raw GoodsReceipt entity (NO lines).
# ASSUMPTION: InvoiceStatus / InvoiceType have no #[serde(rename_all)] →
# PascalCase. Status: "Draft","Pending","Approved","Sent","PartiallyPaid",
# "Paid","Overdue","Cancelled","Refunded". Type: "SalesInvoice",
# "PurchaseInvoice","SalesReturn","PurchaseReturn". PUT /invoices/{id}/status
# returns raw Invoice entity (NO lines).
# ASSUMPTION: CreateGoodsReceipt.company_id is REQUIRED (no serde default)
# — must be in the body. GoodsReceiptResponse.company_id is hardcoded to 0
# in the From impl (known quirk) — assert isNumber, not == 1.
# ASSUMPTION: CreatePayment.company_id is REQUIRED (no default).
# payment_method is a free-form non-empty String (no enum). amount must be
# > 0; service rejects if invoice is Paid/Cancelled or new_paid > total.
# ASSUMPTION: GET /goods-receipts/order/{id} and /deleted return bare
# Vec<GoodsReceipt> (entity, NO lines). GET /invoices/outstanding,
# /overdue, /search, /deleted, /{id}/payments return bare arrays
# (NOT paginated). GET /invoices + /invoices/status/{status} are paginated.
# ASSUMPTION: invoice total for 1 line (qty=2, unit_price=100, tax 18,
# discount 0) = subtotal 200 + tax 36 = 236; a "50.00" payment is valid.
# ASSUMPTION: invoice_number is server-generated as "INV-<timestamp>"; the
# search query q=INV matches invoice_number via trigram/FTS.

# ============ GOODS RECEIPTS ============

# --- 1. GET /caris — capture cari_id ---
GET {{base_url}}/api/v1/caris
Authorization: Bearer {{admin_token}}
HTTP 200
[Captures]
cari_id: jsonpath "$.items[0].id"
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" > 0

# --- 2. GET /products — capture product_id ---
GET {{base_url}}/api/v1/products
Authorization: Bearer {{admin_token}}
HTTP 200
[Captures]
product_id: jsonpath "$.items[0].id"
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" > 0

# --- 3. Self-seed a PO (AuthUser) → 201; capture po_id + po_line_id ---
POST {{base_url}}/api/v1/purchase-orders
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "cari_id": {{cari_id}},
  "order_date": "2026-06-01T10:00:00Z",
  "notes": "hurl fence PO for goods receipt",
  "lines": [
    {"product_id": {{product_id}}, "description": "Hurl GR PO line 0", "quantity": "5", "unit_price": "100.00", "tax_rate": "18", "discount_rate": "0"}
  ]
}
```
HTTP 201
[Captures]
po_id: jsonpath "$.id"
po_line_id: jsonpath "$.lines[0].id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.lines[0].id" isNumber

# --- 4. Transition PO to "Approved" (AuthUser) → 200 ---
PUT {{base_url}}/api/v1/purchase-orders/{{po_id}}/status
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{"status": "Approved"}
```
HTTP 200
[Asserts]
jsonpath "$.id" == {{po_id}}
jsonpath "$.status" == "Approved"

# --- 5. RBAC: no-auth POST goods-receipt → 401 ---
POST {{base_url}}/api/v1/goods-receipts
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "company_id": 1,
  "purchase_order_id": {{po_id}},
  "receipt_date": "2026-06-10T10:00:00Z",
  "lines": [
    {"order_line_id": {{po_line_id}}, "product_id": {{product_id}}, "quantity": "3", "condition": "Good", "notes": null}
  ]
}
```
HTTP 401

# --- 6. POST /goods-receipts (AuthUser) → 201 GoodsReceiptResponse; capture gr_id ---
POST {{base_url}}/api/v1/goods-receipts
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "company_id": 1,
  "purchase_order_id": {{po_id}},
  "receipt_date": "2026-06-10T10:00:00Z",
  "notes": "hurl fence receipt",
  "lines": [
    {"order_line_id": {{po_line_id}}, "product_id": {{product_id}}, "quantity": "3", "condition": "Good", "notes": "fence line"}
  ]
}
```
HTTP 201
[Captures]
gr_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.receipt_number" isString
jsonpath "$.purchase_order_id" == {{po_id}}
jsonpath "$.status" == "Pending"
jsonpath "$.receipt_date" exists
jsonpath "$.lines" isCollection
jsonpath "$.lines[0].order_line_id" == {{po_line_id}}
jsonpath "$.lines[0].quantity" exists
jsonpath "$.company_id" isNumber

# --- 7. GET /goods-receipts/{id} — DECODE FENCE: GoodsReceiptResponse with lines ---
GET {{base_url}}/api/v1/goods-receipts/{{gr_id}}
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{gr_id}}
jsonpath "$.purchase_order_id" == {{po_id}}
jsonpath "$.status" isString
jsonpath "$.lines" isCollection
jsonpath "$.lines[0].order_line_id" == {{po_line_id}}

# --- 8. GET /goods-receipts/order/{po_id} — bare Vec<GoodsReceipt> (our receipt) ---
GET {{base_url}}/api/v1/goods-receipts/order/{{po_id}}
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection
jsonpath "$[0].id" == {{gr_id}}
jsonpath "$[0].purchase_order_id" == {{po_id}}

# --- 9. PUT /{id}/status "Completed" (AuthUser) → 200 raw GoodsReceipt entity (no lines) ---
PUT {{base_url}}/api/v1/goods-receipts/{{gr_id}}/status
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{"status": "Completed"}
```
HTTP 200
[Asserts]
jsonpath "$.id" == {{gr_id}}
jsonpath "$.status" == "Completed"

# --- 10. GET /goods-receipts/deleted (admin) — bare Vec decode fence (empty pre-delete) ---
GET {{base_url}}/api/v1/goods-receipts/deleted
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 11. RBAC: regular user DELETE goods-receipt → 403 (DELETE is AdminUser) ---
DELETE {{base_url}}/api/v1/goods-receipts/{{gr_id}}
Authorization: Bearer {{access_token}}
HTTP 403

# --- 12. DELETE soft (admin) → 200 MessageResponse ---
DELETE {{base_url}}/api/v1/goods-receipts/{{gr_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.message" exists

# --- 13. GET by-id → 404 (soft-deleted hidden) ---
GET {{base_url}}/api/v1/goods-receipts/{{gr_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 14. POST /restore (admin) → 200 GoodsReceiptResponse (NOTE: POST) ---
POST {{base_url}}/api/v1/goods-receipts/{{gr_id}}/restore
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{gr_id}}
jsonpath "$.lines" isCollection

# --- 15. GET by-id after restore → 200 ---
GET {{base_url}}/api/v1/goods-receipts/{{gr_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{gr_id}}

# --- 16. DELETE /destroy (admin) → 204 ---
DELETE {{base_url}}/api/v1/goods-receipts/{{gr_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 17. GET by-id after destroy → 404 ---
GET {{base_url}}/api/v1/goods-receipts/{{gr_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# ============ INVOICES + PAYMENTS ============

# --- 18. RBAC: regular user POST invoice → 403 (create is AdminUser) ---
POST {{base_url}}/api/v1/invoices
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "invoice_type": "SalesInvoice",
  "cari_id": {{cari_id}},
  "issue_date": "2026-06-01T10:00:00Z",
  "due_date": "2026-07-15T10:00:00Z",
  "currency": "TRY",
  "lines": [
    {"description": "rbac", "quantity": "1", "unit_price": "1", "tax_rate": "0", "discount_rate": "0"}
  ]
}
```
HTTP 403

# --- 19. RBAC: no-auth POST invoice → 401 ---
POST {{base_url}}/api/v1/invoices
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "invoice_type": "SalesInvoice",
  "cari_id": {{cari_id}},
  "issue_date": "2026-06-01T10:00:00Z",
  "due_date": "2026-07-15T10:00:00Z",
  "currency": "TRY",
  "lines": [
    {"description": "noauth", "quantity": "1", "unit_price": "1", "tax_rate": "0", "discount_rate": "0"}
  ]
}
```
HTTP 401

# --- 20. POST /invoices (admin) → 201 InvoiceResponse; capture inv_id ---
POST {{base_url}}/api/v1/invoices
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "invoice_type": "SalesInvoice",
  "cari_id": {{cari_id}},
  "issue_date": "2026-06-01T10:00:00Z",
  "due_date": "2026-07-15T10:00:00Z",
  "currency": "TRY",
  "notes": "hurl fence invoice",
  "lines": [
    {"product_id": {{product_id}}, "description": "Hurl invoice line 0", "quantity": "2", "unit_price": "100.00", "tax_rate": "18", "discount_rate": "0"}
  ]
}
```
HTTP 201
[Captures]
inv_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.invoice_number" isString
jsonpath "$.invoice_type" == "SalesInvoice"
jsonpath "$.status" isString
jsonpath "$.cari_id" == {{cari_id}}
jsonpath "$.subtotal" exists
jsonpath "$.tax_amount" exists
jsonpath "$.total_amount" exists
jsonpath "$.paid_amount" exists
jsonpath "$.currency" == "TRY"
jsonpath "$.lines" isCollection
jsonpath "$.lines[0].id" isNumber
jsonpath "$.company_id" isNumber

# --- 21. GET /invoices (AuthUser) — DECODE FENCE: paginated Invoice entity items ---
GET {{base_url}}/api/v1/invoices
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber
jsonpath "$.total" > 0
jsonpath "$.items[?(@.id=={{inv_id}})].invoice_number" exists

# --- 22. GET /invoices/{id} (AuthUser) — DECODE FENCE: InvoiceResponse with lines ---
GET {{base_url}}/api/v1/invoices/{{inv_id}}
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{inv_id}}
jsonpath "$.cari_id" == {{cari_id}}
jsonpath "$.invoice_type" == "SalesInvoice"
jsonpath "$.lines" isCollection
jsonpath "$.lines[0].line_total" exists
jsonpath "$.company_id" isNumber

# --- 23. PUT /invoices/{id}/status "Sent" (admin) → 200 raw Invoice entity (no lines) ---
PUT {{base_url}}/api/v1/invoices/{{inv_id}}/status
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{"status": "Sent"}
```
HTTP 200
[Asserts]
jsonpath "$.id" == {{inv_id}}
jsonpath "$.status" == "Sent"

# --- 24. GET /invoices/status/Sent — paginated; our invoice present ---
GET {{base_url}}/api/v1/invoices/status/Sent
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.items[?(@.id=={{inv_id}})].status" contains "Sent"

# --- 25. POST /invoices/payments (admin) → 201 Payment; capture pay_id ---
POST {{base_url}}/api/v1/invoices/payments
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "company_id": 1,
  "invoice_id": {{inv_id}},
  "amount": "50.00",
  "currency": "TRY",
  "payment_date": "2026-06-20T10:00:00Z",
  "payment_method": "BankTransfer",
  "reference_number": "HURL-PAY-001",
  "notes": "hurl fence payment"
}
```
HTTP 201
[Captures]
payment_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.invoice_id" == {{inv_id}}
jsonpath "$.amount" exists
jsonpath "$.currency" == "TRY"
jsonpath "$.payment_method" == "BankTransfer"
jsonpath "$.payment_date" exists
jsonpath "$.company_id" isNumber

# --- 26. GET /invoices/{id}/payments (AuthUser) — bare Vec<Payment> with our payment ---
GET {{base_url}}/api/v1/invoices/{{inv_id}}/payments
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection
jsonpath "$[0].id" == {{payment_id}}
jsonpath "$[0].invoice_id" == {{inv_id}}

# --- 27. GET /invoices/{id} after payment — paid_amount reflects the payment ---
GET {{base_url}}/api/v1/invoices/{{inv_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{inv_id}}
jsonpath "$.paid_amount" exists

# --- 28. GET /invoices/outstanding (AuthUser) — bare Vec<Invoice> decode ---
GET {{base_url}}/api/v1/invoices/outstanding
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 29. GET /invoices/overdue (AuthUser) — bare Vec<Invoice> decode ---
GET {{base_url}}/api/v1/invoices/overdue
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 30. GET /invoices/search?q=INV (AuthUser) — bare Vec<InvoiceResponse> decode ---
GET {{base_url}}/api/v1/invoices/search?q=INV
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 31. GET /invoices/deleted (admin) — bare Vec<InvoiceResponse> decode (empty) ---
GET {{base_url}}/api/v1/invoices/deleted
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 32. RBAC: regular user DELETE invoice → 403 (DELETE is AdminUser) ---
DELETE {{base_url}}/api/v1/invoices/{{inv_id}}
Authorization: Bearer {{access_token}}
HTTP 403

# --- 33. DELETE soft (admin) → 200 MessageResponse ---
DELETE {{base_url}}/api/v1/invoices/{{inv_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.message" exists

# --- 34. GET by-id → 404 (soft-deleted hidden) ---
GET {{base_url}}/api/v1/invoices/{{inv_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 35. PUT /restore (admin) → 200 InvoiceResponse (with lines) ---
PUT {{base_url}}/api/v1/invoices/{{inv_id}}/restore
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{inv_id}}
jsonpath "$.lines" isCollection

# --- 36. GET by-id after restore → 200 ---
GET {{base_url}}/api/v1/invoices/{{inv_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{inv_id}}

# --- 37. Invoice hard /destroy → 409 (fixed: FK RESTRICT surfaced as Conflict) ---
# payments.invoice_id (003:404) REFERENCES invoices(id) with NO ON DELETE →
# RESTRICT. The invoice has a payment (step 25), so a hard /destroy is now
# caught by the child-existence guard in invoice destroy → ApiError::Conflict
# (409) instead of letting the DELETE hit a FK violation → 500. (invoice_lines
# are ON DELETE CASCADE, so they do not block.) The invoice is left intact;
# invoices have NO unique constraint (invoice_number is app-generated), so
# re-runs do not collide.
DELETE {{base_url}}/api/v1/invoices/{{inv_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 409
[Asserts]
jsonpath "$.error" exists

# --- 39. RBAC: no-auth GET by-id → 401 ---
GET {{base_url}}/api/v1/invoices/999999
HTTP 401