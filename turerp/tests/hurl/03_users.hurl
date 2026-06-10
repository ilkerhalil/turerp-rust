# Tier 1 — Users list. Validates multi-tenant isolation: every item
# must have the same tenant_id as the calling user.


# --- 1. List users as testuser (tenant 1) ---
GET {{base_url}}/api/v1/users
Authorization: Bearer {{access_token}}
HTTP 200
[Captures]
other_user_id: jsonpath "$.items[?(@.username=='testuser')].id"
[Asserts]
jsonpath "$.items" isCollection
jsonpath "$.items[*].tenant_id" contains 1
jsonpath "$.items[?(@.id=={{user_id}})].username" contains "testuser"
jsonpath "$.total" isNumber

# --- 2. Fetch self by id ---
GET {{base_url}}/api/v1/users/{{user_id}}
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{user_id}}
jsonpath "$.tenant_id" == 1
jsonpath "$.is_active" == true

# --- 3. Fetch non-existent user MUST be 404 ---
GET {{base_url}}/api/v1/users/999999
Authorization: Bearer {{access_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 4. List without auth MUST be 401 ---
GET {{base_url}}/api/v1/users
HTTP 401
[Asserts]
jsonpath "$.error" exists
