# Tier 2 — Customer & vendor self-service portals. Portal resources use
# a SEPARATE auth scheme from the main JWT: a request with the main
# access token gets 401 (not 403) on portal endpoints. The public
# /register and /login routes are POST-only (GET 405).

# --- 1. Customer portal invoices — 401 (portal auth, not main JWT) ---
GET {{base_url}}/api/v1/customer-portal/invoices
Authorization: Bearer {{access_token}}
HTTP 401

# --- 2. Vendor portal invoices — 401 (portal auth, not main JWT) ---
GET {{base_url}}/api/v1/vendor-portal/invoices
Authorization: Bearer {{access_token}}
HTTP 401

# --- 3. Customer portal register — POST-only, GET 405 ---
GET {{base_url}}/api/v1/customer-portal/register
HTTP 405

# --- 4. Vendor portal login — POST-only, GET 405 ---
GET {{base_url}}/api/v1/vendor-portal/login
HTTP 405

# --- 5. Customer portal with no auth — 401 ---
GET {{base_url}}/api/v1/customer-portal/orders
HTTP 401