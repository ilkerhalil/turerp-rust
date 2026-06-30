# 91_hr_attendance_write — WRITE-FENCE for the Attendance CRUD lifecycle.
#
# Covers: POST /hr/attendance → GET /employee/{employee_id} (decode)
#         → GET /deleted (decode) → DELETE /{id}/soft-delete → PUT /{id}/restore
#         → DELETE /{id}/destroy. Self-seeds an employee for the
#         employee_id FK (attendance has no tenant scoping on create).
#
# Why: no read fence covers attendance; Attendance Row decode (Decimal
# hours_worked, AttendanceStatus enum, DateTime check_in/check_out) in the
# by-employee / deleted / soft-delete / restore / destroy paths is otherwise
# unexercised.
#
# RBAC: record/soft-delete/restore/destroy/list-deleted use AdminUser →
# testuser ({{access_token}}) gets 403, admin ({{admin_token}}) gets
# 200/201/204. get_attendance_by_employee uses AuthUser → any authed 200.
# No auth → 401. run-all.sh SKIPS this file when admin_token is empty.
#
# FK STRATEGY: attendance.employee_id is required and has no API to list
# employees that guarantees a row, so we self-seed an employee first via
# POST /hr/employees (admin) and capture its id. The employee is destroyed at
# the end to keep the suite re-runnable without orphans.
# ASSUMPTION: record_attendance forces `create.tenant_id = admin.tenant_id`
# (migration 048 / cross-tenant leak closure); the CreateAttendance struct
# now carries a required `tenant_id` field, so the POST body must include it
# (it is overwritten server-side from the admin token regardless).
# ASSUMPTION: get_attendance_by_employee returns a bare Vec<Attendance> (not
# paginated) — assert `$ isCollection`.
# ASSUMPTION: hours_worked is computed server-side from check_in/check_out
# (CreateAttendance::calculate_hours); we send both and do not send
# hours_worked. Decimal fields serialize as JSON strings.
# ASSUMPTION: DateTime fields (date/check_in/check_out) are chrono::DateTime<
# Utc>, sent as RFC3339.
# ASSUMPTION: AttendanceStatus enum serializes as PascalCase ("Present" etc.).
# Re-run safety: relies on DB reset; the seeded employee uses a unique
# employee_number "WF-ATT-EMP-001".

# --- 1. Self-seed employee (admin) for the employee_id FK ---
POST {{base_url}}/api/v1/hr/employees
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "company_id": 1,
  "employee_number": "WF-ATT-EMP-001",
  "first_name": "Attendance",
  "last_name": "Subject",
  "email": "wf-att-emp-001@example.com",
  "hire_date": "2026-01-15T09:00:00Z",
  "salary": "5000.00",
  "tc_kimlik_no": "12345678910",
  "children_count": 0
}
```
HTTP 201
[Captures]
emp_id: jsonpath "$.id"

# --- 2. RBAC: regular user POST /attendance → 403 ---
POST {{base_url}}/api/v1/hr/attendance
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "employee_id": {{emp_id}},
  "tenant_id": {{tenant_id}},
  "date": "2026-06-15T08:00:00Z",
  "check_in": "2026-06-15T08:00:00Z",
  "check_out": "2026-06-15T17:00:00Z",
  "notes": "should be forbidden"
}
```
HTTP 403

# --- 3. RBAC: no-auth POST /attendance → 401 ---
POST {{base_url}}/api/v1/hr/attendance
Content-Type: application/json
```
{
  "employee_id": {{emp_id}},
  "tenant_id": {{tenant_id}},
  "date": "2026-06-15T08:00:00Z",
  "check_in": "2026-06-15T08:00:00Z",
  "check_out": "2026-06-15T17:00:00Z",
  "notes": "no auth"
}
```
HTTP 401

# --- 4. Admin POST /attendance → 201; capture att_id ---
POST {{base_url}}/api/v1/hr/attendance
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "employee_id": {{emp_id}},
  "tenant_id": {{tenant_id}},
  "date": "2026-06-15T08:00:00Z",
  "check_in": "2026-06-15T08:00:00Z",
  "check_out": "2026-06-15T17:00:00Z",
  "notes": "fence attendance record"
}
```
HTTP 201
[Captures]
att_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.employee_id" == {{emp_id}}
jsonpath "$.status" == "Present"
jsonpath "$.hours_worked" exists
jsonpath "$.date" exists

# --- 5. GET /employee/{employee_id} (any authed) — DECODE FENCE: Vec ---
GET {{base_url}}/api/v1/hr/attendance/employee/{{emp_id}}
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection
jsonpath "$[?(@.id=={{att_id}})].status" contains "Present"

# --- 6. GET /deleted (admin) — DECODE FENCE: Vec<Attendance> ---
GET {{base_url}}/api/v1/hr/attendance/deleted
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 7. DELETE /{id}/soft-delete (admin) → 204 ---
DELETE {{base_url}}/api/v1/hr/attendance/{{att_id}}/soft-delete
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 8. GET /employee/{employee_id} → record no longer listed (soft-deleted) ---
GET {{base_url}}/api/v1/hr/attendance/employee/{{emp_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 9. PUT /{id}/restore (admin) → 200 Attendance ---
PUT {{base_url}}/api/v1/hr/attendance/{{att_id}}/restore
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{att_id}}
jsonpath "$.employee_id" == {{emp_id}}

# --- 10. DELETE /{id}/destroy (admin) → 204 (hard delete) ---
DELETE {{base_url}}/api/v1/hr/attendance/{{att_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 11. Cleanup: destroy the seeded employee ---
DELETE {{base_url}}/api/v1/hr/employees/{{emp_id}}
Authorization: Bearer {{admin_token}}
HTTP 204
DELETE {{base_url}}/api/v1/hr/employees/{{emp_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 204