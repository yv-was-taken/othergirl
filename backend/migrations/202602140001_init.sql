CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE IF NOT EXISTS users (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username         VARCHAR(32) UNIQUE NOT NULL,
    email            VARCHAR(255) UNIQUE,
    password_hash    VARCHAR(255),
    bio              TEXT NOT NULL DEFAULT '',
    is_premium       BOOLEAN NOT NULL DEFAULT FALSE,
    is_age_verified  BOOLEAN NOT NULL DEFAULT FALSE,
    is_suspended     BOOLEAN NOT NULL DEFAULT FALSE,
    reputation_score DOUBLE PRECISION NOT NULL DEFAULT 1.0,
    sparks_balance   BIGINT NOT NULL DEFAULT 0,
    keep_count       INT NOT NULL DEFAULT 0,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);

CREATE TABLE IF NOT EXISTS categories (
    id                   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name                 VARCHAR(64) NOT NULL,
    slug                 VARCHAR(64) UNIQUE NOT NULL,
    description          TEXT NOT NULL DEFAULT '',
    parent_id            UUID REFERENCES categories(id) ON DELETE CASCADE,
    is_nsfw              BOOLEAN NOT NULL DEFAULT FALSE,
    is_admin_created     BOOLEAN NOT NULL DEFAULT TRUE,
    created_by_user_id   UUID REFERENCES users(id),
    active_user_count    INT NOT NULL DEFAULT 0,
    visibility_threshold INT NOT NULL DEFAULT 5,
    is_visible           BOOLEAN NOT NULL DEFAULT TRUE,
    sort_order           INT NOT NULL DEFAULT 0,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_categories_parent ON categories(parent_id);
CREATE INDEX IF NOT EXISTS idx_categories_slug ON categories(slug);

CREATE TABLE IF NOT EXISTS languages (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code       VARCHAR(8) UNIQUE NOT NULL,
    name       VARCHAR(64) NOT NULL,
    is_active  BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE IF NOT EXISTS chats (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_a_id    UUID NOT NULL REFERENCES users(id),
    user_b_id    UUID NOT NULL REFERENCES users(id),
    category_id  UUID REFERENCES categories(id),
    language_id  UUID REFERENCES languages(id),
    started_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ended_at     TIMESTAMPTZ,
    is_kept_by_a BOOLEAN NOT NULL DEFAULT FALSE,
    is_kept_by_b BOOLEAN NOT NULL DEFAULT FALSE,
    is_kept      BOOLEAN GENERATED ALWAYS AS (is_kept_by_a AND is_kept_by_b) STORED
);

CREATE INDEX IF NOT EXISTS idx_chats_user_a ON chats(user_a_id, started_at DESC);
CREATE INDEX IF NOT EXISTS idx_chats_user_b ON chats(user_b_id, started_at DESC);
CREATE INDEX IF NOT EXISTS idx_chats_kept ON chats(is_kept) WHERE is_kept = TRUE;

CREATE TABLE IF NOT EXISTS messages (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    chat_id      UUID NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
    sender_id    UUID NOT NULL REFERENCES users(id),
    content_text TEXT NOT NULL,
    is_read      BOOLEAN NOT NULL DEFAULT FALSE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_messages_chat ON messages(chat_id, created_at);
