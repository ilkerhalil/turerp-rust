# Tier 2 — Projects. WBS (work breakdown structure) and project
# costs are gated behind tier2.projects (default OFF). Tier-1
# smoke verifies the gate fires for every collection route.

# --- 1. Projects list — gated (default OFF) → 404 ---
GET {{base_url}}/api/v1/projects
Authorization: Bearer {{access_token}}
HTTP 404

# --- 2. Project detail — gated → 404 ---
GET {{base_url}}/api/v1/projects/1
Authorization: Bearer {{access_token}}
HTTP 404

# --- 3. WBS list — gated → 404 ---
GET {{base_url}}/api/v1/projects/wbs
Authorization: Bearer {{access_token}}
HTTP 404

# --- 4. Project costs (cross-module with accounting) — gated → 404 ---
GET {{base_url}}/api/v1/projects/costs
Authorization: Bearer {{access_token}}
HTTP 404

# --- 5. Profitability report — gated → 404 ---
GET {{base_url}}/api/v1/projects/1/profitability
Authorization: Bearer {{access_token}}
HTTP 404
