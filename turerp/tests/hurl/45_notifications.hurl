# Tier 2 — Notifications. Read endpoints are admin-only (403 for a
# regular user); the bare root has no GET (404).

# --- 1. Notifications root — 404 (no GET on bare /notifications) ---
GET {{base_url}}/api/v1/notifications
Authorization: Bearer {{access_token}}
HTTP 404

# --- 2. In-app notifications — RBAC 403 ---
GET {{base_url}}/api/v1/notifications/in-app
Authorization: Bearer {{access_token}}
HTTP 403

# --- 3. Unread count — RBAC 403 ---
GET {{base_url}}/api/v1/notifications/unread-count
Authorization: Bearer {{access_token}}
HTTP 403

# --- 4. History — RBAC 403 ---
GET {{base_url}}/api/v1/notifications/history
Authorization: Bearer {{access_token}}
HTTP 403

# --- 5. No auth — 401 ---
GET {{base_url}}/api/v1/notifications/in-app
HTTP 401