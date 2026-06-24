-- 038_employees_updated_at_not_null.down.sql
-- Reversible: restore the original nullable, no-default updated_at column on
-- employees. No data is lost — existing updated_at values are preserved; only
-- the NOT NULL constraint and the DEFAULT are dropped, so future inserts can
-- once again omit updated_at (the pre-038 behavior).
ALTER TABLE employees ALTER COLUMN updated_at DROP NOT NULL;
ALTER TABLE employees ALTER COLUMN updated_at DROP DEFAULT;