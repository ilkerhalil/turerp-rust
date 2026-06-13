# Tier 1 — Categories. Documents known P1: GET /categories is gated
# (core.categories, default-disabled). PR 2 fixed the underlying handler
# (postgres_repository had a phantom `company_id` column drift that caused
# FromRow decode to fail and the handler to return 404). The flag is
# off by default; operator flips it on after deploy.


# --- 1. Categories list (gated; returns 200 + empty paginated body) ---
GET {{base_url}}/api/v1/categories
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber
jsonpath "$.page" isNumber
jsonpath "$.per_page" isNumber
