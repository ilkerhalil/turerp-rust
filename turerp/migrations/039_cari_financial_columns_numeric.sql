-- 039_cari_financial_columns_numeric.sql
--
-- cari.credit_limit and cari.current_balance were declared DOUBLE PRECISION
-- (float8) in 001_initial_schema, but CariRow (and Cari) decode them as
-- rust_decimal::Decimal. sqlx cannot decode a float8 column into a Decimal
-- (mismatched type oid -> ColumnDecode error), so every query that SELECTs
-- or RETURNs these columns — find_by_id, find_by_tenant (GET /caris),
-- find_by_type (GET /caris/type/{t}), and create() RETURNING — fails with
-- HTTP 500 the moment a row exists. The hurl smoke path only exercised the
-- empty-list case, so the bug stayed latent until sample data was seeded.
--
-- Migrate both columns to NUMERIC(18,4): this matches the Decimal struct
-- and the convention used by every other monetary column in the schema
-- (invoices, products, etc.). float8 -> numeric(18,4) is lossless for any
-- value within the target scale.
ALTER TABLE cari ALTER COLUMN credit_limit    TYPE NUMERIC(18,4) USING credit_limit::numeric(18,4);
ALTER TABLE cari ALTER COLUMN current_balance TYPE NUMERIC(18,4) USING current_balance::numeric(18,4);