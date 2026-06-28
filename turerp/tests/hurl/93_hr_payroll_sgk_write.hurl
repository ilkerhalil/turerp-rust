# 93_hr_payroll_sgk_write — RBAC + CALCULATE-SUCCESS FENCE for Payroll + SGK.
#
# Scope: self-seed an employee, fence the RBAC layer (403 regular / 401
# no-auth) and the admin calculate path (200 + payroll row) for BOTH
# /hr/payroll/calculate and /hr/sgk/calculate, then clean up the created
# payrolls + the seeded employee.
#
# FIXED (source-side, this PR): PostgresPayrollRepository::create
# (src/domain/hr/postgres_repository.rs:1205) previously issued an INSERT with
# 14 target columns but 13 value expressions (company_id listed but not
# bound) → PostgreSQL parse error "INSERT has more target columns than
# expressions" → sqlx::Error → map_sqlx_error → ApiError::Database → 500 on
# BOTH calculate_payroll (hr/service.rs) and calculate_sgk_payroll
# (hr/sgk/service.rs:211, calls hr_provider.create_payroll). Fix = drop
# company_id from the INSERT column list (it defaults to 1 via migration 043;
# the 12 binds then align 1:1 with $1..$12 + NOW()). With the fix the admin
# calculate path returns 200 with a Payroll row ($.id) instead of 500.
#
# The full payroll/sgk lifecycle (paid, deleted, soft-delete, restore, bonus,
# summary, ebildirge) is still NOT fenced here — those transitions are out of
# scope for this bug-fix PR. The two created payrolls are hard-destroyed in
# cleanup (step 8) because payrolls.employee_id FK-RESTRICTs the employee
# destroy (step 9).
#
# RBAC: payroll/sgk calculate use the AdminUser extractor → testuser
# ({{access_token}}) gets 403, no-auth gets 401 (JwtAuth runs first), admin
# ({{admin_token}}) reaches the handler and gets 200. run-all.sh SKIPS this
# file when admin_token is empty.
#
# FK STRATEGY: payroll.employee_id + sgk.employee_id → self-seed an employee
# (POST /hr/employees, admin) and capture emp_id. The payrolls created by the
# two calculate calls are destroyed at step 8; the employee is then cleaned up
# (soft-delete + destroy) at step 9; employees(employee_number, email,
# tenant_id) are NON-partial unique (003:439-440), so the hard destroy frees
# the "WF-PAY-EMP-001" / "wf-pay-emp-001@example.com" slots for re-runs.

# --- 1. Self-seed employee (admin) for the employee_id FK → 201; capture emp_id ---
POST {{base_url}}/api/v1/hr/employees
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "company_id": 1,
  "employee_number": "WF-PAY-EMP-001",
  "first_name": "Payroll",
  "last_name": "Subject",
  "email": "wf-pay-emp-001@example.com",
  "hire_date": "2026-01-15T09:00:00Z",
  "salary": "25000.00",
  "tc_kimlik_no": "12345678912",
  "children_count": 0
}
```
HTTP 201
[Captures]
emp_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.employee_number" == "WF-PAY-EMP-001"

# === Payroll calculate: RBAC + 500 fence ===

# --- 2. RBAC: regular user POST /payroll/calculate → 403 (AdminUser first) ---
POST {{base_url}}/api/v1/hr/payroll/calculate
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "employee_id": {{emp_id}},
  "period_start": "2026-06-01T00:00:00Z",
  "period_end": "2026-06-30T23:59:59Z"
}
```
HTTP 403
[Asserts]
jsonpath "$.error" exists

# --- 3. RBAC: no-auth POST /payroll/calculate → 401 (JwtAuth before RBAC) ---
POST {{base_url}}/api/v1/hr/payroll/calculate
Content-Type: application/json
```
{
  "employee_id": {{emp_id}},
  "period_start": "2026-06-01T00:00:00Z",
  "period_end": "2026-06-30T23:59:59Z"
}
```
HTTP 401
[Asserts]
jsonpath "$.error" exists

