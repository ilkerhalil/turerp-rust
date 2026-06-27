# Tier 1.5 — Auth/MFA (setup, verify-setup, disable, status, verify,
# regenerate-backup-codes). Read-leaning fence: GET status (200 — always
# returns a status object even when not enrolled), no-auth 401 on authed
# routes, empty/invalid body 400 on body-requiring routes.
# Why: mfa module had ZERO hurl coverage. We do NOT attempt the full
# enrollment flow (setup → verify-setup) because it is stateful and would
# mutate testuser's MFA state across runs. Instead we fence each route's
# auth + body-validation boundary, which is where the silent-failure bugs hide.
# Scope: /v1/auth/mfa/* (configured under /api → /api/v1/auth/mfa/...).

# --- 1. GET status (authed) — 200. Handler returns MfaStatusResponse
#      regardless of enrollment state (enabled: true/false). ---
GET {{base_url}}/api/v1/auth/mfa/status
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.mfa_enabled" isBoolean

# --- 2. GET status without auth — 401 ---
GET {{base_url}}/api/v1/auth/mfa/status
HTTP 401
[Asserts]
jsonpath "$.error" exists

# --- 3. POST setup without auth — 401 (AuthUser extractor) ---
#      NOTE: mfa_setup takes NO request body, so we only fence the auth
#      boundary here, not a body-validation path. Calling setup with auth
#      would initiate enrollment (stateful) — intentionally skipped.
POST {{base_url}}/api/v1/auth/mfa/setup
HTTP 401
[Asserts]
jsonpath "$.error" exists

# --- 4. POST verify-setup with empty body — 400 (requires VerifyTotpRequest
#      with `code` field; empty body fails JSON decode) ---
POST {{base_url}}/api/v1/auth/mfa/verify-setup
Authorization: Bearer {{access_token}}
Content-Type: application/json
{}
HTTP 400

# --- 5. POST verify-setup without auth — 401 ---
POST {{base_url}}/api/v1/auth/mfa/verify-setup
Content-Type: application/json
{}
HTTP 401

# --- 6. POST disable with empty body — 400 (requires DisableMfaRequest
#      with `password` field) ---
POST {{base_url}}/api/v1/auth/mfa/disable
Authorization: Bearer {{access_token}}
Content-Type: application/json
{}
HTTP 400

# --- 7. POST disable without auth — 401 ---
POST {{base_url}}/api/v1/auth/mfa/disable
Content-Type: application/json
{}
HTTP 401

# --- 8. POST verify (PUBLIC — uses mfa_token in body, not Bearer) with
#      empty body — 400 (requires VerifyMfaRequest: mfa_token + code) ---
POST {{base_url}}/api/v1/auth/mfa/verify
Content-Type: application/json
{}
HTTP 400

# --- 9. POST verify with invalid mfa_token — 401 (token decode fails) ---
POST {{base_url}}/api/v1/auth/mfa/verify
Content-Type: application/json
{
    "mfa_token": "not-a-real-jwt",
    "code": "000000"
}
HTTP 401

# --- 10. POST regenerate-backup-codes without auth — 401 ---
POST {{base_url}}/api/v1/auth/mfa/regenerate-backup-codes
HTTP 401
[Asserts]
jsonpath "$.error" exists

# --- 11. POST regenerate-backup-codes (authed) — 400 when MFA not enabled.
#       testuser is not enrolled, so the service returns BadRequest
#       ("MFA not enabled"). This is stateless (no MFA state to mutate). ---
POST {{base_url}}/api/v1/auth/mfa/regenerate-backup-codes
Authorization: Bearer {{access_token}}
HTTP 400