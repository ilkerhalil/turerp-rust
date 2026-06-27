# 79_cost_center_allocations_write — WRITE-FENCE for cost center sub-resources.
#
# Covers the allocation + profitability sub-resources that 67_cost_centers
# _write.hurl (the parent CRUD lifecycle) does NOT touch:
#   /cost-centers/{id}/allocations   GET (list) + POST (create)
#   /cost-centers/{id}/profitability GET (report)
#
# Why: 25_cost_centers.hurl only hits the seeded list + by-id; 67 only walks
# the parent soft-delete lifecycle. The allocation create/list Row decode
# (AllocationResponse with Decimal amount/percentage) and the profitability
# report decode (ProfitabilityReport with Decimal totals + center_type enum)
# are never exercised — hidden decode bugs (the int4-as-int8 / Decimal class
# from PR #173) stay hidden until a throwaway cost center is seeded and its
# sub-resources are walked.
#
# FK STRATEGY: self-seed a parent cost center (POST /cost-centers, admin),
# capture its id, then exercise allocations + profitability under it. The
# allocation body's cost_center_id is overwritten by the path id in the
# handler, but serde still requires the field present, so it is echoed as
# {{cc_id}}. source_id is an opaque i64 (no FK validation in the handler) —
# a static value (1) is used.
#
# RBAC: parent create uses AdminUser → testuser ({{access_token}}) gets 403,
# admin ({{admin_token}}) gets 201. allocations + profitability use AuthUser
# → any authenticated user 200/201; only no-auth → 401.
# run-all.sh SKIPS this file when admin_token is empty.
#
# ASSUMPTION: suite runs against a freshly reset DB. The static cost center
# code `HURLALLOC` is unique per tenant; a second run against the same DB
# will 409 at step 1 — drop the leftover row or pick a different code.
# ASSUMPTION: center_type is CostCenterType, serialized via serde default
# (PascalCase): "Cost" / "Profit" — NOT lowercase (the lowercase
# Display/FromStr impls are query-string only, not JSON bodies). Mirrors
# 67_cost_centers_write.hurl.
# ASSUMPTION: rust_decimal deserializes amount/percentage from a JSON string
# ("100.00"); if the Decimal serde feature requires a number, step 2 will
# 400 — flagged for review (matches 74_cari_write.hurl credit_limit style).
# ASSUMPTION: profitability with no allocations returns zeroed Decimals
# (total_income=0, total_expense=0, net_profit=0) rather than 404; the
# cost center exists (seeded in step 1) so the report is generated.

# --- 1. POST cost center (admin) → 201; capture cc_id (parent for sub-resources) ---
POST {{base_url}}/api/v1/cost-centers
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "code": "HURLALLOC",
  "name": "Hurl Allocation Cost Center",
  "description": "self-seeded by 79 allocations write-fence",
  "center_type": "Cost",
  "parent_id": null,
  "is_active": true
}
```
HTTP 201
[Captures]
cc_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.code" == "HURLALLOC"
jsonpath "$.center_type" == "Cost"
jsonpath "$.tenant_id" isNumber

# --- 2. POST allocation (auth) → 201 AllocationResponse; decode Decimal amount/percentage ---
POST {{base_url}}/api/v1/cost-centers/{{cc_id}}/allocations
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "source_type": "invoice",
  "source_id": 1,
  "cost_center_id": {{cc_id}},
  "amount": "250.00",
  "percentage": "100.00",
  "allocation_date": null,
  "description": "hurl-seeded allocation"
}
```
HTTP 201
[Captures]
alloc_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.tenant_id" isNumber
jsonpath "$.cost_center_id" == {{cc_id}}
jsonpath "$.source_type" == "invoice"
jsonpath "$.source_id" isNumber
jsonpath "$.amount" exists
jsonpath "$.percentage" exists
jsonpath "$.allocation_date" exists
jsonpath "$.description" exists

# --- 3. GET allocations (auth) → 200 bare Vec decode; self-seeded row present ---
GET {{base_url}}/api/v1/cost-centers/{{cc_id}}/allocations
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection
jsonpath "$[0].id" == {{alloc_id}}
jsonpath "$[0].cost_center_id" == {{cc_id}}
jsonpath "$[0].amount" exists
jsonpath "$[0].percentage" exists

# --- 4. POST second allocation (auth) → 201 (exercise Vec decode with >1 row) ---
POST {{base_url}}/api/v1/cost-centers/{{cc_id}}/allocations
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "source_type": "payroll",
  "source_id": 2,
  "cost_center_id": {{cc_id}},
  "amount": "75.50",
  "percentage": "25.00",
  "allocation_date": null,
  "description": "second hurl-seeded allocation"
}
```
HTTP 201
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.source_type" == "payroll"
jsonpath "$.amount" exists

# --- 5. GET allocations again — decode fence: >=2 rows, both decode ---
GET {{base_url}}/api/v1/cost-centers/{{cc_id}}/allocations
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection
jsonpath "$[0].amount" exists
jsonpath "$[1].amount" exists

# --- 6. GET profitability (auth) → 200 ProfitabilityReport; decode Decimal totals + enum ---
GET {{base_url}}/api/v1/cost-centers/{{cc_id}}/profitability
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.cost_center_id" == {{cc_id}}
jsonpath "$.cost_center_code" == "HURLALLOC"
jsonpath "$.cost_center_name" isString
jsonpath "$.center_type" == "Cost"
jsonpath "$.total_income" exists
jsonpath "$.total_expense" exists
jsonpath "$.net_profit" exists
jsonpath "$.allocation_count" isNumber
jsonpath "$.period_start" exists
jsonpath "$.period_end" exists

# --- 7. GET profitability with explicit period window → 200 (query params optional) ---
GET {{base_url}}/api/v1/cost-centers/{{cc_id}}/profitability?period_start=2026-01-01T00:00:00Z&period_end=2026-12-31T23:59:59Z
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.cost_center_id" == {{cc_id}}
jsonpath "$.allocation_count" isNumber

# --- 8. GET allocations for non-existent cost center → 404 ---
GET {{base_url}}/api/v1/cost-centers/999999/allocations
Authorization: Bearer {{access_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 9. GET profitability for non-existent cost center → 404 ---
GET {{base_url}}/api/v1/cost-centers/999999/profitability
Authorization: Bearer {{access_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 10. No-auth POST allocation → 401 ---
POST {{base_url}}/api/v1/cost-centers/{{cc_id}}/allocations
Content-Type: application/json
```
{
  "source_type": "invoice",
  "source_id": 1,
  "cost_center_id": {{cc_id}},
  "amount": "1.00",
  "percentage": "100.00",
  "allocation_date": null,
  "description": "no auth"
}
```
HTTP 401

# --- 11. No-auth GET profitability → 401 ---
GET {{base_url}}/api/v1/cost-centers/{{cc_id}}/profitability
HTTP 401

# --- 12. Cleanup: DELETE parent cost center (soft) → 204 ---
DELETE {{base_url}}/api/v1/cost-centers/{{cc_id}}
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 13. GET allocations after parent soft-delete → 404 (cost center hidden) ---
GET {{base_url}}/api/v1/cost-centers/{{cc_id}}/allocations
Authorization: Bearer {{access_token}}
HTTP 404

# --- 14. RBAC: POST parent cost center as regular user → 403 (AdminUser) ---
POST {{base_url}}/api/v1/cost-centers
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "code": "FORBIDDEN-ALLOC",
  "name": "should be forbidden",
  "center_type": "Cost",
  "parent_id": null,
  "is_active": true
}
```
HTTP 403