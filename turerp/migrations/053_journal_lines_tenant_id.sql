-- 053: Add tenant_id to journal_lines
--
-- Defense-in-depth backstop for the migration-needed cross-tenant gap on
-- journal_lines, which 003 created with only entry_id (NOT NULL REFERENCES
-- journal_entries(id) ON DELETE CASCADE) and no tenant_id. All three
-- JournalLineRepository methods (create_many / find_by_entry /
-- delete_by_entry) skip tenant scoping at the SQL level, but every
-- HTTP-reachable path validates the parent journal_entries by tenant_id
-- first, so this is LATENT, not a live leak today:
--   * create_many is reached only via create_journal_entry, where the parent
--     entry is created in the same call with create.tenant_id set from
--     AdminUser (handler overwrite) — entry_id is a server-generated, not
--     body-controlled, FK.
--   * find_by_entry is reached only via generate_trial_balance, where the
--     entry set is pre-filtered by find_by_date_range(tenant_id, ...).
--   * delete_by_entry has no caller/route (dead code).
--
-- This column adds the SQL-level backstop so any future code that takes a
-- raw entry_id/line_id and calls the line repo directly cannot become a live
-- cross-tenant leak. Backfill from journal_entries.tenant_id:
-- journal_lines.entry_id is NOT NULL REFERENCES journal_entries(id) and
-- journal_entries.tenant_id is NOT NULL REFERENCES tenants(id), so every
-- existing row receives a valid tenant_id and SET NOT NULL / the new FK are
-- safe. Mirrors 048 (hr), 049 (stock_levels), 050 (mfg children), 051
-- (invoice_lines) and 052 (maintenance_records).

ALTER TABLE journal_lines ADD COLUMN IF NOT EXISTS tenant_id BIGINT;
UPDATE journal_lines l
  SET tenant_id = e.tenant_id
  FROM journal_entries e
  WHERE l.entry_id = e.id AND l.tenant_id IS NULL;
ALTER TABLE journal_lines ALTER COLUMN tenant_id SET NOT NULL;
ALTER TABLE journal_lines ADD CONSTRAINT fk_journal_lines_tenant
  FOREIGN KEY (tenant_id) REFERENCES tenants(id);
CREATE INDEX IF NOT EXISTS idx_journal_lines_tenant_id
  ON journal_lines(tenant_id);
CREATE INDEX IF NOT EXISTS idx_journal_lines_tenant_entry
  ON journal_lines(tenant_id, entry_id);