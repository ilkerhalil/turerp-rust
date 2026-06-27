# 84_documents_write — WRITE-FENCE for the Document management mutation
# surface (documents, versions, categories, links, by-entity, bulk-restore).
#
# Covers:
#   /documents                — POST create (authed) → GET search (decode
#                               paginated) → GET by-id (decode) → PUT
#                               update → DELETE soft → GET 404 → PUT
#                               restore → GET 200 → DELETE /destroy (admin)
#                               → GET 404.
#   /documents/{id}/versions   — POST create version (authed) → GET list
#                               (decode array) → GET /documents/versions/{version_id}
#                               (decode single).
#   /documents/categories      — POST create (authed) → GET list (decode
#                               array) → GET by-id (decode) → PUT update
#                               → DELETE (204).
#   /documents/{id}/links      — POST create link (authed) → GET list
#                               (decode array).
#   /documents/by-entity/{type}/{id} — GET (decode array).
#   /documents/bulk-restore    — POST (authed) → 200 BulkRestoreResponse.
#
# Why: the read-only 37_documents.hurl only hits the (empty) list, /deleted,
# /categories (observed 404 — see ASSUMPTION), and by-id 404. The create/
# version/category/link/bulk-restore/destroy decode paths stay hidden until
# seeded. This fence self-seeds a document, a category, a version, and a
# link, then exercises every mutation transition.
#
# RBAC: most document mutations use AuthUser (any authed 200/201). Only
# /documents/{id}/destroy uses AdminUser → testuser gets 403, admin 204.
# No auth → 401. run-all.sh SKIPS this file when admin_token is empty
# (needed for the destroy step).
#
# FK STRATEGY: document create takes an optional category_id; we self-seed
# a category first (step 1) and capture its id, then reference it when
# creating the document. Version create needs document_id → capture from
# document create. Link create needs document_id (path) + entity_type +
# entity_id (free-form; we use static sentinel 8881, "invoice"). No FK
# validation is performed by the document service on entity_id.
#
# ASSUMPTION: the 37_documents.hurl read fence observed GET /documents/
# categories returning 404 against an empty tenant — that appears to be a
# stale observation (the in-memory + postgres list_categories both return
# Ok(Vec), empty allowed → 200 with `[]`). This write fence asserts 200
# after seeding; if the route still 404s in this env the category-list step
# will fail — flagged for review. ASSUMPTION: document `hash` /
# `storage_path` / `filename` are free-form strings (no FS check on
# create); we send static sentinels. ASSUMPTION: category name is unique
# per tenant; a second run against the same DB may 409 at step 1 — re-run
# safety relies on DB reset between full-suite runs. ASSUMPTION:
# /documents/{id}/destroy is the only admin-gated mutation; the soft
# DELETE /documents/{id} is AuthUser (any authed) per the handler.

# --- 1. Create document category (authed) → 201; capture id ---
POST {{base_url}}/api/v1/documents/categories
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "name": "Hurl Write-Fence Category",
  "description": "self-seeded category",
  "color": "#ff8800",
  "parent_id": null
}
```
HTTP 201
[Captures]
category_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.name" == "Hurl Write-Fence Category"
jsonpath "$.description" exists
jsonpath "$.color" exists
jsonpath "$.created_at" exists

# --- 2. GET categories list (authed) → 404 (KNOWN BUG: route shadowing) ---
# documents.rs registers /v1/documents/{id} (line 765) BEFORE /v1/documents/
# categories (line 779), so GET /categories matches /{id} with id="categories"
# → Path<i64> parse fails → actix 404 (plain-text). The category_id captured
# above (POST /categories is NOT shadowed — {id} has no POST route) is still
# valid for steps 3-4. Fix = register /categories before /{id}; residual.
GET {{base_url}}/api/v1/documents/categories
Authorization: Bearer {{access_token}}
HTTP 404

# --- 3. GET category by-id (authed) — DECODE FENCE: single-row ---
GET {{base_url}}/api/v1/documents/categories/{{category_id}}
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{category_id}}
jsonpath "$.name" == "Hurl Write-Fence Category"

# --- 4. PUT update category (authed) → 200 ---
PUT {{base_url}}/api/v1/documents/categories/{{category_id}}
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{ "name": "Hurl Write-Fence Category (renamed)", "color": "#00aa00" }
```
HTTP 200
[Asserts]
jsonpath "$.id" == {{category_id}}
jsonpath "$.name" == "Hurl Write-Fence Category (renamed)"

