# Tier 2 — Background job scheduler. Read endpoints are admin-only
# (403 for a regular user); the bare root has no GET (404).

# --- 1. Jobs root — 404 (no GET on bare /jobs) ---
GET {{base_url}}/api/v1/jobs
Authorization: Bearer {{access_token}}
HTTP 404

# --- 2. Next job — RBAC 403 ---
GET {{base_url}}/api/v1/jobs/next
Authorization: Bearer {{access_token}}
HTTP 403

# --- 3. Jobs by status — RBAC 403 ---
GET {{base_url}}/api/v1/jobs/status/queued
Authorization: Bearer {{access_token}}
HTTP 403

# --- 4. No auth — 401 ---
GET {{base_url}}/api/v1/jobs/next
HTTP 401