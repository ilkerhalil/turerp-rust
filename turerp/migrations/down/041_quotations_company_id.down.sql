-- 041_quotations_company_id.down.sql
-- Reverse of 041: drop the index, then the column. Idempotent (IF EXISTS).

DROP INDEX IF EXISTS idx_quotations_company_id;
ALTER TABLE quotations DROP COLUMN IF EXISTS company_id;