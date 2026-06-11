# Tier 2 — Payroll. Intentionally NOT implemented in v1 GA (out of
# scope for the 2-week minimal cut). The route does not exist in the
# router; the 404 here documents the deliberate gap, not a regression.
# See docs/superpowers/plans/2026-06-10-production-release.md — payroll
# is Tier 3 in the 2-week cut and is gated behind an explicit feature
# flag once implemented.

# --- 1. Payroll list — does not exist → 404 ---
GET {{base_url}}/api/v1/payroll
Authorization: Bearer {{access_token}}
HTTP 404

# --- 2. Payroll runs — does not exist → 404 ---
GET {{base_url}}/api/v1/payroll/runs
Authorization: Bearer {{access_token}}
HTTP 404

# --- 3. Payslips — does not exist → 404 ---
GET {{base_url}}/api/v1/payroll/payslips
Authorization: Bearer {{access_token}}
HTTP 404

# --- 4. Bonus calculation — does not exist → 404 ---
POST {{base_url}}/api/v1/payroll/calculate
Authorization: Bearer {{access_token}}
Content-Type: application/json
{
  "employee_id": 1,
  "period": "2026-06"
}
HTTP 404
