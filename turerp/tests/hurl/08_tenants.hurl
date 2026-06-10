# Tier 1 — Tenants. Multi-tenant boundary: the testuser's tenant must
# appear in the list, and a foreign tenant id must not leak.


# --- 1. List tenants (admin scope) ---
GET {{base_url}}/api/v1/tenants
Authorization: Bearer {{access_token}}
HTTP 200
[Captures]
first_tenant_id: jsonpath "$.items[0].id"
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.total" isNumber
jsonpath "$.items[0].id" == 1

# --- 2. Fetch tenant 1 directly ---
GET {{base_url}}/api/v1/tenants/1
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == 1
jsonpath "$.subdomain" exists
jsonpath "$.is_active" == true

# --- 3. Fetch a non-existent tenant — 404 ---
GET {{base_url}}/api/v1/tenants/999999
Authorization: Bearer {{access_token}}
HTTP 404
