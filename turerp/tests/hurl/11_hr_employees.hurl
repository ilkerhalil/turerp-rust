# Tier 1.5 — HR. Largest module by path count (34 paths). This scenario
# exercises the employees resource (working) and the leave-types list
# (gated by core.hr.leave_types). Issue #152 fixed the underlying
# LeaveTypeRow decode (phantom company_id column) so leave-types now
# returns 200 when the flag is enabled.


# --- 1. Employees list — expected 200 ---
GET {{base_url}}/api/v1/hr/employees
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber

# --- 2. Leave types — fixed by #152 (handler SQL bug) ---
# The /hr/leave-types endpoint returns a plain JSON array
# (Vec<LeaveType>), not the paginated {items,total,page,per_page}
# envelope used by other list endpoints. Assert an array body.
GET {{base_url}}/api/v1/hr/leave-types
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection
