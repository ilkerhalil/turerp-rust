# Tier 1.5 — Notifications/Push (register, unregister, send, broadcast, tokens).
# Scope: /v1/notifications/push/* (configured under /api → /api/v1/notifications/push/...).
# RBAC map (from src/api/v1/push_notifications.rs):
#   register/unregister/tokens → AuthUser (any authed → 200/204/400)
#   send/broadcast              → AdminUser (testuser 403, admin 200)
# Why: push_notifications module had ZERO hurl coverage. We fence the RBAC
# boundary (admin-only send/broadcast → 403 for testuser), body-validation
# 400 on register/unregister, the tokens bare-array shape, and no-auth 401.

# --- 1. GET tokens (authed) — 200, bare array (Vec<PushTokenResponse>) ---
GET {{base_url}}/api/v1/notifications/push/tokens
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection

# --- 2. POST register with empty body — 400 (RegisterPushTokenRequest
#      requires device_type + token; empty body fails JSON decode) ---
POST {{base_url}}/api/v1/notifications/push/register
Authorization: Bearer {{access_token}}
Content-Type: application/json
{}
HTTP 400

# --- 3. POST register without auth — 401 ---
POST {{base_url}}/api/v1/notifications/push/register
Content-Type: application/json
{}
HTTP 401
[Asserts]
jsonpath "$.error" exists

# --- 4. DELETE unregister with empty body — 400 (UnregisterPushTokenRequest
#      requires device_type) ---
DELETE {{base_url}}/api/v1/notifications/push/unregister
Authorization: Bearer {{access_token}}
Content-Type: application/json
{}
HTTP 400

# --- 5. DELETE unregister without auth — 401 ---
DELETE {{base_url}}/api/v1/notifications/push/unregister
Content-Type: application/json
{}
HTTP 401
[Asserts]
jsonpath "$.error" exists

# --- 6. POST send — RBAC: admin-only (AdminUser) → testuser 403 ---
POST {{base_url}}/api/v1/notifications/push/send
Authorization: Bearer {{access_token}}
Content-Type: application/json
{
    "user_id": 1,
    "title": "t",
    "body": "b"
}
HTTP 403

# --- 7. POST broadcast — RBAC: admin-only (AdminUser) → testuser 403 ---
POST {{base_url}}/api/v1/notifications/push/broadcast
Authorization: Bearer {{access_token}}
Content-Type: application/json
{
    "title": "t",
    "body": "b"
}
HTTP 403

# --- 8. POST send without auth — 401 (auth runs before RBAC) ---
POST {{base_url}}/api/v1/notifications/push/send
Content-Type: application/json
{
    "user_id": 1,
    "title": "t",
    "body": "b"
}
HTTP 401
[Asserts]
jsonpath "$.error" exists

# --- 9. POST broadcast without auth — 401 ---
POST {{base_url}}/api/v1/notifications/push/broadcast
Content-Type: application/json
{
    "title": "t",
    "body": "b"
}
HTTP 401
[Asserts]
jsonpath "$.error" exists

# --- 10. GET tokens without auth — 401 ---
GET {{base_url}}/api/v1/notifications/push/tokens
HTTP 401
[Asserts]
jsonpath "$.error" exists