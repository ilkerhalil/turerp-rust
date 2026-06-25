-- 042_purchase_goods_company_id.sql
-- Adds the missing company_id column to two tables whose repos SELECT
-- company_id but whose schema lacked it (HTTP 500: "column company_id does
-- not exist"):
--   GET /api/v1/purchase-requests      (purchase_requests) — list/find paths
--   GET /api/v1/goods-receipts/{id}     (goods_receipts) — find_by_id/order
-- Migration 023_companies.sql added company_id BIGINT NOT NULL DEFAULT 1 to
-- 10 tables (cari, products, stock_movements, invoices, sales_orders,
-- purchase_orders, journal_entries, payments, assets, employees) but skipped
-- these two. (quotations was the same bug, fixed by migration 041.) Mirrors
-- the 023 pattern exactly (same type, same DEFAULT 1, index, no FK).
-- Idempotent (IF NOT EXISTS).
--
-- NOT covered here (separate follow-ups):
--   - categories / units / leave_types: already fixed by an earlier change
--     that REMOVED company_id from their repo SELECTs (they don't select it),
--     so they need no column here.
--   - payrolls: also skipped by 023 and its repo selects company_id ->
--     GET /api/v1/hr/payroll/employee/{id} (and /deleted for admins) 500.
--     Tracked as the next fix (own PR + hurl fence).
--   - other latent tables 023 skipped (various *_lines, asset_categories,
--     inter_company_*, etc.): unreached or exercised by other broken paths.

ALTER TABLE purchase_requests ADD COLUMN IF NOT EXISTS company_id BIGINT NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_purchase_requests_company_id ON purchase_requests(company_id);

ALTER TABLE goods_receipts ADD COLUMN IF NOT EXISTS company_id BIGINT NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_goods_receipts_company_id ON goods_receipts(company_id);