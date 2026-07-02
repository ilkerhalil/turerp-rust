-- 054: Add tenant_id to purchase_order_lines
--
-- Closes the LIVE cross-tenant leak on purchase_order_lines, which 003 created
-- with only order_id (NOT NULL REFERENCES purchase_orders(id) ON DELETE
-- CASCADE) and no tenant_id. The PurchaseOrderLineRepository methods operate on
-- a raw order_id with no SQL-level tenant filter, and the service reaches them
-- *before* the tenant-scoped parent operation on several paths:
--   * destroy_order (LIVE admin route) calls line_repo.delete_by_order(id)
--     on the raw caller id BEFORE order_repo.destroy(id, tenant_id). A
--     tenant-A admin calling destroy_order(<tenant-B order id>) hard-deletes
--     tenant-B's lines, then the parent destroy returns NotFound -- but the
--     damage is done (no CASCADE-then-check ordering; the explicit delete runs
--     first). The parent FK already has ON DELETE CASCADE, so the explicit
--     delete_by_order call is redundant and is dropped in the code fix (mirror
--     of the #194 invoice_lines fix).
--   * soft_delete_order calls soft_delete_by_order(id, deleted_by) BEFORE
--     order_repo.soft_delete(id, tenant_id, deleted_by). The code fix reorders
--     parent-first and threads tenant_id into the line soft-delete.
--   * find_by_order / restore_by_order are reached only after a tenant-scoped
--     parent lookup, so they are backstop-only today but are threaded with
--     tenant_id for defense-in-depth.
--
-- Backfill from purchase_orders.tenant_id: purchase_order_lines.order_id is
-- NOT NULL REFERENCES purchase_orders(id) ON DELETE CASCADE and
-- purchase_orders.tenant_id is NOT NULL REFERENCES tenants(id), so every
-- existing row receives a valid tenant_id and SET NOT NULL / the new FK are
-- safe. The ON DELETE CASCADE on order_id already removes a parent order's
-- lines on hard delete. The new tenant_id FK uses the default NO ACTION
-- (consistent with the other tenant FKs in this schema); a tenant cannot have
-- purchase_order_lines without first having a parent purchase_order, and the
-- parent's own NO ACTION FK to tenants(id) already blocks tenant hard-delete,
-- so this child FK adds no new blocking power. Mirrors 048 (hr), 049
-- (stock_levels), 050 (mfg children), 051 (invoice_lines), 052
-- (maintenance_records) and 053 (journal_lines).

ALTER TABLE purchase_order_lines ADD COLUMN IF NOT EXISTS tenant_id BIGINT;
UPDATE purchase_order_lines l
  SET tenant_id = o.tenant_id
  FROM purchase_orders o
  WHERE l.order_id = o.id AND l.tenant_id IS NULL;
ALTER TABLE purchase_order_lines ALTER COLUMN tenant_id SET NOT NULL;
ALTER TABLE purchase_order_lines ADD CONSTRAINT fk_purchase_order_lines_tenant
  FOREIGN KEY (tenant_id) REFERENCES tenants(id);
CREATE INDEX IF NOT EXISTS idx_purchase_order_lines_tenant_id
  ON purchase_order_lines(tenant_id);
CREATE INDEX IF NOT EXISTS idx_purchase_order_lines_tenant_order
  ON purchase_order_lines(tenant_id, order_id);