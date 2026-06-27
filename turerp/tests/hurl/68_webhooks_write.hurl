# 68_webhooks_write — WRITE-FENCE for the Webhook CRUD lifecycle.
#
# Covers: POST create → GET list (decode fence) → GET by-id (decode fence)
#         → PUT update → DELETE soft (200 MessageResponse) → GET by-id 404
#         → POST /restore (200) → GET by-id 200
#         → DELETE /destroy (200 MessageResponse) → GET by-id 404.
#
# Why: 09_webhooks.hurl only hits the list (auth-only, may be empty), the
# /999999 → 404 case, and the no-auth 401. The create/update/soft-delete/
# restore/destroy Row decode paths are never exercised; deleted_by audit
# trail is never checked. This fence self-seeds a throwaway webhook and
# walks every lifecycle transition.
#
# Route shape notes (webhooks differ from the other modules):
#   - soft delete:  DELETE /webhooks/{id}        → 200 MessageResponse
#   - restore:       POST   /webhooks/{id}/restore → 200 MessageResponse
#   - destroy:       DELETE /webhooks/{id}/destroy → 200 MessageResponse
# (NOT 204, and restore is POST not PUT — see src/api/v1/webhooks.rs.)
#
# RBAC: writes use AdminUser → testuser ({{access_token}}) gets 403, admin
# ({{admin_token}}) gets 200/201. No auth → 401.
# run-all.sh SKIPS this file when admin_token is empty.
#
# ASSUMPTION: the webhook URL below points at example.com so no real
# delivery is attempted. The list endpoint returns a Vec (not paginated),
# so the decode fence asserts isCollection on the top-level array.

# --- 1. Create webhook as admin — 201 ---
POST {{base_url}}/api/v1/webhooks
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "url": "https://example.com/hurl-test-webhook",
  "description": "self-seeded by hurl write-fence",
  "event_types": ["invoice.created", "invoice.updated"],
  "secret": "hurl-test-secret"
}
```
HTTP 201
[Captures]
created_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.url" == "https://example.com/hurl-test-webhook"
jsonpath "$.status" isString
jsonpath "$.event_types" isCollection
jsonpath "$.tenant_id" isNumber
jsonpath "$.created_at" exists

# --- 2. GET list — decode fence: top-level Vec + self-seeded item present ---
GET {{base_url}}/api/v1/webhooks
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection
jsonpath "$[?(@.id=={{created_id}})].url" contains "example.com/hurl-test-webhook"

# --- 3. GET by-id — decode fence on single-row path ---
GET {{base_url}}/api/v1/webhooks/{{created_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{created_id}}
jsonpath "$.url" == "https://example.com/hurl-test-webhook"
jsonpath "$.status" isString
jsonpath "$.event_types" isCollection
jsonpath "$.tenant_id" isNumber

# --- 4. PUT update — 200, changed fields reflect new values ---
PUT {{base_url}}/api/v1/webhooks/{{created_id}}
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "url": "https://example.com/hurl-test-webhook-v2",
  "description": "updated by hurl write-fence",
  "event_types": ["invoice.created"],
  "status": "inactive"
}
```
HTTP 200
[Asserts]
jsonpath "$.id" == {{created_id}}
jsonpath "$.url" == "https://example.com/hurl-test-webhook-v2"
jsonpath "$.status" == "inactive"

# --- 5. DELETE soft — 200 (MessageResponse) ---
DELETE {{base_url}}/api/v1/webhooks/{{created_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.message" exists

# --- 6. GET by-id after soft-delete — 404 (soft-deleted filtered out) ---
GET {{base_url}}/api/v1/webhooks/{{created_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 7. POST /restore — 200 (MessageResponse) ---
POST {{base_url}}/api/v1/webhooks/{{created_id}}/restore
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.message" exists

# --- 8. GET by-id after restore — 200 ---
GET {{base_url}}/api/v1/webhooks/{{created_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{created_id}}
jsonpath "$.url" == "https://example.com/hurl-test-webhook-v2"
jsonpath "$.status" == "inactive"

# --- 9. DELETE /destroy — 200 (MessageResponse) ---
DELETE {{base_url}}/api/v1/webhooks/{{created_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.message" exists

# --- 10. GET by-id after destroy — 404 ---
GET {{base_url}}/api/v1/webhooks/{{created_id}}
Authorization: Bearer {{admin_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 11. RBAC: POST create as regular user — 403 ---
POST {{base_url}}/api/v1/webhooks
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "url": "https://example.com/forbidden",
  "description": "should be forbidden",
  "event_types": ["invoice.created"],
  "secret": "x"
}
```
HTTP 403

# --- 12. RBAC: POST create with no auth — 401 ---
POST {{base_url}}/api/v1/webhooks
Content-Type: application/json
```
{
  "url": "https://example.com/noauth",
  "description": "should be unauth",
  "event_types": ["invoice.created"],
  "secret": "x"
}
```
HTTP 401