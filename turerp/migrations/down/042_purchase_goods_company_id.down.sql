-- 042_finish_023_company_id.down.sql
-- Reverse of 042: drop each index, then its column. Idempotent (IF EXISTS).
-- Within a table the index must be dropped before its column.

DROP INDEX IF EXISTS idx_purchase_requests_company_id;
ALTER TABLE purchase_requests DROP COLUMN IF EXISTS company_id;

DROP INDEX IF EXISTS idx_goods_receipts_company_id;
ALTER TABLE goods_receipts DROP COLUMN IF EXISTS company_id;