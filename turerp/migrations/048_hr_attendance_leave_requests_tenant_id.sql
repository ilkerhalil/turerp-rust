-- Add tenant_id to attendance + leave_requests for multi-tenant isolation.
--
-- Both tables were created in 003 without tenant_id (see the note in
-- 004_composite_indexes.sql: attendance and leave_requests do not carry
-- tenant_id; isolation was "enforced indirectly via employees.tenant_id",
-- which the repo/service/handlers never actually did). This adds the column
-- and backfills it from the parent employee's tenant_id (employees.tenant_id
-- is NOT NULL), then makes it NOT NULL + FK'd to tenants(id).
--
-- Mirrors 033_stock_movements_tenant_id.sql, with the data-safety deviation
-- that the backfill is a JOIN on employees (not a blanket = 1), because
-- attendance/leave_requests legitimately span tenants via employee_id.

-- attendance ---------------------------------------------------------------
ALTER TABLE attendance ADD COLUMN IF NOT EXISTS tenant_id BIGINT;

-- Backfill from the parent employee's tenant (employee_id is NOT NULL FK to
-- employees, whose tenant_id is NOT NULL → every row gets a valid tenant_id).
UPDATE attendance a
SET tenant_id = e.tenant_id
FROM employees e
WHERE a.employee_id = e.id AND a.tenant_id IS NULL;

ALTER TABLE attendance ALTER COLUMN tenant_id SET NOT NULL;

ALTER TABLE attendance ADD CONSTRAINT fk_attendance_tenant
    FOREIGN KEY (tenant_id) REFERENCES tenants(id);

-- NOTE: name must NOT collide with 035's `idx_attendance_tenant` on
-- attendance_records(tenant_id) — PostgreSQL index names are unique per
-- schema and CREATE INDEX IF NOT EXISTS checks by name only, so a colliding
-- name would silently skip and leave attendance(tenant_id) unindexed.
CREATE INDEX IF NOT EXISTS idx_attendance_tenant_id ON attendance(tenant_id);
-- Covers tenant-scoped find_by_employee (tenant_id, employee_id) lookups.
CREATE INDEX IF NOT EXISTS idx_attendance_tenant_employee_date
    ON attendance(tenant_id, employee_id, date);

-- leave_requests -----------------------------------------------------------
ALTER TABLE leave_requests ADD COLUMN IF NOT EXISTS tenant_id BIGINT;

UPDATE leave_requests lr
SET tenant_id = e.tenant_id
FROM employees e
WHERE lr.employee_id = e.id AND lr.tenant_id IS NULL;

ALTER TABLE leave_requests ALTER COLUMN tenant_id SET NOT NULL;

ALTER TABLE leave_requests ADD CONSTRAINT fk_leave_requests_tenant
    FOREIGN KEY (tenant_id) REFERENCES tenants(id);

CREATE INDEX IF NOT EXISTS idx_leave_requests_tenant ON leave_requests(tenant_id);
CREATE INDEX IF NOT EXISTS idx_leave_requests_tenant_employee
    ON leave_requests(tenant_id, employee_id);