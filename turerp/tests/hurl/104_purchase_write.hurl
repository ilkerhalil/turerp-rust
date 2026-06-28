# 104_purchase_write — WRITE-FENCE for the Purchase Orders + Purchase Requests surface.
#
# Purchase Orders:
#   POST create → GET list (decode) → GET by-id (decode) → PUT /{id}/status
#   → GET /status/{status} → GET /deleted → DELETE soft → GET 404
#   → POST /restore → GET 200 → DELETE /destroy → GET 404.
#
# Purchase Requests:
#   POST create → GET list → GET by-id → PUT /{id} update → POST /{id}/submit
#   → POST /{id}/approve → POST /{id}/reject (on a 2nd request) → GET /deleted
#   → DELETE soft → GET 404 → PUT /restore → GET 200 → DELETE /destroy → GET 404.
#
# Why: the read-only 30_purchase.hurl only hits the (empty) lists + 404s.
# Create/by-id/status/submit/approve/reject/lifecycle decode paths stay
# hidden until seeded. This fence self-seeds a PO + a PR and exercises
# every transition.
#
# RBAC NOTE (asymmetry vs sales):
#   - PO create (POST /purchase-orders) is AuthUser → regular user gets 201,
#     NOT 403. The 403 fence is on DELETE (AdminUser) instead.
#   - PR create (POST /purchase-requests) is AuthUser → same.
#   - PR submit is AuthUser; approve/reject are AdminUser → regular user
#     gets 403 on approve.
#   - restore method differs: PO = POST /{id}/restore; PR = PUT /{id}/restore.
# run-all.sh SKIPS this file when admin_token is empty.
#
# FK STRATEGY: GET-capture cari_id + product_id (seeded). PO create takes
# cari_id + lines (product_id optional). PR create takes requested_by
# (use {{user_id}}) + lines (product_id optional). No FK between PR and PO
# at create time (PR is converted to PO via a service path not exposed as a
# simple route here — ConvertedToOrder is a terminal status set by the
# business layer, not exercised by this fence).
#
# ASSUMPTION: suite runs against a freshly-reset DB (order_number /
# request_number are server-generated; no static unique code sent).
# ASSUMPTION: cari_id / product_id seeded non-empty (seed_sample_data.sql).
# ASSUMPTION: rust_decimal serializes Decimal as a JSON string ("100.00").
# ASSUMPTION: PurchaseOrderStatus / PurchaseRequestStatus have no
# #[serde(rename_all)] → PascalCase variants. PO: "Draft",
# "PendingApproval","Approved","SentToVendor","PartialReceived","Received",
# "Cancelled","OnHold". PR: "Draft","PendingApproval","Approved",
# "Rejected","ConvertedToOrder". PUT /purchase-orders/{id}/status takes
# {"status": "<String>"} (plain string, parsed server-side, no transition
# guard — any valid string accepted).
# ASSUMPTION: PR transition rules (model.rs can_transition_to): Draft →
# PendingApproval (submit); PendingApproval → Approved|Rejected
# (approve/reject); Approved → ConvertedToOrder; Rejected → Draft.
# submit only works from Draft; approve/reject only from PendingApproval.
# ASSUMPTION: PUT /purchase-orders/{id}/status returns raw PurchaseOrder
# entity (NO lines); POST submit/approve/reject return raw PurchaseRequest
# entity (NO lines). GET /{id} returns the *Response struct (with lines).
# ASSUMPTION: GET /deleted returns a bare Vec<...Response> (lines: [] for
# each), NOT paginated.
# ASSUMPTION: requested_by={{user_id}} is a valid user row (testuser,
# tenant 1) so the FK holds.

# ============ PURCHASE ORDERS ============

# --- 1. RBAC: no-auth POST PO → 401 ---
POST {{base_url}}/api/v1/purchase-orders
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

# --- 2. GET /caris — capture cari_id ---
GET {{base_url}}/api/v1/caris
Authorization: Bearer {{admin_token}}
HTTP 200
[Captures]
cari_id: jsonpath "$.items[0].id"
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" > 0

# --- 3. GET /products — capture product_id ---
GET {{base_url}}/api/v1/products
Authorization: Bearer {{admin_token}}
HTTP 200
[Captures]
product_id: jsonpath "$.items[0].id"
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" > 0

