-- 051: Add tenant_id to invoice_lines
--
-- Closes the migration-needed cross-tenant leak on invoice_lines, which 003
-- created with only invoice_id (NOT NULL REFERENCES invoices(id) ON DELETE
-- CASCADE) and no tenant_id. The InvoiceLineRepository trait admits the gap
-- ("InvoiceLine does not have a tenant_id field ... Tenant isolation should be
-- enforced by looking up the parent Invoice first") but destroy_invoice called
-- delete_by_invoice(id) BEFORE invoice_repo.destroy(id, tenant_id) validated
-- the parent's tenant — a tenant-A admin could hard-delete tenant-B's invoice
-- lines by guessing an invoice id, with no compensating rollback.
--
-- Backfill from invoices.tenant_id: invoice_lines.invoice_id is NOT NULL
-- REFERENCES invoices(id) and invoices.tenant_id is NOT NULL REFERENCES
-- tenants(id), so every existing row receives a valid tenant_id and SET NOT
-- NULL / the new FK are safe. Mirrors 048 (hr attendance/leave), 049
-- (stock_levels) and 050 (manufacturing child tables).
--
-- The existing invoice_lines.invoice_id ON DELETE CASCADE already removes a
-- destroyed invoice's lines, so the explicit delete_by_invoice call in
-- destroy_invoice is redundant AND the leak source; the code fix removes that
-- call and relies on the tenant-scoped invoice_repo.destroy + CASCADE. This
-- column adds defense-in-depth (AND tenant_id = $N) so line reads/writes are
-- scoped even without the parent lookup.

ALTER TABLE invoice_lines ADD COLUMN IF NOT EXISTS tenant_id BIGINT;
UPDATE invoice_lines l
  SET tenant_id = i.tenant_id
  FROM invoices i
  WHERE l.invoice_id = i.id AND l.tenant_id IS NULL;
ALTER TABLE invoice_lines ALTER COLUMN tenant_id SET NOT NULL;
ALTER TABLE invoice_lines ADD CONSTRAINT fk_invoice_lines_tenant
  FOREIGN KEY (tenant_id) REFERENCES tenants(id);
CREATE INDEX IF NOT EXISTS idx_invoice_lines_tenant_id
  ON invoice_lines(tenant_id);
CREATE INDEX IF NOT EXISTS idx_invoice_lines_tenant_invoice
  ON invoice_lines(tenant_id, invoice_id);