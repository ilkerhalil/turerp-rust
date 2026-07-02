-- 055: Add tenant_id to goods_receipt_lines
--
-- Closes the LIVE cross-tenant leak on goods_receipt_lines, which 003 created
-- with only receipt_id (NOT NULL REFERENCES goods_receipts(id) ON DELETE
-- CASCADE) and no tenant_id. The GoodsReceiptLineRepository methods operate on
-- a raw receipt_id with no SQL-level tenant filter, and the service reaches
-- them *before* the tenant-scoped parent operation on several paths:
--   * destroy_receipt (LIVE admin route) calls line_repo.delete_by_receipt(id)
--     on the raw caller id BEFORE receipt_repo.destroy(id, tenant_id). A
--     tenant-A admin calling destroy_receipt(<tenant-B receipt id>)
--     hard-deletes tenant-B's lines, then the parent destroy returns NotFound
--     -- damage already done. The parent FK already has ON DELETE CASCADE, so
--     the explicit delete_by_receipt call is redundant and is dropped in the
--     code fix (mirror of the #194 invoice_lines fix).
--   * soft_delete_receipt calls soft_delete_by_receipt(id, deleted_by) BEFORE
--     receipt_repo.soft_delete(id, tenant_id, deleted_by). The code fix
--     reorders parent-first and threads tenant_id into the line soft-delete.
--   * find_by_receipt / restore_by_receipt are reached only after a
--     tenant-scoped parent lookup, so they are backstop-only today but are
--     threaded with tenant_id for defense-in-depth.
--
-- Note: goods_receipt_lines also carries a body-controlled FK order_line_id
-- (NOT NULL REFERENCES purchase_order_lines(id)) -- the goods-receipt create
-- path already validates the parent purchase order via
-- find_by_id(purchase_order_id, tenant_id), so order_line_id is implicitly
-- scoped; it is unchanged by this migration.
--
-- Backfill from goods_receipts.tenant_id: goods_receipt_lines.receipt_id is
-- NOT NULL REFERENCES goods_receipts(id) ON DELETE CASCADE and
-- goods_receipts.tenant_id is NOT NULL (003:323), so every existing row
-- receives a valid tenant_id and SET NOT NULL / the new FK are safe. The ON
-- DELETE CASCADE on receipt_id already removes a parent receipt's lines on
-- hard delete; the new tenant_id FK to tenants(id) is safe. Mirrors 048-053
-- and 054 (purchase_order_lines).

ALTER TABLE goods_receipt_lines ADD COLUMN IF NOT EXISTS tenant_id BIGINT;
UPDATE goods_receipt_lines l
  SET tenant_id = r.tenant_id
  FROM goods_receipts r
  WHERE l.receipt_id = r.id AND l.tenant_id IS NULL;
ALTER TABLE goods_receipt_lines ALTER COLUMN tenant_id SET NOT NULL;
ALTER TABLE goods_receipt_lines ADD CONSTRAINT fk_goods_receipt_lines_tenant
  FOREIGN KEY (tenant_id) REFERENCES tenants(id);
CREATE INDEX IF NOT EXISTS idx_goods_receipt_lines_tenant_id
  ON goods_receipt_lines(tenant_id);
CREATE INDEX IF NOT EXISTS idx_goods_receipt_lines_tenant_receipt
  ON goods_receipt_lines(tenant_id, receipt_id);