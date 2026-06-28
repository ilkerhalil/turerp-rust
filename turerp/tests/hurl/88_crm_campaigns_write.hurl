# 88_crm_campaigns_write — WRITE-FENCE for the CRM Campaign CRUD + status lifecycle.
#
# Covers: POST create → GET list (decode) → GET by-id (decode) → PUT /{id}/status
#         → DELETE soft → GET 404 → PUT /restore → GET 200 → DELETE /destroy
#         → GET 404, plus /deleted (admin-only), status-filter decode.
#         RBAC: testuser 403 on create/status; no-auth 401.
#
# Why: 57_crm.hurl only hits the (empty, gated) read paths, so Row decode bugs
# in the create/by-id/update-status/soft-delete/restore/destroy paths stay
# hidden until seeded. This fence self-seeds a campaign and exercises every
# lifecycle transition.
#
# RBAC: writes use the AdminUser extractor → testuser ({{access_token}}) gets
# 403, admin ({{admin_token}}) gets 200/201. No auth → 401.
# run-all.sh SKIPS this file when admin_token is empty.
#
# ASSUMPTIONS:
# - Status enum serde casing is PascalCase (CampaignStatus has no rename_all):
#   "Draft","Scheduled","Active","Completed","Cancelled".
# - Decimal field (budget) serializes as a JSON STRING (rust_decimal `serde`
#   feature, confirmed by 74_cari_write.hurl "1000.00"). validate() requires
#   budget >= 0; actual_cost is server-side (defaults to 0), NOT in CreateCampaign.
# - tenant_id in the body is overwritten by admin_user.tenant_id, but the field
#   must still deserialize, so {{tenant_id}} is sent.
# - CreateCampaign has NO actual_cost field (server defaults it); sending one
#   would 400 on unknown field (serde defaults deny unknown fields? actix
#   web::Json uses serde default which IGNORES unknown fields — safe either way;
#   omitted to match the struct exactly).
# - Soft-delete returns 200 MessageResponse; destroy returns 204.

# --- 1. POST create (admin) → 201; capture id ---
POST {{base_url}}/api/v1/crm/campaigns
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "name": "Hurl Test Campaign",
  "description": "hurl write-fence seeded campaign",
  "campaign_type": "Email",
  "budget": "10000.00",
  "start_date": null,
  "end_date": null
}
```
HTTP 201
[Captures]
campaign_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.name" == "Hurl Test Campaign"
jsonpath "$.campaign_type" == "Email"
jsonpath "$.status" == "Draft"
jsonpath "$.budget" exists
jsonpath "$.tenant_id" isNumber
jsonpath "$.created_at" exists

# --- 2. GET list (admin) — DECODE FENCE: paginated envelope, self-seeded row ---
GET {{base_url}}/api/v1/crm/campaigns
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber
jsonpath "$.total" > 0
jsonpath "$.items[?(@.id=={{campaign_id}})].name" contains "Hurl Test Campaign"

# --- 3. GET by-id (admin) — DECODE FENCE: single-row Campaign decode ---
GET {{base_url}}/api/v1/crm/campaigns/{{campaign_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{campaign_id}}
jsonpath "$.name" == "Hurl Test Campaign"
jsonpath "$.status" == "Draft"
jsonpath "$.campaign_type" == "Email"
jsonpath "$.budget" exists
jsonpath "$.actual_cost" exists
jsonpath "$.deleted_at" exists
jsonpath "$.created_at" exists

# --- 4. PUT /{id}/status (admin) → 200; status reflects new value ---
PUT {{base_url}}/api/v1/crm/campaigns/{{campaign_id}}/status
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{ "status": "Active" }
```
HTTP 200
[Asserts]
jsonpath "$.id" == {{campaign_id}}
jsonpath "$.status" == "Active"

# --- 5. GET status-filter /status/Active (auth) — decode fence, paginated ---
GET {{base_url}}/api/v1/crm/campaigns/status/Active
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber

# --- 6. DELETE soft (admin) → 200 MessageResponse ---
DELETE {{base_url}}/api/v1/crm/campaigns/{{campaign_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.message" exists

# --- 7. GET by-id after soft-delete → 404 (fixed: find_by_id filters deleted_at) ---
# crm::find_by_id now filters `deleted_at IS NULL` (postgres_repository.rs:828),
# so a soft-deleted campaign is hidden → 404. (restore SETs deleted_at = NULL
# before reusing find_by_id, so the filter is safe there.)
GET {{base_url}}/api/v1/crm/campaigns/{{campaign_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 8. GET /deleted (admin) — DECODE FENCE: bare Vec<Campaign>, admin-only ---
GET {{base_url}}/api/v1/crm/campaigns/deleted
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 9. PUT /restore (admin) → 200 ---
PUT {{base_url}}/api/v1/crm/campaigns/{{campaign_id}}/restore
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{campaign_id}}
jsonpath "$.name" exists

# --- 10. GET by-id after restore → 200 ---
GET {{base_url}}/api/v1/crm/campaigns/{{campaign_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{campaign_id}}
jsonpath "$.status" == "Active"

# --- 11. DELETE /destroy (admin) → 204 ---
DELETE {{base_url}}/api/v1/crm/campaigns/{{campaign_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 12. GET by-id after destroy → 404 ---
GET {{base_url}}/api/v1/crm/campaigns/{{campaign_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# === RBAC fences ===

# --- 13. RBAC: POST create as regular user → 403 ---
POST {{base_url}}/api/v1/crm/campaigns
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "name": "should be forbidden",
  "campaign_type": "Email",
  "budget": "1000.00",
  "start_date": null,
  "end_date": null
}
```
HTTP 403

# --- 14. RBAC: PUT /{id}/status as regular user → 403 (extractor before path) ---
PUT {{base_url}}/api/v1/crm/campaigns/999999/status
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{ "status": "Active" }
```
HTTP 403

# --- 15. RBAC: DELETE /{id} as regular user → 403 ---
DELETE {{base_url}}/api/v1/crm/campaigns/999999
Authorization: Bearer {{access_token}}
HTTP 403

# --- 16. RBAC: GET /deleted as regular user → 403 (admin-only) ---
GET {{base_url}}/api/v1/crm/campaigns/deleted
Authorization: Bearer {{access_token}}
HTTP 403

# === No-auth fences (auth runs before RBAC) ===

# --- 17. No auth: POST create → 401 ---
POST {{base_url}}/api/v1/crm/campaigns
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "name": "should be unauth",
  "campaign_type": "Email",
  "budget": "1000.00",
  "start_date": null,
  "end_date": null
}
```
HTTP 401
[Asserts]
jsonpath "$.error" exists

# --- 18. No auth: DELETE /{id} → 401 ---
DELETE {{base_url}}/api/v1/crm/campaigns/999999
HTTP 401
[Asserts]
jsonpath "$.error" exists