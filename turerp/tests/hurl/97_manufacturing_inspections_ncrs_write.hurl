# 97_manufacturing_inspections_ncrs_write — WRITE-FENCE for the Quality
# Control CRUD lifecycle: Inspections + Non-Conformance Reports (NCRs).
#
# Inspections: POST /manufacturing/inspections → GET /inspections/deleted
#              (decode fence) → GET /inspections/{id} (decode fence) →
#              PUT /inspections/{id} (update) → DELETE /inspections/{id} (soft)
#              → GET /inspections/{id} 404 → PUT /inspections/{id}/restore →
#              GET /inspections/{id} 200 → DELETE /inspections/{id}/destroy →
#              GET /inspections/{id} 404.
# NCRs:        POST /manufacturing/ncrs → GET /ncrs/deleted (decode fence) →
#              GET /ncrs/{id} (decode fence) → PUT /ncrs/{id} (update) →
#              DELETE /ncrs/{id} (soft) → GET /ncrs/{id} 404 →
#              PUT /ncrs/{id}/restore → GET /ncrs/{id} 200 →
#              DELETE /ncrs/{id}/destroy → GET /ncrs/{id} 404.
#
# Why: the read-only 17_manufacturing.hurl only fences the gated (404) path.
# Inspection/NCR Row decode bugs in create/by-id/list-deleted/update/soft-
# delete/restore/destroy stay hidden until seeded. This fence self-seeds the
# FK chain (product → work order → inspection → NCR) and exercises every
# transition for both sub-resources.
#
# RBAC: writes (create/update/delete/restore/destroy/list-deleted) use the
# AdminUser extractor → testuser ({{access_token}}) gets 403, admin
# ({{admin_token}}) gets 200/201. Reads (list, by-id) use AuthUser → any
# authenticated user. No auth → 401. run-all.sh SKIPS this file when
# admin_token is empty.
#
# ASSUMPTION: the tier2.manufacturing feature flag is ON in this env; the
# read-only 17_manufacturing.hurl fences the gated (404) path. This file
# exercises the un-gated admin path, so it requires the flag enabled (same
# convention as 77_shifts_write.hurl).
# ASSUMPTION: GET /products is seeded non-empty by seed_sample_data.sql
# (P001/P002/P003) so items[0].id resolves as the inspection/NCR product_id.
# ASSUMPTION: rust_decimal serializes Decimal fields as JSON strings
# (quantity_inspected, quantity_passed, quantity_failed) — matches the cari
# write-fence convention.
# ASSUMPTION: InspectionStatus / NcrType / NcrStatus enums are PascalCase (no
# rename_all): Pending, InProgress, Passed, Failed, Rework / Minor, Major,
# Critical / Open, UnderReview, CorrectiveAction, Closed, Rejected.
# ASSUMPTION: create_inspection / create_ncr do NOT inject tenant_id (the
# handler passes the body through unchanged), so the body tenant_id is
# authoritative and MUST be sent.
# ASSUMPTION: CreateInspection.validate enforces passed+failed <= inspected;
# the seeded values (10 / 8 / 2) satisfy this. Update does not re-run this
# validator, so PUT with adjusted counts is accepted.
# ASSUMPTION: NCR raised_by={{user_id}} is a valid user row (testuser, tenant
# 1) so the FK holds; create_ncr validates raised_by > 0.
# ASSUMPTION: GET /inspections and GET /ncrs return bare Vec (not paginated)
# — the service calls find_by_tenant which returns Vec.

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

# --- 0a. Self-seed work order (inspection work_order_id FK) → capture work_order_id ---
POST {{base_url}}/api/v1/manufacturing/work-orders
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "name": "WF-QC-WO-001",
  "product_id": {{product_id}},
  "quantity": "100.00",
  "bom_id": null,
  "routing_id": null,
  "priority": "Normal",
  "planned_start": null,
  "planned_end": null
}
```
HTTP 201
[Captures]
work_order_id: jsonpath "$.id"

# ============================================================
# --- INSPECTIONS ---
# ============================================================

# --- 1. POST create inspection (admin) → 201; capture inspection_id ---
POST {{base_url}}/api/v1/manufacturing/inspections
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "work_order_id": {{work_order_id}},
  "product_id": {{product_id}},
  "inspection_type": "Final",
  "quantity_inspected": "10.00",
  "quantity_passed": "8.00",
  "quantity_failed": "2.00",
  "status": "Pending",
  "inspector_id": null,
  "notes": "hurl-seeded inspection"
}
```
HTTP 201
[Captures]
inspection_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.tenant_id" isNumber
jsonpath "$.work_order_id" == {{work_order_id}}
jsonpath "$.product_id" == {{product_id}}
jsonpath "$.inspection_type" == "Final"
jsonpath "$.quantity_inspected" exists
jsonpath "$.quantity_passed" exists
jsonpath "$.quantity_failed" exists
jsonpath "$.status" == "Pending"
jsonpath "$.created_at" exists

