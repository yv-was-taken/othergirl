-- OAuth and profile interests
CREATE TABLE IF NOT EXISTS oauth_accounts (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider    VARCHAR(32) NOT NULL,
    provider_id VARCHAR(255) NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(provider, provider_id)
);
CREATE INDEX IF NOT EXISTS idx_oauth_user ON oauth_accounts(user_id);

CREATE TABLE IF NOT EXISTS user_interests (
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    category_id UUID NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
    PRIMARY KEY (user_id, category_id)
);

-- Safety + moderation
CREATE TABLE IF NOT EXISTS reports (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    reporter_id UUID NOT NULL REFERENCES users(id),
    reported_id UUID NOT NULL REFERENCES users(id),
    chat_id     UUID NOT NULL REFERENCES chats(id),
    reason      VARCHAR(32) NOT NULL,
    details     TEXT NOT NULL DEFAULT '',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(reporter_id, reported_id, chat_id)
);
CREATE INDEX IF NOT EXISTS idx_reports_reported ON reports(reported_id, created_at DESC);

CREATE TABLE IF NOT EXISTS blocks (
    blocker_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    blocked_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (blocker_id, blocked_id)
);
CREATE INDEX IF NOT EXISTS idx_blocks_blocked ON blocks(blocked_id);

CREATE TABLE IF NOT EXISTS message_flags (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    message_id UUID NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    user_id    UUID NOT NULL REFERENCES users(id),
    reasons    JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_message_flags_user ON message_flags(user_id, created_at DESC);

-- Encryption at rest
CREATE TABLE IF NOT EXISTS chat_keys (
    chat_id     UUID PRIMARY KEY REFERENCES chats(id) ON DELETE CASCADE,
    key_encrypted BYTEA NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    rotated_at  TIMESTAMPTZ
);

ALTER TABLE messages
    ADD COLUMN IF NOT EXISTS content_encrypted BYTEA,
    ADD COLUMN IF NOT EXISTS nonce BYTEA;
ALTER TABLE messages
    ALTER COLUMN content_text DROP NOT NULL;

-- Monetization
CREATE TABLE IF NOT EXISTS subscriptions (
    id                     UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id                UUID NOT NULL REFERENCES users(id),
    stripe_subscription_id VARCHAR(255) UNIQUE NOT NULL,
    stripe_customer_id     VARCHAR(255) NOT NULL,
    status                 VARCHAR(32) NOT NULL,
    current_period_start   TIMESTAMPTZ NOT NULL,
    current_period_end     TIMESTAMPTZ NOT NULL,
    created_at             TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_subscriptions_user ON subscriptions(user_id);

CREATE TABLE IF NOT EXISTS spark_transactions (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id          UUID NOT NULL REFERENCES users(id),
    amount           BIGINT NOT NULL,
    transaction_type VARCHAR(32) NOT NULL,
    reference_id     UUID,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_spark_tx_user ON spark_transactions(user_id, created_at DESC);

CREATE TABLE IF NOT EXISTS awards (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sender_id     UUID NOT NULL REFERENCES users(id),
    recipient_id  UUID NOT NULL REFERENCES users(id),
    chat_id       UUID NOT NULL REFERENCES chats(id),
    award_type    VARCHAR(32) NOT NULL,
    spark_amount  BIGINT NOT NULL,
    recipient_cut BIGINT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_awards_recipient ON awards(recipient_id, created_at DESC);

CREATE TABLE IF NOT EXISTS flare_items (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name         VARCHAR(64) NOT NULL,
    description  TEXT NOT NULL DEFAULT '',
    item_type    VARCHAR(32) NOT NULL,
    price_sparks BIGINT NOT NULL,
    rarity       VARCHAR(16) NOT NULL DEFAULT 'common',
    asset_data   JSONB NOT NULL DEFAULT '{}'::jsonb,
    is_active    BOOLEAN NOT NULL DEFAULT TRUE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS user_flare (
    user_id       UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    flare_item_id UUID NOT NULL REFERENCES flare_items(id),
    is_equipped   BOOLEAN NOT NULL DEFAULT FALSE,
    purchased_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, flare_item_id)
);

CREATE TABLE IF NOT EXISTS stripe_connect_accounts (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id           UUID UNIQUE NOT NULL REFERENCES users(id),
    stripe_account_id VARCHAR(255) UNIQUE NOT NULL,
    payouts_enabled   BOOLEAN NOT NULL DEFAULT FALSE,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
