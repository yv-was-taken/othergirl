-- payment_webhook_events accumulates Stripe webhook payloads (JSONB).
-- Add an index on created_at to support efficient periodic cleanup.
-- Recommended: run a scheduled job to DELETE WHERE created_at < NOW() - INTERVAL '90 days'.
CREATE INDEX IF NOT EXISTS idx_webhook_events_created_at ON payment_webhook_events(created_at);