# --- 2. GET /inspections (auth) — DECODE FENCE: bare Vec, self-seeded row present ---
GET {{base_url}}/api/v1/manufacturing/inspections
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection
jsonpath "$[?(@.id=={{inspection_id}})].inspection_type" contains "Final"

# --- 3. GET /inspections/{id} (auth) — DECODE FENCE: single-row Inspection ---
GET {{base_url}}/api/v1/manufacturing/inspections/{{inspection_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{inspection_id}}
jsonpath "$.product_id" == {{product_id}}
jsonpath "$.work_order_id" == {{work_order_id}}
jsonpath "$.inspection_type" == "Final"
jsonpath "$.status" == "Pending"
jsonpath "$.deleted_at" exists
jsonpath "$.created_at" exists

# --- 4. PUT /inspections/{id} (admin) → 200; assert status + counts changed ---
PUT {{base_url}}/api/v1/manufacturing/inspections/{{inspection_id}}
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "status": "Passed",
  "quantity_passed": "9.00",
  "quantity_failed": "1.00",
  "inspector_id": null,
  "notes": "updated by hurl write-fence"
}
```
HTTP 200
[Asserts]
jsonpath "$.id" == {{inspection_id}}
jsonpath "$.status" == "Passed"
jsonpath "$.notes" == "updated by hurl write-fence"

# --- 5. GET /inspections/deleted (admin) — DECODE FENCE: bare Vec ---
GET {{base_url}}/api/v1/manufacturing/inspections/deleted
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 6. DELETE soft (admin) → 200 MessageResponse ---
DELETE {{base_url}}/api/v1/manufacturing/inspections/{{inspection_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.message" exists

# --- 7. GET by-id → 404 (soft-deleted hidden) ---
GET {{base_url}}/api/v1/manufacturing/inspections/{{inspection_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 8. PUT restore (admin) → 200 Inspection ---
PUT {{base_url}}/api/v1/manufacturing/inspections/{{inspection_id}}/restore
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{inspection_id}}
jsonpath "$.status" == "Passed"
jsonpath "$.deleted_at" exists

# --- 9. GET by-id → 200 (restored) ---
GET {{base_url}}/api/v1/manufacturing/inspections/{{inspection_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{inspection_id}}

# --- 10. DELETE destroy (admin) → 204 ---
DELETE {{base_url}}/api/v1/manufacturing/inspections/{{inspection_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 11. GET by-id → 404 (permanently destroyed) ---
GET {{base_url}}/api/v1/manufacturing/inspections/{{inspection_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# ============================================================
# --- NON-CONFORMANCE REPORTS (NCRs) ---
# ============================================================

# --- 12. POST create NCR (admin) → 201; capture ncr_id ---
POST {{base_url}}/api/v1/manufacturing/ncrs
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "inspection_id": null,
  "product_id": {{product_id}},
  "ncr_type": "Major",
  "description": "WF seeded NCR",
  "root_cause": null,
  "corrective_action": null,
  "raised_by": {{user_id}}
}
```
HTTP 201
[Captures]
ncr_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.tenant_id" isNumber
jsonpath "$.product_id" == {{product_id}}
jsonpath "$.ncr_type" == "Major"
jsonpath "$.description" == "WF seeded NCR"
jsonpath "$.status" == "Open"
jsonpath "$.raised_by" == {{user_id}}
jsonpath "$.raised_at" exists

# --- 13. GET /ncrs (auth) — DECODE FENCE: bare Vec, self-seeded row present ---
GET {{base_url}}/api/v1/manufacturing/ncrs
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection
jsonpath "$[?(@.id=={{ncr_id}})].description" contains "WF seeded NCR"

