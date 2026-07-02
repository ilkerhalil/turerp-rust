-- 058: Add tenant_id to quotation_lines
--
-- Closes the LIVE cross-tenant leak on quotation_lines, which 003 created
-- with only quotation_id (NOT NULL REFERENCES quotations(id) ON DELETE
-- CASCADE) and no tenant_id. Same leak class as 057 (sales_order_lines): the
-- QuotationLineRepository methods operate on a raw quotation_id / line_id
-- with no SQL-level tenant filter, and the service reaches them *before* the
-- tenant-scoped parent operation on several paths:
--   * destroy_quotation (LIVE admin route) calls line_repo.delete_by_quotation
--     (id) on the raw caller id BEFORE quotation_repo.destroy(id, tenant_id).
--     The parent FK already has ON DELETE CASCADE, so the explicit
--     delete_by_quotation call is redundant and is dropped in the code fix
--     (mirror of the #194 invoice_lines fix and 054/057).
--   * soft_delete_quotation soft-deletes lines BEFORE
--     quotation_repo.soft_delete(id, tenant_id, deleted_by). The code fix
--     reorders parent-first and threads tenant_id into the line soft-delete.
--   * The per-line methods (soft_delete / restore / find_deleted / destroy)
--     take a raw line_id with no tenant_id and -- in the Postgres impl --
--     ignore the passed quotation_id entirely (WHERE id = $1 only). The code
--     fix threads tenant_id and a real quotation_id filter (AND quotation_id
--     = $N AND tenant_id = $N) into every per-line method, and adds a
--     parent-ownership precheck in the service (find_by_id(quotation_id,
--     tenant_id) -> NotFound before touching the line). This closes the
--     per-line IDOR: today a tenant-A admin calling
--     DELETE .../quotations/{own_quotation}/lines/{tenant-B line}/destroy
--     hard-deletes tenant-B's line by guessing line_id.
--   * find_by_quotation / restore are reached only after a tenant-scoped
--     parent lookup, so they are backstop-only today but are threaded with
--     tenant_id for defense-in-depth.
--
-- Backfill from quotations.tenant_id: quotation_lines.quotation_id is
-- NOT NULL REFERENCES quotations(id) ON DELETE CASCADE and
-- quotations.tenant_id is NOT NULL REFERENCES tenants(id), so every existing
-- row receives a valid tenant_id and SET NOT NULL / the new FK are safe. The
-- ON DELETE CASCADE on quotation_id already removes a parent quotation's
-- lines on hard delete. The new tenant_id FK uses the default NO ACTION
-- (consistent with the other tenant FKs); the parent's own NO ACTION FK to
-- tenants(id) already blocks tenant hard-delete, so this child FK adds no new
-- blocking power. Mirrors 048-054 and 057.

ALTER TABLE quotation_lines ADD COLUMN IF NOT EXISTS tenant_id BIGINT;
UPDATE quotation_lines l
  SET tenant_id = q.tenant_id
  FROM quotations q
  WHERE l.quotation_id = q.id AND l.tenant_id IS NULL;
ALTER TABLE quotation_lines ALTER COLUMN tenant_id SET NOT NULL;
ALTER TABLE quotation_lines ADD CONSTRAINT fk_quotation_lines_tenant
  FOREIGN KEY (tenant_id) REFERENCES tenants(id);
CREATE INDEX IF NOT EXISTS idx_quotation_lines_tenant_id
  ON quotation_lines(tenant_id);
CREATE INDEX IF NOT EXISTS idx_quotation_lines_tenant_quotation
  ON quotation_lines(tenant_id, quotation_id);