-- Add missing index on subscriptions.stripe_customer_id for payment/webhook lookups
CREATE INDEX IF NOT EXISTS idx_subscriptions_stripe_customer ON subscriptions(stripe_customer_id);