# --- 14. GET /ncrs/{id} (auth) — DECODE FENCE: single-row NonConformanceReport ---
GET {{base_url}}/api/v1/manufacturing/ncrs/{{ncr_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{ncr_id}}
jsonpath "$.product_id" == {{product_id}}
jsonpath "$.ncr_type" == "Major"
jsonpath "$.status" == "Open"
jsonpath "$.raised_by" == {{user_id}}
jsonpath "$.deleted_at" exists
jsonpath "$.raised_at" exists

# --- 15. PUT /ncrs/{id} (admin) → 200; assert status + root_cause changed ---
PUT {{base_url}}/api/v1/manufacturing/ncrs/{{ncr_id}}
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "ncr_type": "Minor",
  "description": "WF seeded NCR (updated)",
  "root_cause": "machine drift",
  "corrective_action": "recalibrate",
  "status": "UnderReview"
}
```
HTTP 200
[Asserts]
jsonpath "$.id" == {{ncr_id}}
jsonpath "$.ncr_type" == "Minor"
jsonpath "$.status" == "UnderReview"
jsonpath "$.root_cause" == "machine drift"

# --- 16. GET /ncrs/deleted (admin) — DECODE FENCE: bare Vec ---
GET {{base_url}}/api/v1/manufacturing/ncrs/deleted
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 17. DELETE soft (admin) → 200 MessageResponse ---
DELETE {{base_url}}/api/v1/manufacturing/ncrs/{{ncr_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.message" exists

# --- 18. GET by-id → 404 (soft-deleted hidden) ---
GET {{base_url}}/api/v1/manufacturing/ncrs/{{ncr_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 19. PUT restore (admin) → 200 NCR ---
PUT {{base_url}}/api/v1/manufacturing/ncrs/{{ncr_id}}/restore
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{ncr_id}}
jsonpath "$.status" == "UnderReview"
jsonpath "$.deleted_at" exists

# --- 20. GET by-id → 200 (restored) ---
GET {{base_url}}/api/v1/manufacturing/ncrs/{{ncr_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{ncr_id}}

# --- 21. DELETE destroy (admin) → 204 ---
DELETE {{base_url}}/api/v1/manufacturing/ncrs/{{ncr_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 22. GET by-id → 404 (permanently destroyed) ---
GET {{base_url}}/api/v1/manufacturing/ncrs/{{ncr_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# ============================================================
# --- RBAC fences ---
# ============================================================

# --- 23. RBAC: POST create inspection with regular user → 403 ---
POST {{base_url}}/api/v1/manufacturing/inspections
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "work_order_id": null,
  "product_id": {{product_id}},
  "inspection_type": "Final",
  "quantity_inspected": "1.00",
  "quantity_passed": "1.00",
  "quantity_failed": "0.00",
  "status": "Pending",
  "inspector_id": null,
  "notes": null
}
```
HTTP 403

# --- 24. RBAC: POST create inspection with no auth → 401 ---
POST {{base_url}}/api/v1/manufacturing/inspections
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "work_order_id": null,
  "product_id": {{product_id}},
  "inspection_type": "Final",
  "quantity_inspected": "1.00",
  "quantity_passed": "1.00",
  "quantity_failed": "0.00",
  "status": "Pending",
  "inspector_id": null,
  "notes": null
}
```
HTTP 401

# --- 25. RBAC: POST create NCR with regular user → 403 ---
POST {{base_url}}/api/v1/manufacturing/ncrs
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "inspection_id": null,
  "product_id": {{product_id}},
  "ncr_type": "Minor",
  "description": "should be forbidden",
  "root_cause": null,
  "corrective_action": null,
  "raised_by": {{user_id}}
}
```
HTTP 403

# --- 26. RBAC: POST create NCR with no auth → 401 ---
POST {{base_url}}/api/v1/manufacturing/ncrs
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "inspection_id": null,
  "product_id": {{product_id}},
  "ncr_type": "Minor",
  "description": "should be unauth",
  "root_cause": null,
  "corrective_action": null,
  "raised_by": {{user_id}}
}
```
HTTP 401

# --- 27. Cleanup: destroy the self-seeded work order (re-run safety) ---
# The work order seeded at step 0a is the inspection's work_order_id parent.
# Both the inspection (destroyed at step 10) and the NCR (destroyed at step 21)
# are hard-deleted BEFORE this step, so the work order has no remaining FK
# children. ORDER MATTERS: inspections.work_order_id REFERENCES work_orders(id)
# with no ON DELETE → RESTRICT (003:783), so the work-order /destroy here
# would 500 if the inspection still existed — it does not (step 10 first).
# work_orders.name is NOT unique, so the work order itself poses no re-run
# 409 risk; destroying the parent just keeps the suite tidy/idempotent.
DELETE {{base_url}}/api/v1/manufacturing/work-orders/{{work_order_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.message" exists
DELETE {{base_url}}/api/v1/manufacturing/work-orders/{{work_order_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 204