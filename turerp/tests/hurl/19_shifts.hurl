# Tier 2 — Shifts. Module includes shifts, assignments, and
# attendance clock-in/clock-out. All gated behind tier2.shifts
# (default OFF) so we can roll it on for pilot customers without
# touching the wider HR module.

# --- 1. Shifts list — gated (default OFF) → 404 ---
GET {{base_url}}/api/v1/shifts
Authorization: Bearer {{access_token}}
HTTP 404

# --- 2. Shift detail — gated → 404 ---
GET {{base_url}}/api/v1/shifts/1
Authorization: Bearer {{access_token}}
HTTP 404

# --- 3. Shift assignments — gated → 404 ---
GET {{base_url}}/api/v1/shifts/assignments
Authorization: Bearer {{access_token}}
HTTP 404

# --- 4. Attendance clock-in (POST) — gated → 404 ---
POST {{base_url}}/api/v1/shifts/attendance/clock-in
Authorization: Bearer {{access_token}}
Content-Type: application/json
{
  "employee_id": 1,
  "timestamp": "2026-06-08T09:00:00Z"
}
HTTP 404

# --- 5. Overtime report — gated → 404 ---
GET {{base_url}}/api/v1/shifts/overtime
Authorization: Bearer {{access_token}}
HTTP 404
