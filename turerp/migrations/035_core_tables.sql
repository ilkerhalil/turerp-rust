-- Migration 035: Core tables missing from earlier migrations
--
-- The following 10 tables are referenced by Postgres repositories but were
-- never created. They are wired into AppState in lib.rs::create_app_state,
-- so production deployments crash at first query against document, shift,
-- or archive endpoints with "relation does not exist".
--
-- Sources of truth for the column shape are the corresponding
-- *Row structs in turerp/src/domain/{document,shift,archive}/postgres_repository.rs.

-- ---------------------------------------------------------------------------
-- Documents (general-purpose file attachments)
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS document_categories (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    name VARCHAR(100) NOT NULL,
    parent_id BIGINT REFERENCES document_categories(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (tenant_id, name)
);
CREATE INDEX IF NOT EXISTS idx_document_categories_tenant ON document_categories(tenant_id);

CREATE TABLE IF NOT EXISTS documents (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    title VARCHAR(255) NOT NULL,
    file_name VARCHAR(255) NOT NULL,
    file_size BIGINT NOT NULL,
    mime_type VARCHAR(100) NOT NULL,
    storage_path TEXT NOT NULL,
    category_id BIGINT REFERENCES document_categories(id) ON DELETE SET NULL,
    uploaded_by BIGINT NOT NULL,
    description TEXT,
    is_deleted BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_documents_tenant ON documents(tenant_id);
CREATE INDEX IF NOT EXISTS idx_documents_category ON documents(tenant_id, category_id);
CREATE INDEX IF NOT EXISTS idx_documents_uploaded_by ON documents(tenant_id, uploaded_by);

CREATE TABLE IF NOT EXISTS document_links (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    document_id BIGINT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    entity_type VARCHAR(50) NOT NULL,
    entity_id BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_document_links_doc ON document_links(tenant_id, document_id);
CREATE INDEX IF NOT EXISTS idx_document_links_entity ON document_links(tenant_id, entity_type, entity_id);

CREATE TABLE IF NOT EXISTS document_versions (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    document_id BIGINT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    version_number INT NOT NULL,
    storage_path TEXT NOT NULL,
    file_size BIGINT NOT NULL,
    uploaded_by BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (document_id, version_number)
);
CREATE INDEX IF NOT EXISTS idx_document_versions_doc ON document_versions(tenant_id, document_id);

-- ---------------------------------------------------------------------------
-- Shifts & attendance
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS shifts (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    name VARCHAR(100) NOT NULL,
    start_time TIME NOT NULL,
    end_time TIME NOT NULL,
    break_minutes INT NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    is_deleted BOOLEAN NOT NULL DEFAULT false
);
CREATE INDEX IF NOT EXISTS idx_shifts_tenant ON shifts(tenant_id);
CREATE INDEX IF NOT EXISTS idx_shifts_tenant_active ON shifts(tenant_id, is_active);

CREATE TABLE IF NOT EXISTS shift_assignments (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    employee_id BIGINT NOT NULL,
    shift_id BIGINT NOT NULL REFERENCES shifts(id) ON DELETE CASCADE,
    assignment_date DATE NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (tenant_id, employee_id, assignment_date)
);
CREATE INDEX IF NOT EXISTS idx_shift_assignments_tenant ON shift_assignments(tenant_id);
CREATE INDEX IF NOT EXISTS idx_shift_assignments_employee ON shift_assignments(tenant_id, employee_id);
CREATE INDEX IF NOT EXISTS idx_shift_assignments_shift ON shift_assignments(tenant_id, shift_id);

CREATE TABLE IF NOT EXISTS attendance_records (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    employee_id BIGINT NOT NULL,
    clock_in TIMESTAMPTZ NOT NULL,
    clock_out TIMESTAMPTZ,
    break_minutes INT NOT NULL DEFAULT 0,
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_attendance_tenant ON attendance_records(tenant_id);
CREATE INDEX IF NOT EXISTS idx_attendance_employee ON attendance_records(tenant_id, employee_id);
CREATE INDEX IF NOT EXISTS idx_attendance_clock_in ON attendance_records(tenant_id, clock_in);

-- ---------------------------------------------------------------------------
-- Archive (long-term storage policies and jobs)
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS archive_policies (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    entity_type VARCHAR(50) NOT NULL,
    retention_days INT NOT NULL,
    storage_target VARCHAR(50) NOT NULL DEFAULT 'cold',
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_archive_policies_tenant ON archive_policies(tenant_id);
CREATE INDEX IF NOT EXISTS idx_archive_policies_entity ON archive_policies(tenant_id, entity_type);

CREATE TABLE IF NOT EXISTS archive_jobs (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    policy_id BIGINT NOT NULL REFERENCES archive_policies(id) ON DELETE CASCADE,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    status VARCHAR(20) NOT NULL DEFAULT 'Running',
    records_archived BIGINT NOT NULL DEFAULT 0,
    error_message TEXT
);
CREATE INDEX IF NOT EXISTS idx_archive_jobs_tenant ON archive_jobs(tenant_id);
CREATE INDEX IF NOT EXISTS idx_archive_jobs_policy ON archive_jobs(tenant_id, policy_id);
CREATE INDEX IF NOT EXISTS idx_archive_jobs_status ON archive_jobs(tenant_id, status);

CREATE TABLE IF NOT EXISTS archive_records (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    job_id BIGINT NOT NULL REFERENCES archive_jobs(id) ON DELETE CASCADE,
    entity_type VARCHAR(50) NOT NULL,
    entity_id BIGINT NOT NULL,
    archived_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    storage_path TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_archive_records_tenant ON archive_records(tenant_id);
CREATE INDEX IF NOT EXISTS idx_archive_records_job ON archive_records(tenant_id, job_id);
CREATE INDEX IF NOT EXISTS idx_archive_records_entity ON archive_records(tenant_id, entity_type, entity_id);
