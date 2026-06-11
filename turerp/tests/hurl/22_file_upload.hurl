# Tier 2 — File upload. S3-backed file storage module is wired in
# (`src/domain/file/`), but the upload endpoint is gated behind
# tier2.file_upload (default OFF) for the GA cut. The 404 here is
# the gate firing, not a missing route. Operator enables the flag
# and the same routes return 200/201.

# --- 1. File upload (POST multipart) — gated → 404 ---
POST {{base_url}}/api/v1/files/upload
Authorization: Bearer {{access_token}}
[Multipart]
field1: value1
HTTP 404

# --- 2. File list — gated → 404 ---
GET {{base_url}}/api/v1/files
Authorization: Bearer {{access_token}}
HTTP 404

# --- 3. File detail — gated → 404 ---
GET {{base_url}}/api/v1/files/1
Authorization: Bearer {{access_token}}
HTTP 404

# --- 4. Attachment download — gated → 404 ---
GET {{base_url}}/api/v1/files/1/download
Authorization: Bearer {{access_token}}
HTTP 404
