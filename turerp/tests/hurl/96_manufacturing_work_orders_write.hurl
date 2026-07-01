# 96_manufacturing_work_orders_write — WRITE-FENCE for the Work Order CRUD
# lifecycle + status transition + operations/materials sub-resources.
#
# Covers: POST /manufacturing/work-orders → GET /work-orders (decode fence,
#         paginated) → GET /work-orders/{id} (decode fence) →
#         PUT /work-orders/{id}/status → DELETE /work-orders/{id} (soft) →
#         GET /work-orders/{id} 404 → PUT /work-orders/{id}/restore →
#         GET /work-orders/{id} 200 → DELETE /work-orders/{id}/destroy →
#         GET /work-orders/{id} 404 → GET /work-orders/deleted (decode fence).
#         Plus: POST /work-orders/operations → GET /work-orders/{id}/operations
#         (decode fence, bare Vec) → POST /work-orders/materials →
#         GET /work-orders/{id}/materials (decode fence, bare Vec).
#
# Why: the read-only 17_manufacturing.hurl only fences the gated (404) path.
# WorkOrder Row decode bugs in create/by-id/list/operations/materials/status/
# soft-delete/restore/destroy stay hidden until a work order is seeded. This
# fence self-seeds the FK chain (product → BOM + routing → work order) and
# exercises every transition.
#
# RBAC: writes (create/status/operation/material/soft-delete/restore/destroy/
# list-deleted) use the AdminUser extractor → testuser ({{access_token}}) gets
# 403, admin ({{admin_token}}) gets 200/201. Reads (list, by-id, operations,
# materials) use AuthUser → any authenticated user. No auth → 401.
# run-all.sh SKIPS this file when admin_token is empty.
#
# ASSUMPTION: the tier2.manufacturing feature flag is ON in this env; the
# read-only 17_manufacturing.hurl fences the gated (404) path. This file
# exercises the un-gated admin path, so it requires the flag enabled (same
# convention as 77_shifts_write.hurl).
# ASSUMPTION: GET /products is seeded non-empty by seed_sample_data.sql
# (P001/P002/P003) so items[0].id resolves as the work order's product_id and
# the material's product_id.
# ASSUMPTION: rust_decimal serializes Decimal fields as JSON strings (quantity,
# planned_hours, quantity_required) — matches the cari write-fence convention.
# ASSUMPTION: WorkOrderStatus / WorkOrderPriority enums are PascalCase (no
# rename_all): Draft, Scheduled, InProgress, OnHold, Completed, Cancelled /
# Low, Normal, High, Urgent.
# ASSUMPTION: create_work_order injects tenant_id from admin_user (handler
# overrides the body value), so the body tenant_id is informational only.
# ASSUMPTION: new work orders default to status=Draft; the PUT /status transition
# Draft → Scheduled → InProgress is accepted by the service.

# --- 0. FK capture: GET /products → product_id ---
GET {{base_url}}/api/v1/products
Authorization: Bearer {{admin_token}}
HTTP 200
[Captures]
product_id: jsonpath "$.items[0].id"
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber
jsonpath "$.total" > 0

# --- 0a. Self-seed BOM (work order bom_id FK) → capture bom_id ---
POST {{base_url}}/api/v1/manufacturing/boms
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "product_id": {{product_id}},
  "version": "WF-WO-BOM-1.0",
  "is_active": true,
  "is_primary": true,
  "valid_from": null,
  "valid_to": null
}
```
HTTP 201
[Captures]
bom_id: jsonpath "$.id"

# --- 0b. Self-seed Routing (work order routing_id FK) → capture routing_id ---
POST {{base_url}}/api/v1/manufacturing/routings
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "product_id": {{product_id}},
  "version": "WF-WO-RT-1.0",
  "is_active": true,
  "is_primary": true
}
```
HTTP 201
[Captures]
routing_id: jsonpath "$.id"

# --- 1. POST create work order (admin) → 201; capture work_order_id ---
POST {{base_url}}/api/v1/manufacturing/work-orders
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "name": "WF-WO-001",
  "product_id": {{product_id}},
  "quantity": "100.00",
  "bom_id": {{bom_id}},
  "routing_id": {{routing_id}},
  "priority": "Normal",
  "planned_start": null,
  "planned_end": null
}
```
HTTP 201
[Captures]
work_order_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.tenant_id" isNumber
jsonpath "$.name" == "WF-WO-001"
jsonpath "$.product_id" == {{product_id}}
jsonpath "$.bom_id" == {{bom_id}}
jsonpath "$.routing_id" == {{routing_id}}
jsonpath "$.quantity" exists
jsonpath "$.status" == "Draft"
jsonpath "$.priority" == "Normal"
jsonpath "$.created_at" exists

# --- 2. GET /work-orders (auth) — DECODE FENCE: paginated envelope, self-seeded row present ---
GET {{base_url}}/api/v1/manufacturing/work-orders
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber
jsonpath "$.total" > 0
jsonpath "$.items[?(@.id=={{work_order_id}})].name" contains "WF-WO-001"

# --- 3. GET /work-orders/{id} (auth) — DECODE FENCE: single-row WorkOrder ---
GET {{base_url}}/api/v1/manufacturing/work-orders/{{work_order_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{work_order_id}}
jsonpath "$.name" == "WF-WO-001"
jsonpath "$.product_id" == {{product_id}}
jsonpath "$.status" == "Draft"
jsonpath "$.priority" == "Normal"
jsonpath "$.deleted_at" exists
jsonpath "$.created_at" exists

# --- 4. PUT /work-orders/{id}/status (admin) → 200; assert status transition ---
PUT {{base_url}}/api/v1/manufacturing/work-orders/{{work_order_id}}/status
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{ "status": "Scheduled" }
```
HTTP 200
[Asserts]
jsonpath "$.id" == {{work_order_id}}
jsonpath "$.status" == "Scheduled"

