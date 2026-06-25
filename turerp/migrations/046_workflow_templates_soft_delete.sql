-- 046_workflow_templates_soft_delete.sql
--
-- workflow_templates (migration 024_workflows.sql) was created AFTER
-- migration 020 (soft_delete_complete) but WITHOUT the soft-delete
-- columns, even though WorkflowTemplateRow
-- (src/domain/workflow/postgres_repository.rs:21) declares
--   deleted_at: Option<DateTime<Utc>>
--   deleted_by: Option<i64>
-- and every read/write query references them and filters
-- `WHERE deleted_at IS NULL` (find_templates, find_template_by_id,
-- create_template RETURNING, delete_template SET deleted_at = NOW()).
-- So every query against workflow_templates fails with
-- `column "deleted_at" does not exist` -> HTTP 500 on
-- GET /api/v1/workflows/templates (and find_template_by_id / create /
-- delete soft-delete paths).
--
-- Same soft-delete-missing-column class as tax_periods (040),
-- reconciliation_rules (044), and subscriptions (045 — which was broader,
-- also adding feature columns). Here only the two soft-delete columns are
-- missing; the other 8 columns match WorkflowTemplateRow exactly.
-- Mirrors the 020/040/044 declaration:
--   deleted_at TIMESTAMPTZ (nullable)
--   deleted_by BIGINT REFERENCES users(id) ON DELETE SET NULL
-- plus the partial-index convention from 020/040/044.
--
-- NOTE (follow-up, pre-existing, NOT addressed here): the soft-delete
-- write path (delete_template) SETs deleted_at = NOW() but takes no
-- deleted_by argument, so deleted_by is never populated on soft-delete
-- (stays NULL). Same audit-attribution residual class as
-- reconciliation_rules.delete_rule (044) and subscription soft-deletes
-- (045); the column is required because WorkflowTemplateRow SELECTs it.
ALTER TABLE workflow_templates
    ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS deleted_by BIGINT REFERENCES users(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_workflow_templates_deleted_at
    ON workflow_templates(deleted_at) WHERE deleted_at IS NULL;