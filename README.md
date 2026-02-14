# Othergirl Monorepo

Othergirl is a privacy-first random 1:1 text chat platform with category/language matchmaking, keeps, moderation, sparks economy, flare cosmetics, and custom emotes.

## Stack

- Frontend: SvelteKit + Tailwind + Bun
- Backend: Rust + Axum
- Database: PostgreSQL + SQLx migrations
- Queue/cache: Redis
- Payments: Stripe (subscriptions, sparks checkout, connect cashout)

## Repository Layout

- `backend/`: API, WebSocket server, SQLx migrations
- `frontend/`: SvelteKit app
- `deploy/`: backend systemd, nginx API config, cloudflare pages docs, backup/deploy scripts

## Implemented Features

- Auth: email/password + OAuth (`google`, `discord`, `github`, `telegram`)
- OAuth security: Redis-backed state validation and provider callback verification
- Matchmaking: category/language queueing with Redis ZSETs, background matcher, cooldown, block/reputation filtering
- WebSocket: token in query or first message, queued/matched/message/typing/read/keep/award/leave events
- Chat safety: keyword/flood/url signal scanning with flagged-message persistence
- Encryption at rest: AES-256-GCM message encryption, per-chat DEKs wrapped by KEK before DB storage
- History: chat list, kept chats, decrypted transcript retrieval
- Moderation: reports, blocks, reputation recalculation, auto-suspension threshold
- Payments: Stripe checkout for premium + sparks, Stripe webhook processing, Stripe Connect onboarding + transfer cashout
- Economy: sparks ledger + transactions, flare store purchase/equip, awards with 70/30 split
- Emotes: emote catalog, purchase flow, admin upload endpoint, frontend emote rendering
- Health endpoint: `/health`

## Prerequisites

- Rust stable (`cargo`, `rustc`)
- PostgreSQL 16+
- Redis 7+
- Bun 1.x

## Local Run

1. Backend

```bash
cd backend
cp .env.example .env
cargo run
```

2. Frontend

```bash
cd frontend
cp .env.example .env
bun install
bun run dev
```

Default URLs:

- Frontend: `http://localhost:5173`
- Backend: `http://localhost:8080`

## Checks

Backend:

```bash
cd backend
cargo check
```

Frontend:

```bash
cd frontend
bun run check
bun run build
```

## Key API Surface

- Auth: `/api/auth/*`
- Users + flare: `/api/users/me`, `/api/users/me/flare`
- Categories/languages: `/api/categories`, `/api/languages`
- Matchmaking: `/api/matchmaking/*`
- Chat WS: `/api/chat`
- History: `/api/chats`, `/api/chats/keeps`, `/api/chats/:id`
- Moderation: `/api/reports`, `/api/blocks`
- Payments/sparks: `/api/payments/*`, `/api/sparks/*`
- Cashout: `/api/cashout/*`
- Flare store: `/api/store/*`
- Emotes: `/api/emotes/*`
- Awards: `/api/awards/*`
- Health: `/health`

## Deployment

- Backend runs on Hetzner VPS (`deploy/systemd/othergirl-backend.service`, `deploy/nginx/othergirl.conf`, `deploy/scripts/deploy.sh`)
- Frontend runs on Cloudflare Pages (`deploy/cloudflare/README.md`)
- PostgreSQL backup helper: `deploy/scripts/backup_postgres.sh`
