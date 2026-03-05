-- payment_webhook_events accumulates Stripe webhook payloads (JSONB).
-- Add an index on received_at to support efficient periodic cleanup.
-- Recommended: run a scheduled job to DELETE WHERE received_at < NOW() - INTERVAL '90 days'.
CREATE INDEX IF NOT EXISTS idx_webhook_events_received_at ON payment_webhook_events(received_at);
