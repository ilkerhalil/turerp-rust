-- 039_cari_financial_columns_numeric.down.sql
-- Revert cari financial columns to DOUBLE PRECISION (the pre-039 type).
-- Values within float8 precision are preserved. NOTE: re-introduces the
-- float8-vs-Decimal decode bug in CariRow (list/find/create 500 with data);
-- only run this when rolling back the full 039 change set.
ALTER TABLE cari ALTER COLUMN credit_limit    TYPE DOUBLE PRECISION USING credit_limit::double precision;
ALTER TABLE cari ALTER COLUMN current_balance TYPE DOUBLE PRECISION USING current_balance::double precision;