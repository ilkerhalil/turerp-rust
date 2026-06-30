-- 050: Add tenant_id to manufacturing child tables
--
-- Closes the migration-needed cross-tenant leak on the manufacturing child
-- tables (work_order_operations, work_order_materials, bom_lines,
-- routing_operations), which 003 created with only a parent FK and no
-- tenant_id. Each child is backfilled from its parent's NOT NULL tenant_id:
--   work_order_operations / work_order_materials ← work_orders
--   bom_lines                               ← bills_of_materials
--   routing_operations                       ← routings
-- All parent FKs are NOT NULL and every parent has tenant_id NOT NULL
-- REFERENCES tenants(id), so every existing row receives a valid tenant_id
-- and SET NOT NULL / the new FK are safe.
-- Mirrors 048 (hr attendance/leave) and 049 (stock_levels).

-- work_order_operations ← work_orders
ALTER TABLE work_order_operations ADD COLUMN IF NOT EXISTS tenant_id BIGINT;
UPDATE work_order_operations o
  SET tenant_id = w.tenant_id
  FROM work_orders w
  WHERE o.work_order_id = w.id AND o.tenant_id IS NULL;
ALTER TABLE work_order_operations ALTER COLUMN tenant_id SET NOT NULL;
ALTER TABLE work_order_operations ADD CONSTRAINT fk_work_order_operations_tenant
  FOREIGN KEY (tenant_id) REFERENCES tenants(id);
CREATE INDEX IF NOT EXISTS idx_work_order_operations_tenant_id
  ON work_order_operations(tenant_id);
CREATE INDEX IF NOT EXISTS idx_work_order_operations_tenant_wo
  ON work_order_operations(tenant_id, work_order_id);

-- work_order_materials ← work_orders
ALTER TABLE work_order_materials ADD COLUMN IF NOT EXISTS tenant_id BIGINT;
UPDATE work_order_materials m
  SET tenant_id = w.tenant_id
  FROM work_orders w
  WHERE m.work_order_id = w.id AND m.tenant_id IS NULL;
ALTER TABLE work_order_materials ALTER COLUMN tenant_id SET NOT NULL;
ALTER TABLE work_order_materials ADD CONSTRAINT fk_work_order_materials_tenant
  FOREIGN KEY (tenant_id) REFERENCES tenants(id);
CREATE INDEX IF NOT EXISTS idx_work_order_materials_tenant_id
  ON work_order_materials(tenant_id);
CREATE INDEX IF NOT EXISTS idx_work_order_materials_tenant_wo
  ON work_order_materials(tenant_id, work_order_id);

-- bom_lines ← bills_of_materials
ALTER TABLE bom_lines ADD COLUMN IF NOT EXISTS tenant_id BIGINT;
UPDATE bom_lines l
  SET tenant_id = b.tenant_id
  FROM bills_of_materials b
  WHERE l.bom_id = b.id AND l.tenant_id IS NULL;
ALTER TABLE bom_lines ALTER COLUMN tenant_id SET NOT NULL;
ALTER TABLE bom_lines ADD CONSTRAINT fk_bom_lines_tenant
  FOREIGN KEY (tenant_id) REFERENCES tenants(id);
CREATE INDEX IF NOT EXISTS idx_bom_lines_tenant_id
  ON bom_lines(tenant_id);
CREATE INDEX IF NOT EXISTS idx_bom_lines_tenant_bom
  ON bom_lines(tenant_id, bom_id);

-- routing_operations ← routings
ALTER TABLE routing_operations ADD COLUMN IF NOT EXISTS tenant_id BIGINT;
UPDATE routing_operations r
  SET tenant_id = rt.tenant_id
  FROM routings rt
  WHERE r.routing_id = rt.id AND r.tenant_id IS NULL;
ALTER TABLE routing_operations ALTER COLUMN tenant_id SET NOT NULL;
ALTER TABLE routing_operations ADD CONSTRAINT fk_routing_operations_tenant
  FOREIGN KEY (tenant_id) REFERENCES tenants(id);
CREATE INDEX IF NOT EXISTS idx_routing_operations_tenant_id
  ON routing_operations(tenant_id);
CREATE INDEX IF NOT EXISTS idx_routing_operations_tenant_routing
  ON routing_operations(tenant_id, routing_id);