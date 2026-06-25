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

# --- 3. Payroll by employee — 200 (was 500: payrolls table missing
#     company_id, which migration 023 added to 10 tables but skipped;
#     fixed by migration 043). find_by_employee returns a plain JSON
#     array; non-existent employee → empty array. 200 proves the query
#     runs (the list-empty path still hits the missing-column bug, so
#     this fences the decode fix). ---
GET {{base_url}}/api/v1/hr/payroll/employee/999999
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 4. Deleted payroll — RBAC: regular user gets 403 ---
GET {{base_url}}/api/v1/hr/payroll/deleted
Authorization: Bearer {{access_token}}
HTTP 403