# --- 4. Admin POST /payroll/calculate → 200 (fixed: payrolls INSERT) ---
# Admin passes the extractor and reaches create_payroll → the INSERT now has
# 13 columns ↔ 13 values (company_id dropped; it defaults to 1 via migration
# 043) → a Payroll row is created and returned (200, $.id). payroll_id is
# captured for cleanup at step 8 (payrolls.employee_id FK-RESTRICTs the
# employee destroy, so the payroll MUST be destroyed before the employee).
POST {{base_url}}/api/v1/hr/payroll/calculate
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "employee_id": {{emp_id}},
  "period_start": "2026-06-01T00:00:00Z",
  "period_end": "2026-06-30T23:59:59Z"
}
```
HTTP 200
[Captures]
payroll_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.employee_id" == {{emp_id}}
jsonpath "$.tenant_id" isNumber
jsonpath "$.status" exists

# === SGK calculate: RBAC + 500 fence (same INSERT bug via create_payroll) ===

# --- 5. RBAC: regular user POST /sgk/calculate → 403 ---
POST {{base_url}}/api/v1/hr/sgk/calculate
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "employee_id": {{emp_id}},
  "period_start": "2026-06-01T00:00:00Z",
  "period_end": "2026-06-30T23:59:59Z"
}
```
HTTP 403
[Asserts]
jsonpath "$.error" exists

# --- 6. RBAC: no-auth POST /sgk/calculate → 401 ---
POST {{base_url}}/api/v1/hr/sgk/calculate
Content-Type: application/json
```
{
  "employee_id": {{emp_id}},
  "period_start": "2026-06-01T00:00:00Z",
  "period_end": "2026-06-30T23:59:59Z"
}
```
HTTP 401
[Asserts]
jsonpath "$.error" exists

# --- 7. Admin POST /sgk/calculate → 200 (fixed: same payrolls INSERT) ---
# calculate_sgk_payroll (sgk/service.rs:211) calls hr_provider.create_payroll
# → the fixed INSERT → a Payroll row is created and returned (200, $.id).
# sgk_payroll_id captured for cleanup at step 8.
POST {{base_url}}/api/v1/hr/sgk/calculate
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "employee_id": {{emp_id}},
  "period_start": "2026-06-01T00:00:00Z",
  "period_end": "2026-06-30T23:59:59Z"
}
```
HTTP 200
[Captures]
sgk_payroll_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.employee_id" == {{emp_id}}

# --- 8. Cleanup: destroy the two calculated payrolls (FK + re-run safety) ---
# payrolls.employee_id REFERENCES employees(id) with no ON DELETE → RESTRICT,
# so the payrolls created in steps 4 & 7 MUST be hard-destroyed BEFORE the
# employee destroy (step 9), or the employee /destroy would 500 on the FK.
# payrolls has no unique constraint and no child FKs → destroy is 204 and
# re-runs do not 409 on re-calculate.
DELETE {{base_url}}/api/v1/hr/payroll/{{payroll_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 204
DELETE {{base_url}}/api/v1/hr/payroll/{{sgk_payroll_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 9. Cleanup: soft-delete + destroy the seeded employee (re-run safety) ---
# DELETE /v1/hr/employees/{id} → soft_delete_employee → 204 (NoContent).
# DELETE /v1/hr/employees/{id}/destroy → destroy_employee → 204 (hard delete,
# frees the non-partial unique employee_number/email slots). The payrolls
# referencing this employee were destroyed in step 8, so no child FK rows
# block the employee destroy.
DELETE {{base_url}}/api/v1/hr/employees/{{emp_id}}
Authorization: Bearer {{admin_token}}
HTTP 204
DELETE {{base_url}}/api/v1/hr/employees/{{emp_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 204