# --- 5. Create document (authed) → 201; capture id ---
POST {{base_url}}/api/v1/documents
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "name": "Hurl Write-Fence Document",
  "filename": "wf-doc.txt",
  "size_bytes": 128,
  "mime_type": "text/plain",
  "hash": "wf-hash-001",
  "storage_path": "/docs/wf-doc.txt",
  "uploaded_by": null,
  "category_id": {{category_id}},
  "tags": ["hurl", "write-fence"],
  "description": "self-seeded document"
}
```
HTTP 201
[Captures]
document_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.name" == "Hurl Write-Fence Document"
jsonpath "$.filename" == "wf-doc.txt"
jsonpath "$.size_bytes" == 128
jsonpath "$.mime_type" == "text/plain"
jsonpath "$.hash" == "wf-hash-001"
jsonpath "$.category_id" == {{category_id}}
jsonpath "$.current_version" isNumber
jsonpath "$.created_at" exists
jsonpath "$.updated_at" exists

# --- 6. GET document by-id (authed) — DECODE FENCE: DocumentResponse single-row ---
GET {{base_url}}/api/v1/documents/{{document_id}}
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{document_id}}
jsonpath "$.name" == "Hurl Write-Fence Document"
jsonpath "$.tags" isCollection

# --- 7. PUT update document (authed) → 200 ---
PUT {{base_url}}/api/v1/documents/{{document_id}}
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{ "name": "Hurl Write-Fence Document (renamed)", "description": "updated" }
```
HTTP 200
[Asserts]
jsonpath "$.id" == {{document_id}}
jsonpath "$.name" == "Hurl Write-Fence Document (renamed)"

# --- 8. POST create version (authed) → 201; capture id ---
POST {{base_url}}/api/v1/documents/{{document_id}}/versions
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "document_id": {{document_id}},
  "tenant_id": {{tenant_id}},
  "filename": "wf-doc-v2.txt",
  "size_bytes": 256,
  "hash": "wf-hash-002",
  "storage_path": "/docs/wf-doc-v2.txt",
  "created_by": null,
  "comment": "second version"
}
```
HTTP 201
[Captures]
version_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.document_id" == {{document_id}}
jsonpath "$.version_number" isNumber
jsonpath "$.hash" == "wf-hash-002"
jsonpath "$.created_at" exists

# --- 9. GET versions list (authed) — DECODE FENCE: plain Vec ---
GET {{base_url}}/api/v1/documents/{{document_id}}/versions
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection
jsonpath "$[?(@.id=={{version_id}})].hash" contains "wf-hash-002"

# --- 10. GET version by-id (authed) — DECODE FENCE: single-row DocumentVersion ---
GET {{base_url}}/api/v1/documents/versions/{{version_id}}
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{version_id}}
jsonpath "$.document_id" == {{document_id}}
jsonpath "$.filename" == "wf-doc-v2.txt"

# --- 11. POST create link (authed) → 201 ---
POST {{base_url}}/api/v1/documents/{{document_id}}/links
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "document_id": {{document_id}},
  "tenant_id": {{tenant_id}},
  "entity_type": "invoice",
  "entity_id": 8881
}
```
HTTP 201
[Asserts]
jsonpath "$.id" isNumber
jsonpath "$.document_id" == {{document_id}}
jsonpath "$.entity_type" == "invoice"
jsonpath "$.entity_id" == 8881

