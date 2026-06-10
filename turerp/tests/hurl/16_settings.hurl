# Tier 1 — Settings. Currently 404 (route not registered), see issue #152.

# --- 1. Settings list — currently 404 (P1) ---
GET {{base_url}}/api/v1/settings
Authorization: Bearer {{access_token}}
HTTP 404

# --- 2. Specific setting — currently 404 ---
GET {{base_url}}/api/v1/settings/default_locale
Authorization: Bearer {{access_token}}
HTTP 404
