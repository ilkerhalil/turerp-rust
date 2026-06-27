# 83_workflows_write — WRITE-FENCE for the Workflow template + instance
# approval lifecycle.
#
# Covers:
#   /workflows/templates  — POST create (admin) → GET list (decode array)
#   /workflows/instances  — POST start (authed) → GET /{id} detail (decode)
#                           → POST /{id}/approve → POST /{id}/reject (on a
#                           second instance) → POST /{id}/resubmit → GET
#                           /{id}/audit (decode array) → GET /pending (decode).
#
# Why: the read-only 51_workflows.hurl only hits the (empty) templates list,
# POST-only instances root (405), /pending array, and by-id 404. The
# create/start/approve/reject/resubmit/audit decode paths stay hidden until
# seeded. This fence self-seeds a template, starts two instances, and
# exercises the full approval + rejection + resubmission lifecycle.
#
# RBAC: create_template uses AdminUser → testuser ({{access_token}}) gets
# 403, admin ({{admin_token}}) gets 201. start_workflow / approve / reject
# / resubmit / get_instance / audit / pending use AuthUser → any authed
# 200. run-all.sh SKIPS this file when admin_token is empty.
#
# FK STRATEGY: start_workflow needs template_id → self-seed a template
# (step 1) and capture its id. The template config_json `steps` array
# sets approver_user_id explicitly to {{user_id}} (testuser) so the step is
# deterministically assigned without relying on role-based user lookup
# (which requires exactly 1 user with the given role). entity_id is a
# free-form i64; we use a static sentinel (9991) — no FK validation is
# performed by the workflow service.
#
# ASSUMPTION: WorkflowEntityType / WorkflowStatus / WorkflowStepStatus use
# serde rename_all = "snake_case", so entity_type is sent as "invoice" and
# status asserts as "pending" / "approved" / "rejected" / "completed".
# ASSUMPTION: approve_step does NOT verify the caller matches the
# assigned approver_user_id (service only checks instance+step are
# Pending) — so admin can approve a step assigned to testuser. If a future
# change adds assignee matching, the approve step will 400 and this fence
# must be updated to approve as {{user_id}}. ASSUMPTION: template name is
# unique per tenant; a second run against the same DB may 409 at step 1 —
# re-run safety relies on DB reset between full-suite runs.

# --- 1. Create workflow template (admin) → 201; capture id ---
POST {{base_url}}/api/v1/workflows/templates
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "name": "Hurl Write-Fence Template",
  "description": "Self-seeded template for hurl write-fence",
  "entity_type": "invoice",
  "config_json": {
    "steps": [
      {
        "step_number": 1,
        "step_name": "Manager Review",
        "approver_role": "user",
        "approver_user_id": {{user_id}}
      },
      {
        "step_number": 2,
        "step_name": "Admin Approval",
        "approver_role": "admin",
        "approver_user_id": {{user_id}}
      }
    ]
  }
}
```
HTTP 201
[Captures]
template_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.name" == "Hurl Write-Fence Template"
jsonpath "$.entity_type" == "invoice"
jsonpath "$.is_active" == true
jsonpath "$.config_json" exists
jsonpath "$.created_at" exists

# --- 2. GET templates list (authed) — DECODE FENCE: plain Vec ---
GET {{base_url}}/api/v1/workflows/templates
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection
jsonpath "$[?(@.id=={{template_id}})].name" contains "Hurl Write-Fence Template"

# --- 3. Start workflow instance A (authed) → 201; capture id (approve path) ---
POST {{base_url}}/api/v1/workflows/instances
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "template_id": {{template_id}},
  "entity_id": 9991,
  "entity_type": "invoice"
}
```
HTTP 201
[Captures]
instance_a_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.template_id" == {{template_id}}
jsonpath "$.entity_id" == 9991
jsonpath "$.entity_type" == "invoice"
jsonpath "$.status" == "pending"
jsonpath "$.current_step" isNumber
jsonpath "$.assigned_user_id" exists
jsonpath "$.created_by" isNumber

# --- 4. GET instance A detail (authed) — DECODE FENCE: WorkflowInstanceDetailResponse (flattened) ---
GET {{base_url}}/api/v1/workflows/instances/{{instance_a_id}}
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{instance_a_id}}
jsonpath "$.status" == "pending"
jsonpath "$.steps" isCollection
jsonpath "$.steps[0].status" == "pending"
jsonpath "$.audit_log" isCollection

# --- 5. POST /approve (authed) → 200; step 1 approved, status stays pending (multi-step) ---
POST {{base_url}}/api/v1/workflows/instances/{{instance_a_id}}/approve
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{ "comment": "approved by hurl write-fence" }
```
HTTP 200
[Asserts]
jsonpath "$.id" == {{instance_a_id}}
jsonpath "$.status" exists

# --- 6. GET audit trail (authed) — DECODE FENCE: plain Vec<WorkflowAuditLogResponse> ---
GET {{base_url}}/api/v1/workflows/instances/{{instance_a_id}}/audit
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection
jsonpath "$[0].action" exists
jsonpath "$[0].user_id" isNumber
jsonpath "$[0].timestamp" exists

# --- 7. Start workflow instance B (authed) → 201; capture id (reject path) ---
POST {{base_url}}/api/v1/workflows/instances
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "template_id": {{template_id}},
  "entity_id": 9992,
  "entity_type": "invoice"
}
```
HTTP 201
[Captures]
instance_b_id: jsonpath "$.id"
[Asserts]
jsonpath "$.status" == "pending"

# --- 8. POST /reject on instance B (authed) → 200; status → rejected ---
POST {{base_url}}/api/v1/workflows/instances/{{instance_b_id}}/reject
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{ "comment": "rejected by hurl write-fence" }
```
HTTP 200
[Asserts]
jsonpath "$.id" == {{instance_b_id}}
jsonpath "$.status" == "rejected"

# --- 9. POST /resubmit on instance B (authed) → 200; status → pending ---
POST {{base_url}}/api/v1/workflows/instances/{{instance_b_id}}/resubmit
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{instance_b_id}}
jsonpath "$.status" == "pending"

# --- 10. GET /pending (authed) — DECODE FENCE: plain Vec ---
GET {{base_url}}/api/v1/workflows/pending
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 11. GET non-existent instance → 404 ---
GET {{base_url}}/api/v1/workflows/instances/999999
Authorization: Bearer {{access_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 12. RBAC: POST create template as regular user → 403 ---
POST {{base_url}}/api/v1/workflows/templates
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "name": "should be forbidden",
  "description": "rbac 403",
  "entity_type": "invoice",
  "config_json": { "steps": [ { "step_number": 1, "step_name": "X", "approver_user_id": {{user_id}} } ] }
}
```
HTTP 403

# --- 13. RBAC: POST create template with no auth → 401 ---
POST {{base_url}}/api/v1/workflows/templates
Content-Type: application/json
```
{
  "name": "should be unauth",
  "description": "no auth",
  "entity_type": "invoice",
  "config_json": { "steps": [ { "step_number": 1, "step_name": "X", "approver_user_id": {{user_id}} } ] }
}
```
HTTP 401

# --- 14. No auth GET /pending → 401 ---
GET {{base_url}}/api/v1/workflows/pending
HTTP 401