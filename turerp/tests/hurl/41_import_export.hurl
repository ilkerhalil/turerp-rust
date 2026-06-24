# Tier 2 — Bulk import / export. Both modules are scoped with no GET on
# the bare root: /import is POST-driven (per-entity), /export serves
# /export/{entity} (GET). Roots 404 for a regular caller.

# --- 1. Import root — 404 (no GET; POST per-entity) ---
GET {{base_url}}/api/v1/import
Authorization: Bearer {{access_token}}
HTTP 404

# --- 2. Export root — 404 (needs /{entity}) ---
GET {{base_url}}/api/v1/export
Authorization: Bearer {{access_token}}
HTTP 404

# --- 3. Export by entity — RBAC: regular user 403 (export is admin-only) ---
GET {{base_url}}/api/v1/export/this-entity-does-not-exist
Authorization: Bearer {{access_token}}
HTTP 403

# --- 4. No auth on export — 401 ---
GET {{base_url}}/api/v1/export
HTTP 401