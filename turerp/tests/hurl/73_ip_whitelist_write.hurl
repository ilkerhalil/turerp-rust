# Tier 2 — IP Whitelist WRITE-FENCE.
#
# Exercises POST /ip-whitelist, GET list (bare array, NOT paginated
# envelope), GET /{id}, PUT /{id}, DELETE /{id} (hard delete, 204 — no
# soft/restore routes exist for this module). The read-only
# 42_ip_whitelist.hurl fence only covers the RBAC 403/401 fence. Requires
# {{admin_token}} (run-all.sh skips *_write.hurl when admin unavailable).
#
# Decode fences: step 2 (list Vec<IpWhitelistEntryResponse> decode — root is
# a JSON array, not {items,...}), step 3 (single-row decode).
#
# ASSUMPTION: suite runs against a freshly reset DB. The static CIDR
# `10.99.99.0/24` is unique per tenant; a second run against the same DB
# may 409 at step 1 (depends on whether the service enforces a uniqueness
# constraint on (tenant_id, ip_address) — not verified here). Re-run
# safety relies on DB reset between full-suite runs.

# --- 1. POST create (admin) → 201; capture id ---
POST {{base_url}}/api/v1/ip-whitelist
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "ip_address": "10.99.99.0/24",
  "description": "hurl write-fence entry",
  "is_active": true
}
```
HTTP 201
[Captures]
created_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.ip_address" == "10.99.99.0/24"
jsonpath "$.is_active" == true

# --- 2. GET list (admin) — DECODE FENCE: root is a bare JSON array ---
GET {{base_url}}/api/v1/ip-whitelist
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection
jsonpath "$[0].id" isNumber

# --- 3. GET by-id (admin) — DECODE FENCE: single-row IpWhitelistEntryResponse ---
GET {{base_url}}/api/v1/ip-whitelist/{{created_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{created_id}}
jsonpath "$.tenant_id" isNumber
jsonpath "$.ip_address" isString
jsonpath "$.description" exists
jsonpath "$.is_active" isBoolean
jsonpath "$.created_at" exists

# --- 4. PUT update by-id (admin) → 200; assert changed description + is_active ---
PUT {{base_url}}/api/v1/ip-whitelist/{{created_id}}
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{ "description": "updated by write-fence", "is_active": false }
```
HTTP 200
[Asserts]
jsonpath "$.description" == "updated by write-fence"
jsonpath "$.is_active" == false

# --- 5. DELETE hard-delete by-id (admin) → 204 ---
DELETE {{base_url}}/api/v1/ip-whitelist/{{created_id}}
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 6. GET by-id → 404 (hard-deleted) ---
GET {{base_url}}/api/v1/ip-whitelist/{{created_id}}
Authorization: Bearer {{admin_token}}
HTTP 404

# --- 7. RBAC: POST create with regular user → 403 ---
POST {{base_url}}/api/v1/ip-whitelist
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{ "tenant_id": {{tenant_id}}, "ip_address": "10.99.99.1/32", "description": "rbac" }
```
HTTP 403

# --- 8. No auth POST → 401 ---
POST {{base_url}}/api/v1/ip-whitelist
Content-Type: application/json
```
{ "tenant_id": {{tenant_id}}, "ip_address": "10.99.99.2/32", "description": "noauth" }
```
HTTP 401