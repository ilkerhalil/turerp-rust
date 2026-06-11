# Verify gate behavior: with default flags (disabled), a tier-2 route
# returns 404. Operators enable via /api/v1/feature-flags/{id}/enable.
#
# This is a temporary smoke scenario for PR 1 (gate middleware). PR 7
# will replace it with a dedicated tier-2 suite (17_..22_*.hurl).

GET {{base_url}}/api/v1/manufacturing/work-orders
Authorization: Bearer {{access_token}}
HTTP 404