# --- 4. POST create PO (AuthUser — access_token works) → 201; capture ids ---
POST {{base_url}}/api/v1/purchase-orders
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "cari_id": {{cari_id}},
  "order_date": "2026-06-01T10:00:00Z",
  "expected_delivery_date": "2026-06-20T10:00:00Z",
  "notes": "hurl fence PO",
  "lines": [
    {"product_id": {{product_id}}, "description": "Hurl PO line 0", "quantity": "5", "unit_price": "100.00", "tax_rate": "18", "discount_rate": "0"},
    {"description": "Hurl PO line 1", "quantity": "2", "unit_price": "50.00", "tax_rate": "18", "discount_rate": "0"}
  ]
}
```
HTTP 201
[Captures]
po_id: jsonpath "$.id"
po_line0_id: jsonpath "$.lines[0].id"
po_line1_id: jsonpath "$.lines[1].id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.order_number" isString
jsonpath "$.cari_id" == {{cari_id}}
jsonpath "$.status" isString
jsonpath "$.lines" isCollection
jsonpath "$.lines[0].id" isNumber
jsonpath "$.company_id" isNumber

# --- 5. GET list — DECODE FENCE: paginated PurchaseOrder entity items ---
GET {{base_url}}/api/v1/purchase-orders
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber
jsonpath "$.total" > 0
jsonpath "$.items[?(@.id=={{po_id}})].order_number" exists

# --- 6. GET by-id — DECODE FENCE: PurchaseOrderResponse with lines ---
GET {{base_url}}/api/v1/purchase-orders/{{po_id}}
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{po_id}}
jsonpath "$.cari_id" == {{cari_id}}
jsonpath "$.status" isString
jsonpath "$.lines" isCollection
jsonpath "$.lines[0].id" == {{po_line0_id}}
jsonpath "$.lines[1].id" == {{po_line1_id}}
jsonpath "$.company_id" isNumber

# --- 7. PUT /{id}/status "Approved" (access_token, AuthUser) → 200 raw PO entity ---
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
jsonpath "$.cari_id" == {{cari_id}}

# --- 8. GET /status/Approved — paginated; our PO present ---
GET {{base_url}}/api/v1/purchase-orders?status=Approved
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.items[?(@.id=={{po_id}})].status" contains "Approved"

# --- 9. GET /purchase-orders/deleted (admin) — bare Vec decode fence (empty) ---
GET {{base_url}}/api/v1/purchase-orders/deleted
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 10. RBAC: regular user DELETE PO → 403 (DELETE is AdminUser) ---
DELETE {{base_url}}/api/v1/purchase-orders/{{po_id}}
Authorization: Bearer {{access_token}}
HTTP 403

# --- 11. DELETE soft (admin) → 200 MessageResponse ---
DELETE {{base_url}}/api/v1/purchase-orders/{{po_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.message" exists

# --- 12. GET by-id → 404 (soft-deleted hidden) ---
GET {{base_url}}/api/v1/purchase-orders/{{po_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 13. POST /restore (admin) → 200 PurchaseOrderResponse (NOTE: POST not PUT) ---
POST {{base_url}}/api/v1/purchase-orders/{{po_id}}/restore
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{po_id}}
jsonpath "$.cari_id" == {{cari_id}}
jsonpath "$.lines" isCollection

# --- 14. GET by-id after restore → 200 ---
GET {{base_url}}/api/v1/purchase-orders/{{po_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{po_id}}

# --- 15. DELETE /destroy (admin) → 204 ---
DELETE {{base_url}}/api/v1/purchase-orders/{{po_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 16. GET by-id after destroy → 404 ---
GET {{base_url}}/api/v1/purchase-orders/{{po_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# ============ PURCHASE REQUESTS ============

# --- 17. RBAC: no-auth POST request → 401 ---
POST {{base_url}}/api/v1/purchase-requests
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "company_id": 1,
  "requested_by": {{user_id}},
  "priority": "Medium",
  "lines": [
    {"description": "noauth", "quantity": "1"}
  ]
}
```
HTTP 401

