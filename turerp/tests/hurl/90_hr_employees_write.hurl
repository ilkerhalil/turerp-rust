# 90_hr_employees_write — WRITE-FENCE for the Employee CRUD + lifecycle.
#
# Covers: POST /hr/employees → GET list (decode) → GET /{id} (decode)
#         → PUT /{id}/status → POST /{id}/terminate → DELETE /{id} (soft)
#         → GET /{id} 404 → PUT /{id}/restore → GET /{id} 200
#         → DELETE /{id}/destroy → GET /{id} 404 → GET /deleted (decode).
#
# Why: 11_hr_employees.hurl only fences the empty list + leave-types list,
# so EmployeeResponse Row decode (Decimal salary/gross_salary, EmployeeStatus
# enum, DateTime hire_date) in the by-id / status / terminate / deleted
# paths stays hidden until seeded. This fence self-seeds an employee.
#
# RBAC: create/status/terminate/soft-delete/restore/destroy/list-deleted all
# use AdminUser → testuser ({{access_token}}) gets 403, admin ({{admin_token}})
# gets 200/201/204. No auth → 401. run-all.sh SKIPS this file when admin_token
# is empty.
#
# ASSUMPTION: the handler injects tenant_id from admin_user.0.tenant_id, so
# the body's tenant_id is overridden — we still send {{tenant_id}} because
# CreateEmployee.tenant_id is a required (non-Option) field. company_id
# defaults to 1 via #[serde(default)]; we send it explicitly for clarity.
# ASSUMPTION: salary is a rust_decimal Decimal sent as a JSON string
# ("5000.00") — matches 74_cari_write.hurl credit_limit convention.
# ASSUMPTION: hire_date is chrono::DateTime<Utc>, sent as RFC3339.
# ASSUMPTION: EmployeeStatus enum serializes as PascalCase ("Active"/"OnLeave"
# /"Suspended"/"Terminated") — no serde rename_all on the enum.
# ASSUMPTION: terminate_employee flips status to Terminated but does NOT soft-
# delete; the subsequent DELETE /{id} soft-delete still 204s.
# ASSUMPTION: restore_employee returns EmployeeResponse (200 body); destroy
# returns 204. After soft-delete, GET /{id} 404; after destroy, GET /{id} 404.
# Re-run safety: employee_number "WF-EMP-001" is unique per tenant; a second
# run against the same DB will 409 at step 1 — relies on DB reset between runs.

# --- 1. RBAC: regular user POST create → 403 ---
POST {{base_url}}/api/v1/hr/employees
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "company_id": 1,
  "employee_number": "WF-EMP-RBAC",
  "first_name": "Should",
  "last_name": "Forbidden",
  "email": "wf-emp-rbac@example.com",
  "hire_date": "2026-01-15T09:00:00Z",
  "salary": "5000.00",
  "tc_kimlik_no": "12345678901",
  "children_count": 0
}
```
HTTP 403

# --- 2. RBAC: no-auth POST create → 401 ---
POST {{base_url}}/api/v1/hr/employees
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "company_id": 1,
  "employee_number": "WF-EMP-NOAUTH",
  "first_name": "No",
  "last_name": "Auth",
  "email": "wf-emp-noauth@example.com",
  "hire_date": "2026-01-15T09:00:00Z",
  "salary": "5000.00",
  "tc_kimlik_no": "12345678902",
  "children_count": 0
}
```
HTTP 401

# --- 3. Admin POST create → 201; capture id ---
POST {{base_url}}/api/v1/hr/employees
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "company_id": 1,
  "employee_number": "WF-EMP-001",
  "first_name": "Wf",
  "last_name": "Employee",
  "email": "wf-emp-001@example.com",
  "phone": "+905551112233",
  "department": "Engineering",
  "position": "Developer",
  "hire_date": "2026-01-15T09:00:00Z",
  "salary": "5000.00",
  "tc_kimlik_no": "12345678903",
  "children_count": 0
}
```
HTTP 201
[Captures]
emp_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.employee_number" == "WF-EMP-001"
jsonpath "$.full_name" == "Wf Employee"
jsonpath "$.status" == "Active"
jsonpath "$.salary" exists
jsonpath "$.gross_salary" exists
jsonpath "$.tenant_id" isNumber
jsonpath "$.company_id" isNumber
jsonpath "$.hire_date" exists

# --- 4. GET list (admin) — DECODE FENCE: paginated {items,total} ---
GET {{base_url}}/api/v1/hr/employees
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber
jsonpath "$.total" > 0
jsonpath "$.items[?(@.id=={{emp_id}})].employee_number" contains "WF-EMP-001"

# --- 5. GET by-id (admin) — DECODE FENCE: single EmployeeResponse ---
GET {{base_url}}/api/v1/hr/employees/{{emp_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{emp_id}}
jsonpath "$.employee_number" == "WF-EMP-001"
jsonpath "$.first_name" == "Wf"
jsonpath "$.last_name" == "Employee"
jsonpath "$.full_name" == "Wf Employee"
jsonpath "$.email" == "wf-emp-001@example.com"
jsonpath "$.status" == "Active"
jsonpath "$.salary" exists
jsonpath "$.gross_salary" exists
jsonpath "$.hire_date" exists
jsonpath "$.tenant_id" isNumber
jsonpath "$.company_id" isNumber

# --- 6. PUT /{id}/status (admin) → 200; status flips to Suspended ---
PUT {{base_url}}/api/v1/hr/employees/{{emp_id}}/status
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{ "status": "Suspended" }
```
HTTP 200
[Asserts]
jsonpath "$.status" == "Suspended"

# --- 7. GET by-id — confirms status persisted ---
GET {{base_url}}/api/v1/hr/employees/{{emp_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.status" == "Suspended"

# --- 8. POST /{id}/terminate (admin) → 200; status flips to Terminated ---
POST {{base_url}}/api/v1/hr/employees/{{emp_id}}/terminate
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.status" == "Terminated"

# --- 9. DELETE /{id} soft (admin) → 204 ---
DELETE {{base_url}}/api/v1/hr/employees/{{emp_id}}
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 10. GET by-id → 404 (soft-deleted hidden) ---
GET {{base_url}}/api/v1/hr/employees/{{emp_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 11. GET /deleted (admin) → 200 array; self-seeded row visible ---
GET {{base_url}}/api/v1/hr/employees/deleted
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 12. PUT /{id}/restore (admin) → 200 EmployeeResponse ---
PUT {{base_url}}/api/v1/hr/employees/{{emp_id}}/restore
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{emp_id}}
jsonpath "$.employee_number" == "WF-EMP-001"
jsonpath "$.status" == "Terminated"

# --- 13. GET by-id → 200 (restored; status remains Terminated) ---
GET {{base_url}}/api/v1/hr/employees/{{emp_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{emp_id}}

# --- 14. DELETE /{id}/destroy (admin) → 204 (hard delete) ---
DELETE {{base_url}}/api/v1/hr/employees/{{emp_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 15. GET by-id → 404 (permanently destroyed) ---
GET {{base_url}}/api/v1/hr/employees/{{emp_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists