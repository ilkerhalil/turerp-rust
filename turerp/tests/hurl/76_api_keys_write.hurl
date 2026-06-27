# Tier 2 — API keys WRITE-FENCE. Admin-only CRUD with tenant_id in path.
# Captures the created id and plain_key, then walks the full soft-delete
# lifecycle: create → list (decode fence) → by-id (decode fence) → update →
# soft-delete → 404 → restore → 200 → destroy → 404. RBAC: regular user
# POST 403, no-auth POST 401.
#
# ASSUMPTION: soft-delete returns 204 with empty body (per utoipa spec).
# ASSUMPTION: restore returns 204 with empty body (per utoipa spec).
# ASSUMPTION: after soft-delete, GET by-id returns 404 (service filters
# deleted rows out of normal reads); after /restore the row is visible again.
# ASSUMPTION: /destroy on a hard-deleted row returns 404; we do not re-destroy.
# ASSUMPTION: check-scope uses the X-API-Key header with the captured
# plain_key (src/middleware/api_key.rs reads X-API-Key).

# --- 1. RBAC: regular user POST create → 403 ---
POST {{base_url}}/api/v1/api-keys
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "name": "fence-key-regular",
  "tenant_id": {{tenant_id}},
  "user_id": {{user_id}},
  "scopes": ["all"],
  "expires_at": null
}
```
HTTP 403

# --- 2. RBAC: no-auth POST create → 401 ---
POST {{base_url}}/api/v1/api-keys
Content-Type: application/json
```
{
  "name": "fence-key-noauth",
  "tenant_id": {{tenant_id}},
  "user_id": {{user_id}},
  "scopes": ["all"],
  "expires_at": null
}
```
HTTP 401

# --- 3. Admin POST create → 201, capture id + plain_key ---
POST {{base_url}}/api/v1/api-keys
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "name": "fence-key-admin",
  "tenant_id": {{tenant_id}},
  "user_id": {{user_id}},
  "scopes": ["cari:read", "invoice:write"],
  "expires_at": null
}
```
HTTP 201
[Captures]
created_id: jsonpath "$.api_key.id"
plain_key: jsonpath "$.plain_key"

# --- 4. GET list (decode fence) → 200 ---
GET {{base_url}}/api/v1/api-keys/tenant/{{tenant_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$[*]" isCollection

# --- 5. GET by-id (decode fence) → 200 ---
GET {{base_url}}/api/v1/api-keys/{{created_id}}/tenant/{{tenant_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{created_id}}
jsonpath "$.name" == "fence-key-admin"

# --- 6. PUT update → 200 ---
PUT {{base_url}}/api/v1/api-keys/{{created_id}}/tenant/{{tenant_id}}
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "name": "fence-key-renamed",
  "scopes": ["all"],
  "is_active": false
}
```
HTTP 200
[Asserts]
jsonpath "$.name" == "fence-key-renamed"
jsonpath "$.is_active" == false

# --- 7. check-scope with the captured plain_key → 200 ---
GET {{base_url}}/api/v1/api-keys/check-scope/cari:read
X-API-Key: {{plain_key}}
HTTP 200
[Asserts]
jsonpath "$.has_access" == true

# --- 8. DELETE soft → 204 ---
DELETE {{base_url}}/api/v1/api-keys/{{created_id}}/tenant/{{tenant_id}}/soft
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 9. GET by-id after soft-delete → 404 ---
GET {{base_url}}/api/v1/api-keys/{{created_id}}/tenant/{{tenant_id}}
Authorization: Bearer {{admin_token}}
HTTP 404

# --- 10. POST restore → 204 ---
POST {{base_url}}/api/v1/api-keys/{{created_id}}/tenant/{{tenant_id}}/restore
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 11. GET by-id after restore → 200 (decode fence) ---
GET {{base_url}}/api/v1/api-keys/{{created_id}}/tenant/{{tenant_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{created_id}}

# --- 12. DELETE destroy → 204 ---
DELETE {{base_url}}/api/v1/api-keys/{{created_id}}/tenant/{{tenant_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 13. GET by-id after destroy → 404 ---
GET {{base_url}}/api/v1/api-keys/{{created_id}}/tenant/{{tenant_id}}
Authorization: Bearer {{admin_token}}
HTTP 404

# --- 14. Cleanup: re-destroy on already-destroyed row → 404 (idempotency probe) ---
DELETE {{base_url}}/api/v1/api-keys/{{created_id}}/tenant/{{tenant_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 404