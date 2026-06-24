# Tier 1.5 — Cost centers. Seeded (cost_centers table). The DB column is
# `type` (migration 023); the repository now aliases `type AS center_type`
# in SELECT/RETURNING so CostCenterRow decodes. List MUST be non-empty.

# --- 1. Cost centers list — seeded, non-empty paginated envelope ---
GET {{base_url}}/api/v1/cost-centers
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber
jsonpath "$.total" > 0

# --- 2. Filter by center_type=profit — only Profit rows ---
GET {{base_url}}/api/v1/cost-centers?center_type=profit
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.items[*].center_type" contains "Profit"

# --- 3. Non-existent cost center — 404 ---
GET {{base_url}}/api/v1/cost-centers/999999
Authorization: Bearer {{access_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 4. No auth — 401 ---
GET {{base_url}}/api/v1/cost-centers
HTTP 401