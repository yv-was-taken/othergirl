CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_cashout_requests_updated_at
    BEFORE UPDATE ON cashout_requests
    FOR EACH ROW
    EXECUTE FUNCTION set_updated_at();
