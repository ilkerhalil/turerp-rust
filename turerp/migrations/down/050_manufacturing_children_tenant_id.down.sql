-- 050_manufacturing_children_tenant_id.down.sql
-- Reverse of 050: drop the indexes, the FK constraints, then the tenant_id
-- columns on all four manufacturing child tables. Idempotent.
--
-- DESTRUCTIVE (data loss): dropping tenant_id loses the columns we backfilled
-- from the parent tables' tenant_id. Intended only for the offline/conservative
-- down-replay path documented in src/db/pool.rs (use scripts/backup_pg.sh
-- for a real point-in-time rollback). Re-adding the columns later would
-- require re-running the backfills. The original cross-tenant leaks return
-- (isolation was never actually enforced pre-050). Run only against a
-- snapshot where losing the tenant_id columns on work_order_operations,
-- work_order_materials, bom_lines, and routing_operations is acceptable.

-- work_order_operations
DROP INDEX IF EXISTS idx_work_order_operations_tenant_wo;
DROP INDEX IF EXISTS idx_work_order_operations_tenant_id;
ALTER TABLE work_order_operations DROP CONSTRAINT IF EXISTS fk_work_order_operations_tenant;
ALTER TABLE work_order_operations DROP COLUMN IF EXISTS tenant_id;

-- work_order_materials
DROP INDEX IF EXISTS idx_work_order_materials_tenant_wo;
DROP INDEX IF EXISTS idx_work_order_materials_tenant_id;
ALTER TABLE work_order_materials DROP CONSTRAINT IF EXISTS fk_work_order_materials_tenant;
ALTER TABLE work_order_materials DROP COLUMN IF EXISTS tenant_id;

-- bom_lines
DROP INDEX IF EXISTS idx_bom_lines_tenant_bom;
DROP INDEX IF EXISTS idx_bom_lines_tenant_id;
ALTER TABLE bom_lines DROP CONSTRAINT IF EXISTS fk_bom_lines_tenant;
ALTER TABLE bom_lines DROP COLUMN IF EXISTS tenant_id;

-- routing_operations
DROP INDEX IF EXISTS idx_routing_operations_tenant_routing;
DROP INDEX IF EXISTS idx_routing_operations_tenant_id;
ALTER TABLE routing_operations DROP CONSTRAINT IF EXISTS fk_routing_operations_tenant;
ALTER TABLE routing_operations DROP COLUMN IF EXISTS tenant_id;