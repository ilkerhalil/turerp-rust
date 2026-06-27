# Tier 2 — Shifts WRITE-FENCE. Admin-only CRUD on /shifts.
# Walks the full soft-delete lifecycle: create → list (decode fence) →
# by-id (decode fence) → update → soft-delete → 404 → restore → 200 →
# destroy → 404. RBAC: regular user POST 403, no-auth POST 401.
#
# ASSUMPTION: the shifts feature flag (tier2.shifts) is ON in this env;
# the read-only 19_shifts.hurl fences the gated (404) path. This file
# exercises the un-gated admin path, so it requires the flag enabled.
# ASSUMPTION: create_shift returns ShiftResponse (ShiftResponse has id,
# tenant_id, name, shift_type, start_time, end_time, break_duration_minutes,
# expected_hours, is_active, created_at, updated_at — no soft-delete fields).
# ASSUMPTION: soft_delete returns 204 empty; restore returns 200 with body.
# ASSUMPTION: after soft-delete, GET by-id returns 404; after restore, 200.
# ASSUMPTION: Decimal fields serialize as JSON strings (rust_decimal default);
# NaiveTime serializes as "HH:MM:SS".

# --- 1. RBAC: regular user POST create → 403 ---
POST {{base_url}}/api/v1/shifts
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "name": "fence-shift-regular",
  "shift_type": "Morning",
  "start_time": "08:00:00",
  "end_time": "16:00:00",
  "break_duration_minutes": 60,
  "expected_hours": "8.0"
}
```
HTTP 403

# --- 2. RBAC: no-auth POST create → 401 ---
POST {{base_url}}/api/v1/shifts
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "name": "fence-shift-noauth",
  "shift_type": "Morning",
  "start_time": "08:00:00",
  "end_time": "16:00:00",
  "break_duration_minutes": 60,
  "expected_hours": "8.0"
}
```
HTTP 401

# --- 3. Admin POST create → 201, capture id ---
POST {{base_url}}/api/v1/shifts
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "name": "fence-shift-admin",
  "shift_type": "Morning",
  "start_time": "08:00:00",
  "end_time": "16:00:00",
  "break_duration_minutes": 60,
  "expected_hours": "8.0"
}
```
HTTP 201
[Captures]
created_id: jsonpath "$.id"

# --- 4. GET list (decode fence) → 200 (paginated {items,total,...}, not a
#      bare Vec — exercising the ShiftResponse Row decode path) ---
GET {{base_url}}/api/v1/shifts
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber

# --- 5. GET by-id (decode fence) → 200 ---
GET {{base_url}}/api/v1/shifts/{{created_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{created_id}}
jsonpath "$.name" == "fence-shift-admin"

# --- 6. PUT update → 200 ---
PUT {{base_url}}/api/v1/shifts/{{created_id}}
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "name": "fence-shift-renamed",
  "shift_type": "Evening",
  "start_time": "16:00:00",
  "end_time": "23:00:00",
  "break_duration_minutes": 30,
  "expected_hours": "7.0",
  "is_active": false
}
```
HTTP 200
[Asserts]
jsonpath "$.name" == "fence-shift-renamed"
jsonpath "$.is_active" == false

# --- 7. DELETE soft-delete → 204 ---
DELETE {{base_url}}/api/v1/shifts/{{created_id}}/soft-delete
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 8. GET by-id after soft-delete → 404 ---
GET {{base_url}}/api/v1/shifts/{{created_id}}
Authorization: Bearer {{admin_token}}
HTTP 404

# --- 9. PUT restore → 200 ---
PUT {{base_url}}/api/v1/shifts/{{created_id}}/restore
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{created_id}}

# --- 10. GET by-id after restore → 200 (decode fence) ---
GET {{base_url}}/api/v1/shifts/{{created_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{created_id}}

# --- 11. DELETE destroy → 204 ---
DELETE {{base_url}}/api/v1/shifts/{{created_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 12. GET by-id after destroy → 404 ---
GET {{base_url}}/api/v1/shifts/{{created_id}}
Authorization: Bearer {{admin_token}}
HTTP 404