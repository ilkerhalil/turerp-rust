-- 041_quotations_company_id.sql
-- Fixes GET /api/v1/sales/quotations 500: every quotation SELECT references
-- company_id, but migration 023_companies.sql added company_id to 10 tables
-- (cari, products, stock_movements, invoices, sales_orders, purchase_orders,
-- journal_entries, payments, assets, employees) and MISSED quotations.
-- QuotationRow / QuotationRowWithTotal declare company_id and the postgres
-- repo SELECTs it -> "column company_id does not exist" -> 500 on list,
-- find_by_id, find_by_tenant, find_by_status, find_by_cari.
-- Mirrors the 023 pattern exactly (NOT NULL DEFAULT 1 + index).

ALTER TABLE quotations ADD COLUMN IF NOT EXISTS company_id BIGINT NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_quotations_company_id ON quotations(company_id);