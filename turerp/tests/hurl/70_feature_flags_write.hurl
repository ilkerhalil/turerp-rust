# Tier 2 — Feature Flags WRITE-FENCE.
#
# Exercises the create / update / soft-delete / restore / destroy +
# enable / disable / check paths that the read-only 39_feature_flags.hurl
# fence does not cover. Requires {{admin_token}} (run-all.sh skips
# *_write.hurl when admin is unavailable).
#
# Decode fences: step 2 (paginated list Row decode), step 3 (single-row
# decode), step 7 (FeatureStatusResponse decode).
#
# ASSUMPTION: suite runs against a freshly reset DB. The static flag name
# `wf_test_flag` is unique per tenant; a second run against the same DB will
# 409 at step 1. Re-run safety relies on DB reset between full-suite runs.
# ASSUMPTION: get_flag_by_id filters soft-deleted rows so step 9 / 13 return
# 404 after soft-delete / destroy (the service uses a soft-delete-aware
# filter; not verified by sqlx::test here).

# --- 1. POST create (admin) → 201; capture id ---
POST {{base_url}}/api/v1/feature-flags
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{
  "name": "wf_test_flag",
  "description": "hurl write-fence seeded flag",
  "status": "disabled",
  "tenant_id": {{tenant_id}}
}
```
HTTP 201
[Captures]
created_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.name" == "wf_test_flag"
jsonpath "$.status" == "disabled"

# --- 2. GET list (admin) — DECODE FENCE: paginated envelope, self-seeded row present ---
GET {{base_url}}/api/v1/feature-flags
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber
jsonpath "$.total" > 0

# --- 3. GET by-id (admin) — DECODE FENCE: single-row decode of FeatureFlagResponse ---
GET {{base_url}}/api/v1/feature-flags/{{created_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{created_id}}
jsonpath "$.name" isString
jsonpath "$.status" isString
jsonpath "$.tenant_id" isNumber

# --- 4. PUT update (admin) → 200; assert changed description + status ---
PUT {{base_url}}/api/v1/feature-flags/{{created_id}}
Authorization: Bearer {{admin_token}}
Content-Type: application/json
```
{ "description": "updated by hurl write-fence", "status": "enabled" }
```
HTTP 200
[Asserts]
jsonpath "$.description" == "updated by hurl write-fence"
jsonpath "$.status" == "enabled"

# --- 5. POST enable (admin) → 200; assert status enabled ---
POST {{base_url}}/api/v1/feature-flags/{{created_id}}/enable
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.status" == "enabled"

# --- 6. POST disable (admin) → 200; assert status disabled ---
POST {{base_url}}/api/v1/feature-flags/{{created_id}}/disable
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.status" == "disabled"

# --- 7. GET /check/{name} (auth) — DECODE FENCE: FeatureStatusResponse ---
GET {{base_url}}/api/v1/feature-flags/check/wf_test_flag
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.name" == "wf_test_flag"
jsonpath "$.enabled" isBoolean
jsonpath "$.tenant_id" isNumber

# --- 8. DELETE soft (admin) → 200 MessageResponse ---
DELETE {{base_url}}/api/v1/feature-flags/{{created_id}}/soft
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.message" exists

# --- 9. GET by-id → 404 (soft-deleted hidden) ---
GET {{base_url}}/api/v1/feature-flags/{{created_id}}
Authorization: Bearer {{admin_token}}
HTTP 404

# --- 10. POST restore (admin) → 200 MessageResponse ---
POST {{base_url}}/api/v1/feature-flags/{{created_id}}/restore
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.message" exists

# --- 11. GET by-id → 200 (restored) ---
GET {{base_url}}/api/v1/feature-flags/{{created_id}}
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{created_id}}

# --- 12. DELETE destroy (admin) → 200 MessageResponse ---
DELETE {{base_url}}/api/v1/feature-flags/{{created_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 200
[Asserts]
jsonpath "$.message" exists

# --- 13. GET by-id → 404 (permanently destroyed) ---
GET {{base_url}}/api/v1/feature-flags/{{created_id}}
Authorization: Bearer {{admin_token}}
HTTP 404

# --- 14. RBAC: POST create with regular user → 403 ---
POST {{base_url}}/api/v1/feature-flags
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{ "name": "wf_rbac_403", "description": "should be rejected" }
```
HTTP 403

# --- 15. No auth POST → 401 ---
POST {{base_url}}/api/v1/feature-flags
Content-Type: application/json
```
{ "name": "wf_noauth" }
```
HTTP 401