-- 056: Add tenant_id to purchase_request_lines
--
-- Closes the LIVE cross-tenant leak on purchase_request_lines, which 003
-- created with only request_id (NOT NULL REFERENCES purchase_requests(id) ON
-- DELETE CASCADE) and no tenant_id. The PurchaseRequestLineRepository methods
-- operate on a raw request_id with no SQL-level tenant filter, and the service
-- reaches them *before* the tenant-scoped parent operation on several paths:
--   * destroy_request (LIVE admin route) calls
--     line_repo.delete_by_request(id) on the raw caller id BEFORE
--     request_repo.destroy(id, tenant_id). A tenant-A admin calling
--     destroy_request(<tenant-B request id>) hard-deletes tenant-B's lines,
--     then the parent destroy returns NotFound -- damage already done. The
--     parent FK already has ON DELETE CASCADE, so the explicit
--     delete_by_request call is redundant and is dropped in the code fix
--     (mirror of the #194 invoice_lines fix).
--   * soft_delete_request calls soft_delete_by_request(id, deleted_by) BEFORE
--     request_repo.soft_delete(id, tenant_id, deleted_by). The code fix
--     reorders parent-first and threads tenant_id into the line soft-delete.
--   * find_by_request / restore_by_request are reached only after a
--     tenant-scoped parent lookup, so they are backstop-only today but are
--     threaded with tenant_id for defense-in-depth.
--
-- Backfill from purchase_requests.tenant_id: purchase_request_lines.request_id
-- is NOT NULL REFERENCES purchase_requests(id) ON DELETE CASCADE and
-- purchase_requests.tenant_id is NOT NULL (003:287), so every existing row
-- receives a valid tenant_id and SET NOT NULL / the new FK are safe. The ON
-- DELETE CASCADE on request_id already removes a parent request's lines on
-- hard delete; the new tenant_id FK to tenants(id) is safe. Mirrors 048-054
-- and 055 (goods_receipt_lines).

ALTER TABLE purchase_request_lines ADD COLUMN IF NOT EXISTS tenant_id BIGINT;
UPDATE purchase_request_lines l
  SET tenant_id = r.tenant_id
  FROM purchase_requests r
  WHERE l.request_id = r.id AND l.tenant_id IS NULL;
ALTER TABLE purchase_request_lines ALTER COLUMN tenant_id SET NOT NULL;
ALTER TABLE purchase_request_lines ADD CONSTRAINT fk_purchase_request_lines_tenant
  FOREIGN KEY (tenant_id) REFERENCES tenants(id);
CREATE INDEX IF NOT EXISTS idx_purchase_request_lines_tenant_id
  ON purchase_request_lines(tenant_id);
CREATE INDEX IF NOT EXISTS idx_purchase_request_lines_tenant_request
  ON purchase_request_lines(tenant_id, request_id);