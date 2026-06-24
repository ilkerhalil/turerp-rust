-- Make employees.updated_at NOT NULL with a DEFAULT NOW().
--
-- Root cause being fixed: the employees table defined updated_at as a nullable
-- column with no default and no BEFORE-UPDATE trigger backfilling it, while the
-- EmployeeRow struct (src/domain/hr/postgres_repository.rs) decodes it as a
-- non-null DateTime<Utc>. The API's create()/update() always set updated_at
-- explicitly, so the API path never produced a NULL — but any row inserted
-- outside the API without an explicit updated_at (seed scripts, manual SQL,
-- future bulk imports) left it NULL, and sqlx::FromRow then failed decoding
-- the row, making GET /hr/employees return HTTP 500.
--
-- The fix aligns the schema with the code's non-null assumption: a NOT NULL
-- DEFAULT NOW() column auto-fills updated_at on insert (matching created_at's
-- semantics and the app's own behavior) and prevents the decode failure for
-- any writer, not just the API.
--
-- Safe on existing data: backfill any NULLs first, then add the default and
-- the NOT NULL constraint. All three statements are idempotent on re-run
-- against an already-conforming column (the UPDATE matches zero rows; SET
-- DEFAULT / SET NOT NULL are no-ops if already set).

UPDATE employees SET updated_at = NOW() WHERE updated_at IS NULL;

ALTER TABLE employees ALTER COLUMN updated_at SET DEFAULT NOW();

ALTER TABLE employees ALTER COLUMN updated_at SET NOT NULL;