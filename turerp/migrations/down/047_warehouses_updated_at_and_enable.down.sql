-- Revert 047: re-disable the warehouse gate and drop the added column.
-- (Dropping updated_at re-breaks the BEFORE UPDATE trigger, restoring the
-- pre-fix 500-on-UPDATE state — consistent with a full down-revert.)
UPDATE feature_flags
    SET status = 'disabled', updated_at = NOW()
    WHERE name = 'core.stock.warehouses' AND tenant_id IS NULL;

ALTER TABLE warehouses
    DROP COLUMN IF EXISTS updated_at;