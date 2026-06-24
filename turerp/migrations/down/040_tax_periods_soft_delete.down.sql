-- 040_tax_periods_soft_delete.down.sql
-- Revert the soft-delete columns added by 040. NOTE: re-introduces the
-- `column "deleted_at" does not exist` 500 on GET /api/v1/tax/periods
-- (TaxPeriodRow still SELECTs them); only run when rolling back 040.
DROP INDEX IF EXISTS idx_tax_periods_deleted_at;
ALTER TABLE tax_periods DROP COLUMN IF EXISTS deleted_by;
ALTER TABLE tax_periods DROP COLUMN IF EXISTS deleted_at;