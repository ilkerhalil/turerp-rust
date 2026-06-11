# Tier 2 — GraphQL. The unified async-graphql schema exposes a single
# POST endpoint that takes a query string and returns JSON. Gated
# behind tier2.graphql (default OFF) so the introspection surface is
# not exposed until the operator opts in.

# --- 1. GraphQL endpoint (gated) — default OFF → 404 ---
POST {{base_url}}/api/v1/graphql
Authorization: Bearer {{access_token}}
Content-Type: application/json
{
  "query": "{ users(page: 1, perPage: 10) { items { id username email } pageInfo { total } } }"
}
HTTP 404

# --- 2. GraphQL with employees query (gated) — 404 ---
POST {{base_url}}/api/v1/graphql
Authorization: Bearer {{access_token}}
Content-Type: application/json
{
  "query": "{ employees(page: 1, perPage: 10) { items { id employeeNumber firstName lastName } pageInfo { total } } }"
}
HTTP 404

# --- 3. GraphQL with invoices query (gated) — 404 ---
POST {{base_url}}/api/v1/graphql
Authorization: Bearer {{access_token}}
Content-Type: application/json
{
  "query": "{ invoices(page: 1, perPage: 10) { items { id invoiceNumber status totalAmount } pageInfo { total } } }"
}
HTTP 404

# --- 4. GraphQL with empty body (gated) — 404 ---
POST {{base_url}}/api/v1/graphql
Authorization: Bearer {{access_token}}
Content-Type: application/json
{}
HTTP 404
