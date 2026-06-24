# Tier 2 — Data archiving (policies, jobs, records). All list endpoints
# work for a regular user (paginated envelopes).

# --- 1. Archive policies list — 200 ---
GET {{base_url}}/api/v1/archive/policies
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber

# --- 2. Active policies — 200 ---
GET {{base_url}}/api/v1/archive/policies/active
Authorization: Bearer {{access_token}}
HTTP 200

# --- 3. Archive jobs list — 200 ---
GET {{base_url}}/api/v1/archive/jobs
Authorization: Bearer {{access_token}}
HTTP 200

# --- 4. Archived records list — 200 ---
GET {{base_url}}/api/v1/archive/records
Authorization: Bearer {{access_token}}
HTTP 200

# --- 5. Non-existent policy — 404 ---
GET {{base_url}}/api/v1/archive/policies/999999
Authorization: Bearer {{access_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 6. No auth — 401 ---
GET {{base_url}}/api/v1/archive/policies
HTTP 401