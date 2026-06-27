# 92_hr_leave_requests_write — WRITE-FENCE for LeaveRequest + LeaveType
# lifecycle.
#
# Leave requests: POST /hr/leave-requests → GET /employee/{employee_id}
#   (decode) → POST /{id}/approve → POST /{id}/reject → GET /deleted (decode)
#   → DELETE /{id}/soft-delete → PUT /{id}/restore → DELETE /{id}/destroy.
# Leave types: DELETE /{id}/soft-delete → PUT /{id}/restore
#   → DELETE /{id}/destroy.
#
# Why: no read fence covers leave-requests; LeaveRequest Row decode (Decimal
# total_days, LeaveRequestStatus enum, DateTime start/end/approved_at) and
# LeaveType Row decode (Decimal max_days_per_year) in the approve/reject/
# soft-delete/restore/destroy paths stay hidden until seeded.
#
# RBAC: create_leave_request & get_leave_requests_by_employee use AuthUser
# (any authed 200, no-auth 401). approve/reject/soft-delete/restore/destroy/
# list-deleted use AdminUser (regular 403, admin 200/204, no-auth 401).
# run-all.sh SKIPS this file when admin_token is empty.
#
# FK STRATEGY:
# - leave_requests.employee_id → self-seed an employee (POST /hr/employees,
#   admin) and capture emp_id.
# - leave_requests.leave_type_id → there is NO POST /hr/leave-types route, so
#   we capture an existing leave_type id from GET /hr/leave-types.
#   ASSUMPTION: at least one leave-type is seeded for tenant 1. If the list is
#   empty, step 4 (POST leave-request) will FK-fail — flagged for review.
#   ASSUMPTION: destroying the leave-type is done AFTER the leave-request that
#   references it is destroyed, so no FK cascade blocks the destroy. Other DB
#   rows referencing this leave-type would still block it — flagged.
# ASSUMPTION: CreateLeaveRequest has no tenant_id field; the service scopes
# via the employee lookup. total_days is computed server-side.
# ASSUMPTION: LeaveRequestStatus enum serializes as PascalCase ("Pending" /
# "Approved" / "Rejected"). DateTime fields are RFC3339.
# Re-run safety: relies on DB reset; the seeded employee uses a unique
# employee_number "WF-LR-EMP-001".

# --- 1. Self-seed employee (admin) for the employee_id FK ---
POST {{base_url}}/api/v1/hr/employees
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "company_id": 1,
  "employee_number": "WF-LR-EMP-001",
  "first_name": "Leave",
  "last_name": "Subject",
  "email": "wf-lr-emp-001@example.com",
  "hire_date": "2026-01-15T09:00:00Z",
  "salary": "5000.00",
  "tc_kimlik_no": "12345678911",
  "children_count": 0
}
```
HTTP 201
[Captures]
emp_id: jsonpath "$.id"

# --- 2. GET /leave-types → capture leave_type_id (ASSUMPTION: ≥1 seeded) ---
GET {{base_url}}/api/v1/hr/leave-types
Authorization: Bearer {{admin_token}}
HTTP 200
[Captures]
lt_id: jsonpath "$[0].id"
[Asserts]
jsonpath "$" isCollection

# --- 3. RBAC: no-auth POST /leave-requests → 401 (create is AuthUser, not Admin) ---
POST {{base_url}}/api/v1/hr/leave-requests
Content-Type: application/json
```
{
  "employee_id": {{emp_id}},
  "leave_type_id": {{lt_id}},
  "start_date": "2026-07-01T09:00:00Z",
  "end_date": "2026-07-03T17:00:00Z",
  "reason": "no auth"
}
```
HTTP 401

# --- 4. Admin POST /leave-requests (request A — to approve) → 201; capture lr_id ---
POST {{base_url}}/api/v1/hr/leave-requests
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "employee_id": {{emp_id}},
  "leave_type_id": {{lt_id}},
  "start_date": "2026-07-01T09:00:00Z",
  "end_date": "2026-07-03T17:00:00Z",
  "reason": "fence leave request A"
}
```
HTTP 201
[Captures]
lr_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.employee_id" == {{emp_id}}
jsonpath "$.leave_type_id" == {{lt_id}}
jsonpath "$.status" == "Pending"
jsonpath "$.total_days" exists

