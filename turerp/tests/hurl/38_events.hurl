# Tier 2 — Event bus / outbox / DLQ / CDC. All read endpoints are
# admin-only: a regular user gets 403 on every sub-route, and the
# collection root has no GET (404). RBAC fence for the event pipeline.

# --- 1. Root GET — 404 (no GET on /events; sub-routes only) ---
GET {{base_url}}/api/v1/events
Authorization: Bearer {{access_token}}
HTTP 404

# --- 2. Outbox pending — RBAC 403 ---
GET {{base_url}}/api/v1/events/outbox/pending
Authorization: Bearer {{access_token}}
HTTP 403

# --- 3. Dead letters — RBAC 403 ---
GET {{base_url}}/api/v1/events/dead-letters
Authorization: Bearer {{access_token}}
HTTP 403

# --- 4. DLQ — RBAC 403 ---
GET {{base_url}}/api/v1/events/dlq
Authorization: Bearer {{access_token}}
HTTP 403

# --- 5. CDC status — RBAC 403 ---
GET {{base_url}}/api/v1/events/cdc/status
Authorization: Bearer {{access_token}}
HTTP 403

# --- 6. No auth — 401 ---
GET {{base_url}}/api/v1/events/dlq
HTTP 401