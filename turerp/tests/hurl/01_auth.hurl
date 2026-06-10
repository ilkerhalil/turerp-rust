# Tier 1 — Auth surface: login, /me, refresh, logout.
# This is the baseline — if these fail, every other scenario will fail.
# Also documents the PR #145 follow-up (issue #152): a wrong password
# must return 401, not 200.

# --- 1. Re-fetch /me with the captured token (proves JWT works) ---
GET {{base_url}}/api/v1/auth/me
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{user_id}}
jsonpath "$.tenant_id" == 1

# --- 2. Refresh token roundtrip ---
POST {{base_url}}/api/v1/auth/refresh
Content-Type: application/json
{
    "refresh_token": "{{refresh_token}}"
}
HTTP 200
[Captures]
new_access: jsonpath "$.access_token"
[Asserts]
jsonpath "$.token_type" == "Bearer"
variable "new_access" exists

# --- 3. Logout (idempotent — body null is fine) ---
POST {{base_url}}/api/v1/auth/logout
Authorization: Bearer {{access_token}}
Content-Type: application/json
{
    "refresh_token": "{{refresh_token}}"
}
HTTP 200

# --- 4. Issue #152 regression: wrong password MUST be 401 (not 200, not 200+token) ---
POST {{base_url}}/api/v1/auth/login?tenant_id=1
Content-Type: application/json
{
    "username": "testuser",
    "password": "definitely-wrong-password-xyz"
}
HTTP 401
[Asserts]
jsonpath "$.error" contains "credentials"
jsonpath "$.tokens" not exists

# --- 5. Login with empty password MUST be 400 (validation) ---
POST {{base_url}}/api/v1/auth/login?tenant_id=1
Content-Type: application/json
{
    "username": "testuser",
    "password": ""
}
HTTP 400
[Asserts]
jsonpath "$.error" exists

# --- 6. Login with missing tenant_id query MUST be 400 ---
POST {{base_url}}/api/v1/auth/login
Content-Type: application/json
{
    "username": "testuser",
    "password": "{{test_password}}"
}
HTTP 400
[Asserts]
jsonpath "$.error" contains "tenant_id"

# --- 7. /me with no Authorization header MUST be 401 ---
GET {{base_url}}/api/v1/auth/me
HTTP 401
[Asserts]
jsonpath "$.error" exists

# --- 8. /me with tampered JWT MUST be 401 ---
GET {{base_url}}/api/v1/auth/me
Authorization: Bearer {{access_token}}TAMPERED
HTTP 401
[Asserts]
jsonpath "$.error" exists
