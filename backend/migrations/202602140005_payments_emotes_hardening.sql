CREATE TABLE IF NOT EXISTS payment_webhook_events (
    stripe_event_id VARCHAR(255) PRIMARY KEY,
    event_type      VARCHAR(128) NOT NULL,
    payload         JSONB NOT NULL DEFAULT '{}'::jsonb,
    received_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS stripe_completed_sessions (
    stripe_session_id VARCHAR(255) PRIMARY KEY,
    user_id           UUID NOT NULL REFERENCES users(id),
    kind              VARCHAR(32) NOT NULL,
    amount            BIGINT NOT NULL,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_stripe_completed_sessions_user
    ON stripe_completed_sessions(user_id, created_at DESC);

CREATE TABLE IF NOT EXISTS emotes (
    id                 UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    token              VARCHAR(32) UNIQUE NOT NULL,
    name               VARCHAR(64) NOT NULL,
    image_url          TEXT NOT NULL,
    price_sparks       BIGINT NOT NULL DEFAULT 0,
    is_active          BOOLEAN NOT NULL DEFAULT TRUE,
    created_by_user_id UUID REFERENCES users(id),
    created_at         TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS user_emotes (
    user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    emote_id     UUID NOT NULL REFERENCES emotes(id) ON DELETE CASCADE,
    purchased_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, emote_id)
);
CREATE INDEX IF NOT EXISTS idx_user_emotes_user
    ON user_emotes(user_id, purchased_at DESC);

INSERT INTO emotes (token, name, image_url, price_sparks, is_active)
VALUES
    (':wave:', 'Wave', '/emotes/wave.png', 0, TRUE),
    (':spark:', 'Spark', '/emotes/spark.png', 0, TRUE)
ON CONFLICT (token) DO NOTHING;
