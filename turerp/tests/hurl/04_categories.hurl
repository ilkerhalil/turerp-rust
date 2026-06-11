# Tier 1 — Categories. Documents known P1: GET /categories is gated
# (core.categories, default-disabled). With the gate in place, the
# request 404s BEFORE the underlying handler runs. PR 2 will fix
# the underlying handler AND the operator will flip the flag on
# (these are tracked together).


# --- 1. Categories list — currently 404 (gated; PR 2 fixes handler) ---
GET {{base_url}}/api/v1/categories
Authorization: Bearer {{access_token}}
HTTP 404