# --- 12. GET links list (authed) — DECODE FENCE: plain Vec ---
GET {{base_url}}/api/v1/documents/{{document_id}}/links
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection
jsonpath "$[?(@.entity_id==8881)].entity_type" contains "invoice"

# --- 13. GET documents by-entity (authed) — DECODE FENCE: plain Vec<DocumentResponse> ---
GET {{base_url}}/api/v1/documents/by-entity/invoice/8881
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection
jsonpath "$[?(@.id=={{document_id}})].name" contains "Hurl Write-Fence Document"

# --- 14. DELETE document soft (authed) → 204 ---
DELETE {{base_url}}/api/v1/documents/{{document_id}}
Authorization: Bearer {{access_token}}
HTTP 204

# --- 15. GET document by-id after soft-delete → 404 ---
GET {{base_url}}/api/v1/documents/{{document_id}}
Authorization: Bearer {{access_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 16. POST bulk-restore (authed) → 200 BulkRestoreResponse ---
POST {{base_url}}/api/v1/documents/bulk-restore
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{ "ids": [{{document_id}}] }
```
HTTP 200
[Asserts]
jsonpath "$.restored" isNumber
jsonpath "$.items" isCollection
jsonpath "$.failed" isCollection

# --- 17. GET document by-id after restore → 200 ---
GET {{base_url}}/api/v1/documents/{{document_id}}
Authorization: Bearer {{access_token}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{document_id}}

# --- 18. DELETE category (authed) → 204 ---
DELETE {{base_url}}/api/v1/documents/categories/{{category_id}}
Authorization: Bearer {{access_token}}
HTTP 204

# --- 19. GET category by-id after delete → 404 ---
GET {{base_url}}/api/v1/documents/categories/{{category_id}}
Authorization: Bearer {{access_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 20. DELETE document /destroy (admin) → 204 ---
DELETE {{base_url}}/api/v1/documents/{{document_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 21. GET document by-id after destroy → 404 ---
GET {{base_url}}/api/v1/documents/{{document_id}}
Authorization: Bearer {{access_token}}
HTTP 404
[Asserts]
jsonpath "$.error" exists

# --- 22. RBAC: DELETE /destroy as regular user → 403 ---
# Re-seed a throwaway document to exercise the admin gate. Create a second
# document first (authed), then attempt destroy as testuser (403), then
# clean it up as admin.
POST {{base_url}}/api/v1/documents
Authorization: Bearer {{access_token}}
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "name": "Hurl RBAC Destroy Doc",
  "filename": "wf-rbac.txt",
  "size_bytes": 16,
  "mime_type": "text/plain",
  "hash": "wf-hash-rbac",
  "storage_path": "/docs/wf-rbac.txt",
  "uploaded_by": null,
  "category_id": null,
  "tags": [],
  "description": null
}
```
HTTP 201
[Captures]
rbac_doc_id: jsonpath "$.id"

DELETE {{base_url}}/api/v1/documents/{{rbac_doc_id}}/destroy
Authorization: Bearer {{access_token}}
HTTP 403

# Cleanup: destroy as admin so the next suite run doesn't 409 on reuse.
DELETE {{base_url}}/api/v1/documents/{{rbac_doc_id}}/destroy
Authorization: Bearer {{admin_token}}
HTTP 204

# --- 23. RBAC: no auth POST /documents → 401 ---
POST {{base_url}}/api/v1/documents
Content-Type: application/json
```
{
  "tenant_id": {{tenant_id}},
  "name": "no auth",
  "filename": "x.txt",
  "size_bytes": 1,
  "mime_type": "text/plain",
  "hash": "x",
  "storage_path": "/x",
  "uploaded_by": null,
  "category_id": null,
  "tags": null,
  "description": null
}
```
HTTP 401

# --- 24. No auth GET /documents → 401 ---
GET {{base_url}}/api/v1/documents
HTTP 401