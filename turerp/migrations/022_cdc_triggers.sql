-- CDC trigger function: notify on table changes via pg_notify
CREATE OR REPLACE FUNCTION notify_on_change()
RETURNS TRIGGER AS $$
DECLARE
    payload JSONB;
    channel TEXT := TG_ARGV[0];
    record_id BIGINT;
    t_id BIGINT;
BEGIN
    IF TG_OP = 'DELETE' THEN
        record_id := OLD.id;
        t_id := OLD.tenant_id;
        payload := jsonb_build_object(
            'table', TG_TABLE_NAME,
            'operation', TG_OP,
            'id', record_id,
            'tenant_id', t_id,
            'old', to_jsonb(OLD),
            'new', null
        );
    ELSE
        record_id := NEW.id;
        t_id := NEW.tenant_id;
        payload := jsonb_build_object(
            'table', TG_TABLE_NAME,
            'operation', TG_OP,
            'id', record_id,
            'tenant_id', t_id,
            'old', CASE WHEN TG_OP = 'UPDATE' THEN to_jsonb(OLD) ELSE null END,
            'new', to_jsonb(NEW)
        );
    END IF;

    PERFORM pg_notify(channel, payload::text);
    RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;

-- Attach triggers to watched tables
DROP TRIGGER IF EXISTS cdc_trigger ON invoices;
CREATE TRIGGER cdc_trigger
    AFTER INSERT OR UPDATE OR DELETE ON invoices
    FOR EACH ROW
    EXECUTE FUNCTION notify_on_change('invoice_changes');

DROP TRIGGER IF EXISTS cdc_trigger ON cari;
CREATE TRIGGER cdc_trigger
    AFTER INSERT OR UPDATE OR DELETE ON cari
    FOR EACH ROW
    EXECUTE FUNCTION notify_on_change('cari_changes');

DROP TRIGGER IF EXISTS cdc_trigger ON stock_movements;
CREATE TRIGGER cdc_trigger
    AFTER INSERT OR UPDATE OR DELETE ON stock_movements
    FOR EACH ROW
    EXECUTE FUNCTION notify_on_change('stock_changes');

DROP TRIGGER IF EXISTS cdc_trigger ON products;
CREATE TRIGGER cdc_trigger
    AFTER INSERT OR UPDATE OR DELETE ON products
    FOR EACH ROW
    EXECUTE FUNCTION notify_on_change('product_changes');

DROP TRIGGER IF EXISTS cdc_trigger ON payments;
CREATE TRIGGER cdc_trigger
    AFTER INSERT OR UPDATE OR DELETE ON payments
    FOR EACH ROW
    EXECUTE FUNCTION notify_on_change('payment_changes');

DROP TRIGGER IF EXISTS cdc_trigger ON journal_entries;
CREATE TRIGGER cdc_trigger
    AFTER INSERT OR UPDATE OR DELETE ON journal_entries
    FOR EACH ROW
    EXECUTE FUNCTION notify_on_change('journal_changes');
