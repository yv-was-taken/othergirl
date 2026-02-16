# Repository Guidelines

## Project Structure & Module Organization
This is a monorepo with three main areas:
- `backend/`: Rust + Axum API, WebSocket server, and SQLx migrations (`backend/migrations/`).
- `frontend/`: SvelteKit app (`frontend/src/routes` for pages, `frontend/src/lib` for components/stores/utils, `frontend/static` for assets/emotes).
- `deploy/`: production scripts, systemd units, Cloudflare tunnel docs, and environment templates.

## Build, Test, and Development Commands
- `./dev.sh`: starts local PostgreSQL + Redis containers, then runs backend and frontend together.
- `cd backend && cargo run`: run backend locally.
- `cd backend && cargo check`: fast Rust compile/type check.
- `cd backend && cargo test`: run backend tests.
- `cd frontend && bun install`: install frontend dependencies.
- `cd frontend && bun run dev`: run frontend locally (Vite).
- `cd frontend && bun run check`: run Svelte/TypeScript checks.
- `cd frontend && bun run build`: production frontend build validation.

## Coding Style & Naming Conventions
- Rust: follow `rustfmt` defaults and keep modules domain-focused (`auth`, `chat`, `payments`, etc.).
- Rust naming: `snake_case` for functions/files, `PascalCase` for types, `SCREAMING_SNAKE_CASE` for constants.
- Svelte/TS: 2-space indentation; route files follow SvelteKit conventions (`+page.svelte`, `+layout.svelte`).
- Components use `PascalCase` filenames in `frontend/src/lib/components` (example: `ChatWindow.svelte`).

## Testing Guidelines
- Prefer unit tests close to code (`#[cfg(test)]` modules in Rust files).
- For backend changes, run at least `cargo check`; for behavior changes, run `cargo test`.
- For frontend changes, run `bun run check`; run `bun run build` before merging UI-impacting work.

## Commit & Pull Request Guidelines
- Follow history style: short imperative subjects (e.g., `Add ...`, `Fix ...`, `Switch ...`).
- Keep each commit scoped to one logical change.
- PRs should include scope/why, affected paths, verification commands run, UI screenshots when relevant, and migration/env rollout notes.

## Security & Configuration Tips
- Copy from env templates (`backend/.env.example`, `deploy/env/*.example`); never commit real secrets.
- A proxy "in front of the app" is any layer that receives internet traffic first, then forwards to backend.
- Current production path is `Client -> Cloudflare -> cloudflared -> backend` (see `deploy/cloudflare/README.md` and `deploy/systemd/othergirl-cloudflared.service`).
- Production backend binds localhost (`deploy/env/backend.production.env.example`), so proxy/tunnel forwarding is expected.
- Set `TRUSTED_PROXY_HOPS` per environment topology (dev/staging/prod), not per replica.
- For this repo's tunnel production path, use `TRUSTED_PROXY_HOPS=1` (template default); `0` collapses users into one rate-limit bucket.
- Typical values elsewhere: local direct dev (`client -> backend`) `=0`; two trusted proxy layers may require `=2`.