# --- 18. POST create request #1 (AuthUser) → 201; capture pr_id ---
POST {{base_url}}/api/v1/purchase-requests
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "company_id": 1,
  "requested_by": {{user_id}},
  "department": "IT",
  "priority": "High",
  "reason": "hurl fence PR",
  "lines": [
    {"product_id": {{product_id}}, "description": "Hurl PR line 0", "quantity": "3", "notes": "fence"}
  ]
}
```
HTTP 201
[Captures]
pr_id: jsonpath "$.id"
pr_line0_id: jsonpath "$.lines[0].id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.request_number" isString
jsonpath "$.status" isString
jsonpath "$.priority" == "High"
jsonpath "$.lines" isCollection
jsonpath "$.lines[0].id" isNumber
jsonpath "$.company_id" isNumber

# --- 19. GET list — DECODE FENCE: paginated PurchaseRequest entity items ---
GET {{base_url}}/api/v1/purchase-requests
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber
jsonpath "$.total" > 0
jsonpath "$.items[?(@.id=={{pr_id}})].request_number" exists

# --- 20. GET by-id — DECODE FENCE: PurchaseRequestResponse with lines ---
GET {{base_url}}/api/v1/purchase-requests/{{pr_id}}
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{pr_id}}
jsonpath "$.status" isString
jsonpath "$.priority" == "High"
jsonpath "$.lines" isCollection
jsonpath "$.lines[0].id" == {{pr_line0_id}}
jsonpath "$.company_id" isNumber

# --- 21. PUT /{id} update (AuthUser) → 200 raw PurchaseRequest entity (no lines) ---
PUT {{base_url}}/api/v1/purchase-requests/{{pr_id}}
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{"priority": "Low", "reason": "deprioritized by fence"}
```
HTTP 200
[Asserts]
jsonpath "$.id" == {{pr_id}}
jsonpath "$.priority" == "Low"

# --- 22. POST /{id}/submit (AuthUser) → 200 PurchaseRequest (status == PendingApproval) ---
POST {{base_url}}/api/v1/purchase-requests/{{pr_id}}/submit
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{pr_id}}
jsonpath "$.status" == "PendingApproval"

# --- 23. RBAC: regular user POST approve → 403 (approve is AdminUser) ---
POST {{base_url}}/api/v1/purchase-requests/{{pr_id}}/approve
Authorization: Bearer {{access_token}}
HTTP 403

# --- 24. POST /{id}/approve (admin) → 200 PurchaseRequest (status == Approved) ---
POST {{base_url}}/api/v1/purchase-requests/{{pr_id}}/approve
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{pr_id}}
jsonpath "$.status" == "Approved"

# --- 25. POST create request #2 (AuthUser) → 201; capture pr_id2 (for reject path) ---
POST {{base_url}}/api/v1/purchase-requests
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "company_id": 1,
  "requested_by": {{user_id}},
  "priority": "Medium",
  "reason": "reject-path fence",
  "lines": [
    {"description": "Hurl PR2 line 0", "quantity": "1"}
  ]
}
```
HTTP 201
[Captures]
pr_id2: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.status" isString

# --- 26. POST /{id2}/submit (AuthUser) → 200 (status == PendingApproval) ---
POST {{base_url}}/api/v1/purchase-requests/{{pr_id2}}/submit
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{pr_id2}}
jsonpath "$.status" == "PendingApproval"

# --- 27. POST /{id2}/reject (admin) → 200 PurchaseRequest (status == Rejected) ---
POST {{base_url}}/api/v1/purchase-requests/{{pr_id2}}/reject
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{pr_id2}}
jsonpath "$.status" == "Rejected"

# --- 28. GET /purchase-requests/deleted (admin) — bare Vec decode fence (empty) ---
GET {{base_url}}/api/v1/purchase-requests/deleted
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 29. RBAC: regular user DELETE request → 403 (DELETE is AdminUser) ---
DELETE {{base_url}}/api/v1/purchase-requests/{{pr_id}}
Authorization: Bearer {{access_token}}
HTTP 403

# --- 30. DELETE soft (admin) → 200 MessageResponse ---
DELETE {{base_url}}/api/v1/purchase-requests/{{pr_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.message" exists

# --- 31. GET by-id → 404 (soft-deleted hidden) ---
GET {{base_url}}/api/v1/purchase-requests/{{pr_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 32. PUT /restore (admin) → 200 PurchaseRequestResponse (NOTE: PUT not POST) ---
PUT {{base_url}}/api/v1/purchase-requests/{{pr_id}}/restore
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{pr_id}}
jsonpath "$.lines" isCollection

# --- 33. GET by-id after restore → 200 ---
GET {{base_url}}/api/v1/purchase-requests/{{pr_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{pr_id}}

# --- 34. DELETE /destroy (admin) → 204 ---
DELETE {{base_url}}/api/v1/purchase-requests/{{pr_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 35. GET by-id after destroy → 404 ---
GET {{base_url}}/api/v1/purchase-requests/{{pr_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 36. RBAC: no-auth GET by-id → 401 ---
GET {{base_url}}/api/v1/purchase-requests/999999
HTTP 401