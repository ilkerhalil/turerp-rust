# 93_hr_payroll_sgk_write — MINIMAL RBAC + 500-FENCE for Payroll + SGK calculate.
#
# Scope (reduced from a full lifecycle fence): self-seed an employee, then
# fence the RBAC layer (403 regular / 401 no-auth) and the KNOWN 500 on the
# admin calculate path for BOTH /hr/payroll/calculate and /hr/sgk/calculate.
#
# Why minimal: the admin calculate path 500s on a parse-time SQL error, so it
# creates NO row — there is nothing to GET/PAY/SOFT-DELETE/RESTORE/DESTROY.
# Fencing the full payroll/sgk lifecycle (paid, deleted, soft-delete, restore,
# destroy, register, bonus, summary, ebildirge) is deferred until the INSERT
# bug is fixed; those steps would all be unreachable (no payroll_id captured).
#
# KNOWN BUG (residual, source-side): PostgresPayrollRepository::create
# (src/domain/hr/postgres_repository.rs:1200) issues
#   INSERT INTO payrolls (tenant_id, company_id, employee_id, period_start,
#     period_end, basic_salary, overtime_hours, overtime_pay, bonuses,
#     deductions, net_salary, status, paid_at, created_at)            -- 14 cols
#   VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, NOW())   -- 13 exprs
# 14 target columns vs 13 value expressions → PostgreSQL parse error
# ("INSERT has more target columns than expressions"), which fails at PARSE
# time before any bind. The bind list is ALSO misaligned (12 .bind() calls
# for $1..$12 map tenant_id→$1, employee_id→$2, period_start→$3, … so the
# company_id column slot would receive employee_id's value), but the parse
# error fires first, so the misalignment is moot. Either way → HTTP 500.
# sqlx::Error → map_sqlx_error → ApiError::Database → 500.
# calculate_payroll (hr/service.rs) and calculate_sgk_payroll
# (hr/sgk/service.rs:211, calls hr_provider.create_payroll) both hit this.
# Fix = drop the company_id column from the INSERT (or add a $-placeholder +
# bind for it). Trivial source fix, deferred to a dedicated bug-fix PR
# (one-bug-per-PR convention; #170-#173 precedent). This file documents the
# 500 so the bug is visible in CI instead of silently uncovered.
#
# RBAC: payroll/sgk calculate use the AdminUser extractor → testuser
# ({{access_token}}) gets 403, no-auth gets 401 (JwtAuth runs first), admin
# ({{admin_token}}) reaches the handler and 500s. run-all.sh SKIPS this file
# when admin_token is empty.
#
# FK STRATEGY: payroll.employee_id + sgk.employee_id → self-seed an employee
# (POST /hr/employees, admin) and capture emp_id. The employee is cleaned up
# (soft-delete + destroy) at the end; employees(employee_number, email,
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

# --- 4. Admin POST /payroll/calculate → 500 (KNOWN BUG: payrolls INSERT) ---
# Admin passes the extractor and reaches create_payroll → the buggy INSERT
# (14 cols / 13 values) → ApiError::Database → 500. No payroll row is
# created, so the rest of the payroll lifecycle is unfenceable until the
# source bug is fixed. Asserting the current (broken) behavior.
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
HTTP 500
[Asserts]
jsonpath "$.error" exists

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

# --- 7. Admin POST /sgk/calculate → 500 (KNOWN BUG: same payrolls INSERT) ---
# calculate_sgk_payroll (sgk/service.rs:211) calls hr_provider.create_payroll
# → the same buggy INSERT → 500. No sgk/payroll row is created.
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
HTTP 500
[Asserts]
jsonpath "$.error" exists

# --- 8. Cleanup: soft-delete + destroy the seeded employee (re-run safety) ---
# DELETE /v1/hr/employees/{id} → soft_delete_employee → 204 (NoContent).
# DELETE /v1/hr/employees/{id}/destroy → destroy_employee → 204 (hard delete,
# frees the non-partial unique employee_number/email slots). No payroll/sgk
# rows were created (calculate 500s), so there are no child FK rows to block
# the employee destroy.
DELETE {{base_url}}/api/v1/hr/employees/{{emp_id}}
Authorization: Bearer {{admin_token}}
HTTP 204
DELETE {{base_url}}/api/v1/hr/employees/{{emp_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 204