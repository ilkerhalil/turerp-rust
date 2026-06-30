-- Add tenant_id to stock_levels for multi-tenant isolation.
--
-- stock_levels was created in 003 without tenant_id (see the stale note in
-- repository.rs: "StockLevel does not have a tenant_id field; it is a child
-- entity of Warehouse. Tenant isolation is enforced via the warehouse_id
-- relationship" — the repo/service/handlers never actually did that). This
-- adds the column and backfills it from the parent product's tenant_id
-- (products.tenant_id is NOT NULL), then makes it NOT NULL + FK'd to
-- tenants(id).
--
-- Mirrors 033_stock_movements_tenant_id.sql and 048_hr_attendance_leave_*
-- with the data-safety deviation that the backfill is a JOIN on products
-- (not a blanket = 1), because stock_levels legitimately spans tenants via
-- product_id (and warehouse_id). products is the cleaner single-hop parent
-- (warehouses.tenant_id is also NOT NULL and would work, but products avoids
-- the warehouses company_id=0 Rust-side phantom).

ALTER TABLE stock_levels ADD COLUMN IF NOT EXISTS tenant_id BIGINT;

-- Backfill from the parent product's tenant (product_id is NOT NULL FK to
-- products, whose tenant_id is NOT NULL → every row gets a valid tenant_id).
UPDATE stock_levels s
SET tenant_id = p.tenant_id
FROM products p
WHERE s.product_id = p.id AND s.tenant_id IS NULL;

ALTER TABLE stock_levels ALTER COLUMN tenant_id SET NOT NULL;

ALTER TABLE stock_levels ADD CONSTRAINT fk_stock_levels_tenant
    FOREIGN KEY (tenant_id) REFERENCES tenants(id);

-- Index names pre-checked against all existing migrations (no collisions).
CREATE INDEX IF NOT EXISTS idx_stock_levels_tenant_id ON stock_levels(tenant_id);
-- Covers tenant-scoped find_by_product (tenant_id, product_id) lookups.
CREATE INDEX IF NOT EXISTS idx_stock_levels_tenant_product
    ON stock_levels(tenant_id, product_id);

-- NOTE: the existing unique idx_stock_levels_warehouse_product(warehouse_id,
-- product_id) is intentionally KEPT — it is the ON CONFLICT target for the
-- update_quantity/reserve/release UPSERTs. warehouse_id maps to exactly one
-- tenant (warehouses.tenant_id NOT NULL), so (warehouse_id, product_id)
-- already implies per-tenant uniqueness; no rebuild needed (lower risk).