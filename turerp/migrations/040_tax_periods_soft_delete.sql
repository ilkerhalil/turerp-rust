-- 040_tax_periods_soft_delete.sql
--
-- tax_periods (migration 012) was never given the soft-delete columns, but
-- TaxPeriod uses impl_soft_deletable! and the PostgresTaxPeriodRepository
-- SELECTs deleted_at / deleted_by (via the TAX_PERIOD_COLUMNS! macro) and
-- filters `WHERE ... AND deleted_at IS NULL` on every read. So every query
-- against tax_periods fails with `column "deleted_at" does not exist` ->
-- HTTP 500 on GET /api/v1/tax/periods (and find_by_id / create RETURNING).
--
-- migration 020 (soft_delete_complete) added deleted_at/deleted_by to many
-- tables but missed tax_periods. This adds them, matching the declaration
-- used by sibling soft-deletable tables (e.g. cost_centers 023):
--   deleted_at TIMESTAMPTZ (nullable)
--   deleted_by BIGINT REFERENCES users(id) ON DELETE SET NULL
-- plus the partial index convention from 020.
ALTER TABLE tax_periods ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ;
ALTER TABLE tax_periods ADD COLUMN IF NOT EXISTS deleted_by BIGINT REFERENCES users(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS idx_tax_periods_deleted_at ON tax_periods(deleted_at) WHERE deleted_at IS NULL;