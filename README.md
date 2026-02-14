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
- `deploy/`: backend+tunnel systemd, cloudflare docs, env templates, backup/deploy scripts

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

- Frontend: `http://localhost:3000`
- Backend: `http://localhost:8080`

### One-command local dev

From repo root:

```bash
./dev.sh
```

`dev.sh` will:

- ensure Docker daemon is running,
- start/create local PostgreSQL and Redis containers,
- create missing `backend/.env` and `frontend/.env` from examples,
- install frontend deps (if needed),
- run backend (`cargo run`) and frontend (`bun run dev`) together.

If `tmux` is installed, it starts them in two panes of the same tmux window by default for split live logging.
Use `./dev.sh --no-tmux` to force single-terminal mode.

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

Backend runs on Hetzner VPS and frontend runs on Cloudflare Pages.
Public backend access is via Cloudflare Tunnel (not nginx).

Deployment assets:

- `deploy/scripts/bootstrap.sh`: one-time host setup + first deploy
- `deploy/scripts/deploy.sh`: idempotent backend deploy (sync/build/systemd/restart)
- `deploy/systemd/othergirl-backend.service`: backend service unit
- `deploy/systemd/othergirl-cloudflared.service`: cloudflared tunnel service unit
- `deploy/scripts/backup_postgres.sh`: postgres backup job
- `deploy/env/backend.production.env.example`: production backend env template
- `deploy/env/cloudflared.env.example`: tunnel token env template
- `deploy/env/backup.env.example`: backup cron env template
- `deploy/cloudflare/README.md`: Cloudflare Pages + Tunnel setup

### One-Time Server Bootstrap (copy/paste)

Run on a fresh Ubuntu VPS:

```bash
sudo apt-get update && sudo apt-get install -y git
git clone https://github.com/yv-was-taken/othergirl.git
cd othergirl
sudo mkdir -p /opt/othergirl/backend /etc/othergirl
sudo cp deploy/env/backend.production.env.example /opt/othergirl/backend/.env
sudo cp deploy/env/cloudflared.env.example /etc/othergirl/cloudflared.env
sudo nano /opt/othergirl/backend/.env
sudo nano /etc/othergirl/cloudflared.env
sudo bash deploy/scripts/bootstrap.sh
```

`/etc/othergirl/cloudflared.env` must contain:

```bash
CLOUDFLARED_TUNNEL_TOKEN=<token from Cloudflare Zero Trust tunnel>
```

### Routine Backend Deploy (after code changes)

From the checked-out repo on the server:

```bash
sudo bash deploy/scripts/deploy.sh
```

### Post-Deploy Checks

```bash
sudo systemctl status othergirl-backend --no-pager
sudo systemctl status othergirl-cloudflared --no-pager
curl -fsS http://127.0.0.1:8080/health
```
