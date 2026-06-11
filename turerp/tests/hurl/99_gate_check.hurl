# Verify gate behavior: with default flags (disabled), gated routes
# return 404. Operators enable via
# /api/v1/feature-flags/{id}/enable.
#
# This file covers BOTH namespaces:
#   - tier2.* (well-known gated modules) — manufacturing is the
#     canonical smoke target because it's the only tier-2 module with
#     a working handler today.
#   - core.*  (PR-2 broken-endpoint gates, issue #152) — these routes
#     are currently 500/404-broken. The gate fires BEFORE the
#     underlying handler, so we still get 404 (not 500). This
#     proves the gate is correctly wired and PR 2's handler fix is
#     the only thing needed for operators to enable the flag.
#
# This is a temporary smoke scenario for PR 1 (gate middleware). PR 2
# will convert the core.* checks below into positive 200 assertions
# as it fixes the underlying handlers. PR 7 will replace the whole
# file with a dedicated tier-2 suite (17_..22_*.hurl).

# --- tier2.* ---

GET {{base_url}}/api/v1/manufacturing/work-orders
Authorization: Bearer {{access_token}}
HTTP 404

# --- core.* (broken-endpoint gates) ---

GET {{base_url}}/api/v1/categories
Authorization: Bearer {{access_token}}
HTTP 404

GET {{base_url}}/api/v1/units
Authorization: Bearer {{access_token}}
HTTP 404

GET {{base_url}}/api/v1/currencies
Authorization: Bearer {{access_token}}
HTTP 404

GET {{base_url}}/api/v1/settings
Authorization: Bearer {{access_token}}
HTTP 404

GET {{base_url}}/api/v1/stock/warehouses
Authorization: Bearer {{access_token}}
HTTP 404

GET {{base_url}}/api/v1/hr/leave-types
Authorization: Bearer {{access_token}}
HTTP 404
