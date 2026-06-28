# 87_crm_opportunities_write — WRITE-FENCE for the CRM Opportunity CRUD + status lifecycle.
#
# Covers: POST create → GET list (decode) → GET by-id (decode) → PUT /{id}/status
#         → DELETE soft → GET 404 → PUT /restore → GET 200 → DELETE /destroy
#         → GET 404, plus /deleted (admin-only), pipeline-value decode.
#         RBAC: testuser 403 on create/status; no-auth 401.
#
# Why: 57_crm.hurl only hits the (empty, gated) read paths, so Row decode bugs
# in the create/by-id/update-status/soft-delete/restore/destroy paths stay
# hidden until seeded. This fence self-seeds an opportunity and exercises every
# lifecycle transition.
#
# RBAC: writes use the AdminUser extractor → testuser ({{access_token}}) gets
# 403, admin ({{admin_token}}) gets 200/201. No auth → 401.
# run-all.sh SKIPS this file when admin_token is empty.
#
# ASSUMPTIONS:
# - Status enum serde casing is PascalCase (OpportunityStatus has no
#   rename_all): "Open","Won","Lost","Cancelled".
# - Decimal fields (value, probability) serialize as JSON STRINGS (rust_decimal
#   `serde` feature, confirmed by 74_cari_write.hurl "1000.00"). probability
#   validate() requires 0..=100.
# - tenant_id in the body is overwritten by admin_user.tenant_id, but the field
#   must still deserialize, so {{tenant_id}} is sent.
# - lead_id and customer_id are Optional; sent as null so the create is
#   self-contained (no FK dependency on seeded leads/cari).
# - Soft-delete returns 200 MessageResponse; destroy returns 204.

# --- 1. POST create (admin) → 201; capture id ---
POST {{base_url}}/api/v1/crm/opportunities
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "lead_id": null,
  "name": "Hurl Test Opportunity",
  "customer_id": null,
  "value": "50000.00",
  "probability": "75.00",
  "expected_close_date": null,
  "assigned_to": null,
  "notes": "hurl write-fence seeded opportunity"
}
```
HTTP 201
[Captures]
opp_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.name" == "Hurl Test Opportunity"
jsonpath "$.status" == "Open"
jsonpath "$.value" exists
jsonpath "$.probability" exists
jsonpath "$.tenant_id" isNumber
jsonpath "$.created_at" exists

# --- 2. GET list (admin) — DECODE FENCE: paginated envelope, self-seeded row ---
GET {{base_url}}/api/v1/crm/opportunities
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber
jsonpath "$.total" > 0
jsonpath "$.items[?(@.id=={{opp_id}})].name" contains "Hurl Test Opportunity"

# --- 3. GET by-id (admin) — DECODE FENCE: single-row Opportunity decode ---
GET {{base_url}}/api/v1/crm/opportunities/{{opp_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{opp_id}}
jsonpath "$.name" == "Hurl Test Opportunity"
jsonpath "$.status" == "Open"
jsonpath "$.value" exists
jsonpath "$.probability" exists
jsonpath "$.deleted_at" exists
jsonpath "$.created_at" exists

# --- 4. PUT /{id}/status (admin) → 200; status reflects new value ---
PUT {{base_url}}/api/v1/crm/opportunities/{{opp_id}}/status
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{ "status": "Won" }
```
HTTP 200
[Asserts]
jsonpath "$.id" == {{opp_id}}
jsonpath "$.status" == "Won"

# --- 5. GET status-filter /status/Open (auth) — decode fence, paginated ---
GET {{base_url}}/api/v1/crm/opportunities/status/Open
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber

# --- 6. GET pipeline-value (auth) — 200, bare object { pipeline_value: <Decimal> } ---
# pipeline_value is a rust_decimal::Decimal (sum of opportunity values); the
# crate uses serde (not serde-float) so it serializes as a JSON STRING, not a
# number — assert isString (matches the value/probability convention).
GET {{base_url}}/api/v1/crm/pipeline-value
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.pipeline_value" isString

# --- 7. DELETE soft (admin) → 200 MessageResponse ---
DELETE {{base_url}}/api/v1/crm/opportunities/{{opp_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.message" exists

# --- 8. GET by-id after soft-delete → 404 (fixed: find_by_id filters deleted_at) ---
# crm::find_by_id now filters `deleted_at IS NULL` (postgres_repository.rs:476),
# so a soft-deleted opportunity is hidden → 404. (restore SETs deleted_at = NULL
# before reusing find_by_id, so the filter is safe there.)
GET {{base_url}}/api/v1/crm/opportunities/{{opp_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 9. GET /deleted (admin) — DECODE FENCE: bare Vec<Opportunity>, admin-only ---
GET {{base_url}}/api/v1/crm/opportunities/deleted
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 10. PUT /restore (admin) → 200 ---
PUT {{base_url}}/api/v1/crm/opportunities/{{opp_id}}/restore
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{opp_id}}
jsonpath "$.name" exists

# --- 11. GET by-id after restore → 200 ---
GET {{base_url}}/api/v1/crm/opportunities/{{opp_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{opp_id}}
jsonpath "$.status" == "Won"

# --- 12. DELETE /destroy (admin) → 204 ---
DELETE {{base_url}}/api/v1/crm/opportunities/{{opp_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 13. GET by-id after destroy → 404 ---
GET {{base_url}}/api/v1/crm/opportunities/{{opp_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# === RBAC fences ===

# --- 14. RBAC: POST create as regular user → 403 ---
POST {{base_url}}/api/v1/crm/opportunities
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "lead_id": null,
  "name": "should be forbidden",
  "customer_id": null,
  "value": "1000.00",
  "probability": "50.00",
  "expected_close_date": null,
  "assigned_to": null,
  "notes": null
}
```
HTTP 403

# --- 15. RBAC: PUT /{id}/status as regular user → 403 (extractor before path) ---
PUT {{base_url}}/api/v1/crm/opportunities/999999/status
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{ "status": "Won" }
```
HTTP 403

# --- 16. RBAC: DELETE /{id} as regular user → 403 ---
DELETE {{base_url}}/api/v1/crm/opportunities/999999
Authorization: Bearer {{access_token}}
HTTP 403

# --- 17. RBAC: GET /deleted as regular user → 403 (admin-only) ---
GET {{base_url}}/api/v1/crm/opportunities/deleted
Authorization: Bearer {{access_token}}
HTTP 403

# === No-auth fences (auth runs before RBAC) ===

# --- 18. No auth: POST create → 401 ---
POST {{base_url}}/api/v1/crm/opportunities
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "lead_id": null,
  "name": "should be unauth",
  "customer_id": null,
  "value": "1000.00",
  "probability": "50.00",
  "expected_close_date": null,
  "assigned_to": null,
  "notes": null
}
```
HTTP 401
[Asserts]
jsonpath "$.error" exists

# --- 19. No auth: DELETE /{id} → 401 ---
DELETE {{base_url}}/api/v1/crm/opportunities/999999
HTTP 401
[Asserts]
jsonpath "$.error" exists