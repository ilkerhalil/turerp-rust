-- File metadata and storage tracking table
-- Supports both local and S3 backends with soft delete

CREATE TABLE IF NOT EXISTS files (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    filename TEXT NOT NULL,
    original_filename TEXT NOT NULL,
    content_type TEXT NOT NULL,
    size_bytes BIGINT NOT NULL,
    storage_path TEXT NOT NULL,
    storage_backend TEXT NOT NULL DEFAULT 'local',
    checksum TEXT NOT NULL,
    uploaded_by BIGINT,
    entity_type TEXT,
    entity_id BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ,
    deleted_by BIGINT
);

-- Tenant isolation index
CREATE INDEX IF NOT EXISTS idx_files_tenant_id ON files(tenant_id);

-- Soft delete filter index
CREATE INDEX IF NOT EXISTS idx_files_tenant_deleted_at ON files(tenant_id, deleted_at) WHERE deleted_at IS NULL;

-- Entity lookup index
CREATE INDEX IF NOT EXISTS idx_files_entity ON files(entity_type, entity_id) WHERE deleted_at IS NULL;

-- File creation time index for sorting
CREATE INDEX IF NOT EXISTS idx_files_created_at ON files(created_at DESC);

-- Storage backend index
CREATE INDEX IF NOT EXISTS idx_files_storage_backend ON files(storage_backend);
