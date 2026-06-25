# Tier 2 — Approval workflows. templates list is 200 (was 500 —
# workflow_templates, created in migration 024, lacked the
# deleted_at/deleted_by soft-delete columns that WorkflowTemplateRow SELECTs
# and filters on; fixed by migration 046, mirroring the 040/044/045
# soft-delete pattern). instances root is POST-only (start a workflow);
# /pending works and returns a plain JSON array.

# --- 1. Workflow templates — 200 (was 500; plain JSON array) ---
GET {{base_url}}/api/v1/workflows/templates
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

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