# Tier 2 — Feature flags. List and per-name check work; /deleted is 404
# (gated or admin-routed-as-404 for a regular user).

# --- 1. Feature flags list — 200 (paginated envelope) ---
GET {{base_url}}/api/v1/feature-flags
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber

# --- 2. Check a known flag — 200 ---
GET {{base_url}}/api/v1/feature-flags/check/tier2.manufacturing
Authorization: Bearer {{access_token}}
HTTP 200

# --- 3. Deleted flags — 404 (fence; not exposed to regular user) ---
GET {{base_url}}/api/v1/feature-flags/deleted
Authorization: Bearer {{access_token}}
HTTP 404

# --- 4. No auth — 401 ---
GET {{base_url}}/api/v1/feature-flags
HTTP 401