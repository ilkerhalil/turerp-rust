-- Migration: 016_full_text_search.sql
-- Full-text search with pg_trgm, unaccent, and Turkish tsvector

-- Enable extensions
CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE EXTENSION IF NOT EXISTS unaccent;

-- ============================================================================
-- Cari (Customer/Vendor) full-text search
-- ============================================================================

-- Add search_vector column for Turkish full-text search
ALTER TABLE cari ADD COLUMN IF NOT EXISTS search_vector tsvector;

-- GIN index on name for fast trigram (fuzzy) search
CREATE INDEX IF NOT EXISTS idx_cari_name_trgm ON cari USING gin (unaccent(name) gin_trgm_ops);

-- GIN index on code for fast trigram search
CREATE INDEX IF NOT EXISTS idx_cari_code_trgm ON cari USING gin (unaccent(code) gin_trgm_ops);

-- GIN index on search_vector for tsquery full-text search
CREATE INDEX IF NOT EXISTS idx_cari_search_vector ON cari USING gin (search_vector);

-- Trigger function to auto-update search_vector
CREATE OR REPLACE FUNCTION cari_search_vector_update()
RETURNS trigger AS $$
BEGIN
    NEW.search_vector :=
        setweight(to_tsvector('turkish', coalesce(unaccent(NEW.name), '')), 'A') ||
        setweight(to_tsvector('turkish', coalesce(unaccent(NEW.code), '')), 'B') ||
        setweight(to_tsvector('turkish', coalesce(unaccent(NEW.tax_office), '')), 'C') ||
        setweight(to_tsvector('turkish', coalesce(unaccent(NEW.city), '')), 'C') ||
        setweight(to_tsvector('turkish', coalesce(unaccent(NEW.address), '')), 'D');
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Attach trigger
DROP TRIGGER IF EXISTS trg_cari_search_vector ON cari;
CREATE TRIGGER trg_cari_search_vector
    BEFORE INSERT OR UPDATE ON cari
    FOR EACH ROW
    EXECUTE FUNCTION cari_search_vector_update();

-- Backfill existing rows
UPDATE cari SET search_vector =
    setweight(to_tsvector('turkish', coalesce(unaccent(name), '')), 'A') ||
    setweight(to_tsvector('turkish', coalesce(unaccent(code), '')), 'B') ||
    setweight(to_tsvector('turkish', coalesce(unaccent(tax_office), '')), 'C') ||
    setweight(to_tsvector('turkish', coalesce(unaccent(city), '')), 'C') ||
    setweight(to_tsvector('turkish', coalesce(unaccent(address), '')), 'D')
WHERE search_vector IS NULL;

-- ============================================================================
-- Products full-text search
-- ============================================================================

ALTER TABLE products ADD COLUMN IF NOT EXISTS search_vector tsvector;

CREATE INDEX IF NOT EXISTS idx_products_name_trgm ON products USING gin (unaccent(name) gin_trgm_ops);
CREATE INDEX IF NOT EXISTS idx_products_code_trgm ON products USING gin (unaccent(code) gin_trgm_ops);
CREATE INDEX IF NOT EXISTS idx_products_search_vector ON products USING gin (search_vector);

CREATE OR REPLACE FUNCTION products_search_vector_update()
RETURNS trigger AS $$
BEGIN
    NEW.search_vector :=
        setweight(to_tsvector('turkish', coalesce(unaccent(NEW.name), '')), 'A') ||
        setweight(to_tsvector('turkish', coalesce(unaccent(NEW.code), '')), 'B') ||
        setweight(to_tsvector('turkish', coalesce(unaccent(NEW.description), '')), 'C') ||
        setweight(to_tsvector('turkish', coalesce(unaccent(NEW.barcode), '')), 'B');
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_products_search_vector ON products;
CREATE TRIGGER trg_products_search_vector
    BEFORE INSERT OR UPDATE ON products
    FOR EACH ROW
    EXECUTE FUNCTION products_search_vector_update();

UPDATE products SET search_vector =
    setweight(to_tsvector('turkish', coalesce(unaccent(name), '')), 'A') ||
    setweight(to_tsvector('turkish', coalesce(unaccent(code), '')), 'B') ||
    setweight(to_tsvector('turkish', coalesce(unaccent(description), '')), 'C') ||
    setweight(to_tsvector('turkish', coalesce(unaccent(barcode), '')), 'B')
WHERE search_vector IS NULL;

-- ============================================================================
-- Invoices full-text search
-- ============================================================================

ALTER TABLE invoices ADD COLUMN IF NOT EXISTS search_vector tsvector;

CREATE INDEX IF NOT EXISTS idx_invoices_notes_trgm ON invoices USING gin (unaccent(notes) gin_trgm_ops);
CREATE INDEX IF NOT EXISTS idx_invoices_number_trgm ON invoices USING gin (unaccent(invoice_number) gin_trgm_ops);
CREATE INDEX IF NOT EXISTS idx_invoices_search_vector ON invoices USING gin (search_vector);

CREATE OR REPLACE FUNCTION invoices_search_vector_update()
RETURNS trigger AS $$
BEGIN
    NEW.search_vector :=
        setweight(to_tsvector('turkish', coalesce(unaccent(NEW.invoice_number), '')), 'A') ||
        setweight(to_tsvector('turkish', coalesce(unaccent(NEW.notes), '')), 'B');
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_invoices_search_vector ON invoices;
CREATE TRIGGER trg_invoices_search_vector
    BEFORE INSERT OR UPDATE ON invoices
    FOR EACH ROW
    EXECUTE FUNCTION invoices_search_vector_update();

UPDATE invoices SET search_vector =
    setweight(to_tsvector('turkish', coalesce(unaccent(invoice_number), '')), 'A') ||
    setweight(to_tsvector('turkish', coalesce(unaccent(notes), '')), 'B')
WHERE search_vector IS NULL;