# --- 5. POST /work-orders/operations (admin) → 201; add an operation ---
POST {{base_url}}/api/v1/manufacturing/work-orders/operations
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "work_order_id": {{work_order_id}},
  "operation_sequence": 10,
  "operation_name": "WF Turn",
  "work_center_id": null,
  "planned_hours": "1.50"
}
```
HTTP 201
[Captures]
wo_op_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.work_order_id" == {{work_order_id}}
jsonpath "$.operation_sequence" isNumber
jsonpath "$.operation_name" == "WF Turn"
jsonpath "$.planned_hours" exists

# --- 6. GET /work-orders/{id}/operations (auth) — DECODE FENCE: bare Vec, self-seeded row present ---
GET {{base_url}}/api/v1/manufacturing/work-orders/{{work_order_id}}/operations
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection
jsonpath "$[?(@.id=={{wo_op_id}})].operation_name" contains "WF Turn"

# --- 7. POST /work-orders/materials (admin) → 201; add a material ---
POST {{base_url}}/api/v1/manufacturing/work-orders/materials
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "work_order_id": {{work_order_id}},
  "product_id": {{product_id}},
  "quantity_required": "50.00"
}
```
HTTP 201
[Captures]
wo_mat_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.work_order_id" == {{work_order_id}}
jsonpath "$.product_id" == {{product_id}}
jsonpath "$.quantity_required" exists

# --- 8. GET /work-orders/{id}/materials (auth) — DECODE FENCE: bare Vec, self-seeded row present ---
GET {{base_url}}/api/v1/manufacturing/work-orders/{{work_order_id}}/materials
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection
jsonpath "$[?(@.id=={{wo_mat_id}})].product_id" == {{product_id}}

# --- 9. DELETE soft (admin) → 200 MessageResponse ---
DELETE {{base_url}}/api/v1/manufacturing/work-orders/{{work_order_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.message" exists

# --- 10. GET by-id → 404 (soft-deleted hidden) ---
GET {{base_url}}/api/v1/manufacturing/work-orders/{{work_order_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 11. GET /work-orders/deleted (admin) — DECODE FENCE: bare Vec, soft-deleted row present ---
GET {{base_url}}/api/v1/manufacturing/work-orders/deleted
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection
jsonpath "$[?(@.id=={{work_order_id}})].name" contains "WF-WO-001"

# --- 12. PUT restore (admin) → 200 WorkOrder ---
PUT {{base_url}}/api/v1/manufacturing/work-orders/{{work_order_id}}/restore
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{work_order_id}}
jsonpath "$.name" == "WF-WO-001"
jsonpath "$.deleted_at" exists

# --- 13. GET by-id → 200 (restored) ---
GET {{base_url}}/api/v1/manufacturing/work-orders/{{work_order_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{work_order_id}}

# --- 14. DELETE destroy (admin) → 204 ---
DELETE {{base_url}}/api/v1/manufacturing/work-orders/{{work_order_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 15. GET by-id → 404 (permanently destroyed) ---
GET {{base_url}}/api/v1/manufacturing/work-orders/{{work_order_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 15b. Cleanup: destroy the self-seeded BOM + Routing (re-run safety) ---
# bills_of_materials(product_id, version, tenant_id) and routings(...) are
# NON-partial unique indexes (003:723, 003:757); the self-seeded parents
# (steps 0a/0b) are NOT otherwise destroyed, so without this cleanup a re-run
# 409s at steps 0a/0b. work_orders.bom_id/routing_id are plain BIGINT (no FK),
# so the work-order destroy above doesn't cascade — destroy parents here.
DELETE {{base_url}}/api/v1/manufacturing/boms/{{bom_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 204
DELETE {{base_url}}/api/v1/manufacturing/routings/{{routing_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 16. RBAC: POST create with regular user → 403 ---
POST {{base_url}}/api/v1/manufacturing/work-orders
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "name": "WF-RBAC-403",
  "product_id": {{product_id}},
  "quantity": "1.00",
  "bom_id": null,
  "routing_id": null,
  "priority": "Low",
  "planned_start": null,
  "planned_end": null
}
```
HTTP 403

# --- 17. RBAC: POST create with no auth → 401 ---
POST {{base_url}}/api/v1/manufacturing/work-orders
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "name": "WF-NOAUTH",
  "product_id": {{product_id}},
  "quantity": "1.00",
  "bom_id": null,
  "routing_id": null,
  "priority": "Low",
  "planned_start": null,
  "planned_end": null
}
```
HTTP 401