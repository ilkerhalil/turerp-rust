# Tier 2 — Approval workflows. templates list is 500 (broken — follow-up
# to #152); instances root is POST-only (start a workflow); /pending
# works and returns a plain JSON array.

# --- 1. Workflow templates — BROKEN (500), assert until fixed ---
GET {{base_url}}/api/v1/workflows/templates
Authorization: Bearer {{access_token}}
HTTP 500

# --- 2. Workflow instances root — POST-only, GET 405 ---
GET {{base_url}}/api/v1/workflows/instances
Authorization: Bearer {{access_token}}
HTTP 405

# --- 3. Pending approvals — 200, plain array ---
GET {{base_url}}/api/v1/workflows/pending
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 4. Non-existent instance — 404 ---
GET {{base_url}}/api/v1/workflows/instances/999999
Authorization: Bearer {{access_token}}
HTTP 404

# --- 5. No auth — 401 ---
GET {{base_url}}/api/v1/workflows/pending
HTTP 401