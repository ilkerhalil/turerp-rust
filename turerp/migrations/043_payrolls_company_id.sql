-- 043_payrolls_company_id.sql
-- Adds the missing company_id column to payrolls, whose repo SELECTs it
-- (PayrollRow declares company_id: i64) but whose schema lacked it
-- (HTTP 500: "column company_id does not exist"):
--   GET /api/v1/hr/payroll/employee/{id}  -> 500 (find_by_employee)
--   GET /api/v1/hr/payroll/deleted        -> 500 for admins (find_deleted)
--   POST /api/v1/hr/payroll/{id}/paid, restore, soft-delete -> 500
--     (RETURNING references company_id)
-- NOT fixed by this migration (separate, pre-existing bug, follow-up):
--   POST /api/v1/hr/payroll/calculate -> 500 (repo create() INSERT lists
--   company_id as a target column but provides no $N placeholder:
--   14 columns vs 13 values). That is a column/value-count mismatch,
--   not the 023-skip missing-column class this migration addresses.
-- Migration 023_companies.sql added company_id BIGINT NOT NULL DEFAULT 1 to
-- 10 tables but skipped payrolls — same 023-skip class as quotations (041),
-- purchase_requests + goods_receipts (042). Mirrors the 023 pattern exactly
-- (same type, same DEFAULT 1, index, no FK). Idempotent (IF NOT EXISTS).
--
-- This was the third broken endpoint of that class, surfaced by adversarial
-- review of PR #166 (previously unfenced and undiscovered by the hurl suite).

ALTER TABLE payrolls ADD COLUMN IF NOT EXISTS company_id BIGINT NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_payrolls_company_id ON payrolls(company_id);