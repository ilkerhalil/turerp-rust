# Tier 2 — Fixed assets. List works (paginated envelope, empty until
# seeded). /deleted is admin-only (403).

# --- 1. Assets list — 200 ---
GET {{base_url}}/api/v1/assets
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber

# --- 2. Non-existent asset — 404 ---
GET {{base_url}}/api/v1/assets/999999
Authorization: Bearer {{access_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 3. Assets by bogus status — 404 (unknown status filter) ---
GET {{base_url}}/api/v1/assets/status/bogus_status_xyz
Authorization: Bearer {{access_token}}
HTTP 404

# --- 4. Deleted assets — RBAC: regular user 403 ---
GET {{base_url}}/api/v1/assets/deleted
Authorization: Bearer {{access_token}}
HTTP 403

# --- 5. No auth — 401 ---
GET {{base_url}}/api/v1/assets
HTTP 401