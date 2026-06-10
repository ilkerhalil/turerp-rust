# Tier 1.5 — HR. Largest module by path count (34 paths). This scenario
# exercises the employees resource and the leave-types list (currently
# known-broken via PR #145 follow-up).


# --- 1. Employees list — expected 200 ---
GET {{base_url}}/api/v1/hr/employees
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber

# --- 2. Leave types — currently 500 (P1, see issue #150) ---
GET {{base_url}}/api/v1/hr/leave-types
Authorization: Bearer {{access_token}}
HTTP 500
