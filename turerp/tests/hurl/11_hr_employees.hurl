# Tier 1.5 — HR. Largest module by path count (34 paths). This scenario
# exercises the employees resource (working) and the leave-types list
# (now gated by core.hr.leave_types, default-disabled; PR 2 fixes the
# underlying handler).


# --- 1. Employees list — expected 200 ---
GET {{base_url}}/api/v1/hr/employees
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber

# --- 2. Leave types — currently 404 (gated; PR 2 fixes handler) ---
GET {{base_url}}/api/v1/hr/leave-types
Authorization: Bearer {{access_token}}
HTTP 404
