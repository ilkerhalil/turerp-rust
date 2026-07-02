-- 057: Add tenant_id to sales_order_lines
--
-- Closes the LIVE cross-tenant leak on sales_order_lines, which 003 created
-- with only order_id (NOT NULL REFERENCES sales_orders(id) ON DELETE CASCADE)
-- and no tenant_id. The SalesOrderLineRepository methods operate on a raw
-- order_id (and, for the per-line methods, a raw line_id) with no SQL-level
-- tenant filter, and the service reaches them *before* the tenant-scoped
-- parent operation on several paths:
--   * destroy_order (LIVE admin route) calls line_repo.delete_by_order(id)
--     on the raw caller id BEFORE order_repo.destroy(id, tenant_id). A
--     tenant-A admin calling destroy_order(<tenant-B order id>) hard-deletes
--     tenant-B's lines, then the parent destroy returns NotFound -- but the
--     damage is done. The parent FK already has ON DELETE CASCADE, so the
--     explicit delete_by_order call is redundant and is dropped in the code
--     fix (mirror of the #194 invoice_lines fix and 054).
--   * soft_delete_order calls line soft-delete BEFORE
--     order_repo.soft_delete(id, tenant_id, deleted_by). The code fix
--     reorders parent-first and threads tenant_id into the line soft-delete.
--   * The per-line methods (soft_delete / restore / find_deleted / destroy)
--     take a raw line_id with no tenant_id and -- in the Postgres impl --
--     ignore the passed order_id entirely (WHERE id = $1 only). The code fix
--     threads tenant_id and a real order_id filter (AND order_id = $N AND
--     tenant_id = $N) into every per-line method, and adds a parent-ownership
--     precheck in the service (find_by_id(order_id, tenant_id) -> NotFound
--     before touching the line). This closes the per-line IDOR: today a
--     tenant-A admin calling DELETE .../orders/{own_order}/lines/{tenant-B
--     line}/destroy hard-deletes tenant-B's line by guessing line_id.
--   * find_by_order / restore are reached only after a tenant-scoped parent
--     lookup, so they are backstop-only today but are threaded with tenant_id
--     for defense-in-depth.
--
-- Backfill from sales_orders.tenant_id: sales_order_lines.order_id is
-- NOT NULL REFERENCES sales_orders(id) ON DELETE CASCADE and
-- sales_orders.tenant_id is NOT NULL REFERENCES tenants(id), so every
-- existing row receives a valid tenant_id and SET NOT NULL / the new FK are
-- safe. The ON DELETE CASCADE on order_id already removes a parent order's
-- lines on hard delete. The new tenant_id FK uses the default NO ACTION
-- (consistent with the other tenant FKs in this schema); a tenant cannot have
-- sales_order_lines without first having a parent sales_order, and the
-- parent's own NO ACTION FK to tenants(id) already blocks tenant hard-delete,
-- so this child FK adds no new blocking power. Mirrors 048 (hr), 049
-- (stock_levels), 050 (mfg children), 051 (invoice_lines), 052
-- (maintenance_records), 053 (journal_lines) and 054-056 (purchase lines).

ALTER TABLE sales_order_lines ADD COLUMN IF NOT EXISTS tenant_id BIGINT;
UPDATE sales_order_lines l
  SET tenant_id = o.tenant_id
  FROM sales_orders o
  WHERE l.order_id = o.id AND l.tenant_id IS NULL;
ALTER TABLE sales_order_lines ALTER COLUMN tenant_id SET NOT NULL;
ALTER TABLE sales_order_lines ADD CONSTRAINT fk_sales_order_lines_tenant
  FOREIGN KEY (tenant_id) REFERENCES tenants(id);
CREATE INDEX IF NOT EXISTS idx_sales_order_lines_tenant_id
  ON sales_order_lines(tenant_id);
CREATE INDEX IF NOT EXISTS idx_sales_order_lines_tenant_order
  ON sales_order_lines(tenant_id, order_id);