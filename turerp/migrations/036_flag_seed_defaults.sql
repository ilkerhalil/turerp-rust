-- 036_flag_seed_defaults.sql
-- Seed the 12 default feature flags for the production release cut.
-- All flags are tenant_id=NULL (global) and Disabled by default. Operators
-- enable per-tenant via PATCH /api/v1/admin/tenants/{id}/flags (added in a
-- follow-up; for v1, the existing /api/v1/feature-flags/{id}/enable route
-- is used, scoped by tenant_id set at flag creation time).
--
-- Flag identifiers use reverse-DNS namespace per design § 4.1.

INSERT INTO feature_flags (name, description, status, tenant_id, created_at, updated_at)
VALUES
  ('tier2.manufacturing',     'Manufacturing module (work orders, BOM)',           'disabled', NULL, NOW(), NOW()),
  ('tier2.projects',          'Projects module (PM)',                              'disabled', NULL, NOW(), NOW()),
  ('tier2.shifts',            'Shifts module (shift scheduling)',                  'disabled', NULL, NOW(), NOW()),
  ('tier2.payroll',           'Payroll module (salary calculation)',               'disabled', NULL, NOW(), NOW()),
  ('tier2.graphql',           'GraphQL endpoint at /api/v1/graphql',               'disabled', NULL, NOW(), NOW()),
  ('tier2.file_upload',       'S3-backed file upload routes',                      'disabled', NULL, NOW(), NOW()),
  ('core.categories',         'GET/POST /api/v1/categories — flip to on after #152-1 fix', 'disabled', NULL, NOW(), NOW()),
  ('core.units',              'GET/POST /api/v1/units — flip to on after #152-2 fix',      'disabled', NULL, NOW(), NOW()),
  ('core.currencies',         'GET/POST /api/v1/currencies — flip to on after #152-3 fix', 'disabled', NULL, NOW(), NOW()),
  ('core.hr.leave_types',     'GET/POST /api/v1/hr/leave-types — flip to on after #152-4 fix', 'disabled', NULL, NOW(), NOW()),
  ('core.stock.warehouses',   'GET/POST /api/v1/stock/warehouses — flip to on after #152-5 fix', 'disabled', NULL, NOW(), NOW()),
  ('core.settings',           'GET/POST /api/v1/settings — flip to on after #152-6 fix', 'disabled', NULL, NOW(), NOW())
ON CONFLICT (name, tenant_id) DO NOTHING;
