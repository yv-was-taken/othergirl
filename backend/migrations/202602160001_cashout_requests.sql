CREATE TABLE IF NOT EXISTS cashout_requests (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id           UUID NOT NULL REFERENCES users(id),
    amount            BIGINT NOT NULL CHECK (amount > 0),
    stripe_account_id VARCHAR(255) NOT NULL,
    status            VARCHAR(16) NOT NULL,
    stripe_transfer_id VARCHAR(255),
    failure_reason    TEXT,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at      TIMESTAMPTZ,
    failed_at         TIMESTAMPTZ,
    refunded_at       TIMESTAMPTZ,
    CHECK (status IN ('pending', 'completed', 'failed'))
);

CREATE INDEX IF NOT EXISTS idx_cashout_requests_user
    ON cashout_requests(user_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_cashout_requests_pending
    ON cashout_requests(status, created_at)
    WHERE status = 'pending';

CREATE UNIQUE INDEX IF NOT EXISTS idx_cashout_requests_pending_user
    ON cashout_requests(user_id)
    WHERE status = 'pending';

CREATE UNIQUE INDEX IF NOT EXISTS idx_cashout_requests_transfer_id
    ON cashout_requests(stripe_transfer_id)
    WHERE stripe_transfer_id IS NOT NULL;
