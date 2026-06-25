-- 043_payrolls_company_id.down.sql
-- Reverse of 043: drop the index, then the column. Idempotent (IF EXISTS).

DROP INDEX IF EXISTS idx_payrolls_company_id;
ALTER TABLE payrolls DROP COLUMN IF EXISTS company_id;