# --- 5. Admin POST /leave-requests (request B — to reject) → 201; capture lr_id2 ---
POST {{base_url}}/api/v1/hr/leave-requests
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "employee_id": {{emp_id}},
  "leave_type_id": {{lt_id}},
  "start_date": "2026-08-01T09:00:00Z",
  "end_date": "2026-08-02T17:00:00Z",
  "reason": "fence leave request B"
}
```
HTTP 201
[Captures]
lr_id2: jsonpath "$.id"
[Asserts]
jsonpath "$.status" == "Pending"

# --- 6. GET /employee/{employee_id} (any authed) — DECODE FENCE: Vec ---
GET {{base_url}}/api/v1/hr/leave-requests/employee/{{emp_id}}
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection
jsonpath "$[?(@.id=={{lr_id}})].status" contains "Pending"

# --- 7. RBAC: regular user POST /{id}/approve → 403 (AdminUser) ---
POST {{base_url}}/api/v1/hr/leave-requests/{{lr_id}}/approve
Authorization: Bearer {{access_token}}
HTTP 403

# --- 8. Admin POST /{id}/approve (request A) → 200; status flips to Approved ---
POST {{base_url}}/api/v1/hr/leave-requests/{{lr_id}}/approve
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{lr_id}}
jsonpath "$.status" == "Approved"
jsonpath "$.approved_by" exists
jsonpath "$.approved_at" exists

# --- 9. Admin POST /{id}/reject (request B) → 200; status flips to Rejected ---
POST {{base_url}}/api/v1/hr/leave-requests/{{lr_id2}}/reject
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{lr_id2}}
jsonpath "$.status" == "Rejected"

# --- 10. GET /deleted (admin) — DECODE FENCE: Vec<LeaveRequest> ---
GET {{base_url}}/api/v1/hr/leave-requests/deleted
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 11. DELETE /{id}/soft-delete (admin, request A) → 204 ---
DELETE {{base_url}}/api/v1/hr/leave-requests/{{lr_id}}/soft-delete
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 12. PUT /{id}/restore (admin, request A) → 200 LeaveRequest ---
PUT {{base_url}}/api/v1/hr/leave-requests/{{lr_id}}/restore
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{lr_id}}

# --- 13. DELETE /{id}/destroy (admin, request A) → 204 (hard delete) ---
DELETE {{base_url}}/api/v1/hr/leave-requests/{{lr_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 14. Cleanup: destroy request B ---
DELETE {{base_url}}/api/v1/hr/leave-requests/{{lr_id2}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 15. Leave-type lifecycle: soft-delete → restore → destroy ---
# ASSUMPTION: by this point no leave-request rows reference lt_id (both
# destroyed above), so the hard destroy will not FK-fail. Other pre-existing
# references in the DB would still block — flagged for review.

# --- 15a. DELETE /leave-types/{id}/soft-delete (admin) → 204 ---
DELETE {{base_url}}/api/v1/hr/leave-types/{{lt_id}}/soft-delete
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 15b. GET /leave-types/deleted (admin) → 200 Vec<LeaveType> ---
GET {{base_url}}/api/v1/hr/leave-types/deleted
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 15c. PUT /leave-types/{id}/restore (admin) → 200 LeaveType ---
PUT {{base_url}}/api/v1/hr/leave-types/{{lt_id}}/restore
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{lt_id}}
jsonpath "$.name" exists

# --- 15d. (omitted) leave-type hard /destroy — shared seeded row ---
# lt_id is the SHARED seeded leave type (seed_sample_data.sql) relied on by
# other suites; hard-destroying it here would break sibling fences. The
# soft-delete (15a) + restore (15c) lifecycle is already exercised, and 15c
# restores lt_id to the alive state, so it is left intact for downstream runs.
# The hard /destroy path is covered by the self-seeded parents in 94/95/96.

# --- 16. Cleanup: destroy the seeded employee ---
DELETE {{base_url}}/api/v1/hr/employees/{{emp_id}}
Authorization: Bearer {{admin_token}}
HTTP 204
DELETE {{base_url}}/api/v1/hr/employees/{{emp_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 204