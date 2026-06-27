# Tier 2 — CRM (leads + opportunities + campaigns + tickets).
# Read-leaning smoke-GET fence: list endpoints (paginated envelope),
# status-filter paths, aggregate GETs (pipeline-value, open-count),
# by-id 404, /deleted RBAC (admin-only → testuser 403), no-auth 401.
# Why: crm module had ZERO hurl coverage; lists may be empty unseeded but
# 200 proves the paginated SQL + Row decode runs (see memory:
# list endpoints hide decode bugs when empty).

# --- 1. Leads list — 200, paginated envelope ---
GET {{base_url}}/api/v1/crm/leads
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber

# --- 2. Opportunities list — 200, paginated envelope ---
GET {{base_url}}/api/v1/crm/opportunities
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber

# --- 3. Campaigns list — 200, paginated envelope ---
GET {{base_url}}/api/v1/crm/campaigns
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber

# --- 4. Tickets list — 200, paginated envelope ---
GET {{base_url}}/api/v1/crm/tickets
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber

# --- 5. Pipeline value aggregate — 200, bare object { pipeline_value: n } ---
GET {{base_url}}/api/v1/crm/pipeline-value
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.pipeline_value" isNumber

# --- 6. Open tickets count aggregate — 200, bare object { open_tickets_count: n } ---
GET {{base_url}}/api/v1/crm/tickets/open-count
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.open_tickets_count" isNumber

# --- 7. Leads by status (valid enum "New") — 200, paginated envelope ---
GET {{base_url}}/api/v1/crm/leads/status/New
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber

# --- 8. Opportunities by status (valid enum "Open") — 200 ---
GET {{base_url}}/api/v1/crm/opportunities/status/Open
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber

# --- 9. Campaigns by status (valid enum "Active") — 200 ---
GET {{base_url}}/api/v1/crm/campaigns/status/Active
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber

# --- 10. Tickets by status (valid enum "Open") — 200 ---
GET {{base_url}}/api/v1/crm/tickets/status/Open
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber

# --- 11. Leads by INVALID status — 400 (path enum deserialize fails) ---
GET {{base_url}}/api/v1/crm/leads/status/NotAStatus
Authorization: Bearer {{access_token}}
HTTP 400

# --- 12. Non-existent lead by id — 404 ---
GET {{base_url}}/api/v1/crm/leads/999999
Authorization: Bearer {{access_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 13. Non-existent opportunity by id — 404 ---
GET {{base_url}}/api/v1/crm/opportunities/999999
Authorization: Bearer {{access_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 14. Non-existent campaign by id — 404 ---
GET {{base_url}}/api/v1/crm/campaigns/999999
Authorization: Bearer {{access_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 15. Non-existent ticket by id — 404 ---
GET {{base_url}}/api/v1/crm/tickets/999999
Authorization: Bearer {{access_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 16. Deleted leads — RBAC: admin-only (AdminUser) → testuser 403 ---
GET {{base_url}}/api/v1/crm/leads/deleted
Authorization: Bearer {{access_token}}
HTTP 403

# --- 17. Deleted opportunities — RBAC: admin-only → testuser 403 ---
GET {{base_url}}/api/v1/crm/opportunities/deleted
Authorization: Bearer {{access_token}}
HTTP 403

# --- 18. Deleted campaigns — RBAC: admin-only → testuser 403 ---
GET {{base_url}}/api/v1/crm/campaigns/deleted
Authorization: Bearer {{access_token}}
HTTP 403

# --- 19. Deleted tickets — RBAC: admin-only → testuser 403 ---
GET {{base_url}}/api/v1/crm/tickets/deleted
Authorization: Bearer {{access_token}}
HTTP 403

# --- 20. No auth on leads list — 401 ---
GET {{base_url}}/api/v1/crm/leads
HTTP 401
[Asserts]
jsonpath "$.error" exists

# --- 21. No auth on pipeline-value — 401 ---
GET {{base_url}}/api/v1/crm/pipeline-value
HTTP 401
[Asserts]
jsonpath "$.error" exists

# --- 22. No auth on tickets open-count — 401 ---
GET {{base_url}}/api/v1/crm/tickets/open-count
HTTP 401
[Asserts]
jsonpath "$.error" exists

# --- 23. No auth on deleted leads — 401 (auth runs before RBAC) ---
GET {{base_url}}/api/v1/crm/leads/deleted
HTTP 401
[Asserts]
jsonpath "$.error" exists