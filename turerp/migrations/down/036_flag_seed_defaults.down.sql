-- 036_flag_seed_defaults.down.sql
-- Reversible: drops the 12 default feature flags added by the up.
-- This down is safe because the flags are seeded on every app start
-- if missing (the FeatureFlagService.ensure_seeded path runs on
-- startup), so a fresh boot after the down re-creates them.
DELETE FROM feature_flags
WHERE name IN (
    'tier2.manufacturing',
    'tier2.projects',
    'tier2.shifts',
    'tier2.payroll',
    'tier2.graphql',
    'tier2.file_upload',
    'core.categories',
    'core.units',
    'core.currencies',
    'core.hr.leave_types',
    'core.stock.warehouses',
    'core.settings'
);
