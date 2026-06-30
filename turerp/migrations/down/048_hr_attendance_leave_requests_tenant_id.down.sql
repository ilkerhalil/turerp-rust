-- 048_hr_attendance_leave_requests_tenant_id.down.sql
-- Reverse of 048: drop the indexes, the FK constraints, then the tenant_id
-- columns. Idempotent.
--
-- DESTRUCTIVE (data loss): dropping tenant_id loses the column we backfilled
-- from employees.tenant_id. Intended only for the offline/conservative
-- down-replay path documented in src/db/pool.rs (use scripts/backup_pg.sh for
-- a real point-in-time rollback). Re-adding the column later would require
-- re-running the backfill. The original cross-tenant leak returns (isolation
-- was never actually enforced pre-048). Run only against a snapshot where
-- losing the tenant_id column on attendance/leave_requests is acceptable.
DROP INDEX IF EXISTS idx_attendance_tenant_employee_date;
DROP INDEX IF EXISTS idx_attendance_tenant_id;
ALTER TABLE attendance DROP CONSTRAINT IF EXISTS fk_attendance_tenant;
ALTER TABLE attendance DROP COLUMN IF EXISTS tenant_id;

DROP INDEX IF EXISTS idx_leave_requests_tenant_employee;
DROP INDEX IF EXISTS idx_leave_requests_tenant;
ALTER TABLE leave_requests DROP CONSTRAINT IF EXISTS fk_leave_requests_tenant;
ALTER TABLE leave_requests DROP COLUMN IF EXISTS tenant_id;