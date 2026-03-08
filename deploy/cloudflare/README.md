# Cloudflare Pages + Tunnel Deployment

Frontend is on Cloudflare Pages. Backend stays on VPS and is exposed through Cloudflare Tunnel (no nginx/certbot required).

## Architecture

- Frontend: `https://<pages-domain>` (Cloudflare Pages)
- Backend origin on VPS: `http://127.0.0.1:8080` (Axum only listens locally/public per env)
- Public API hostname: `https://api.othergirl.lol` (Cloudflare Tunnel route to VPS)

## Cloudflare Tunnel Setup (Dashboard)

1. Open Cloudflare Zero Trust.
2. Go to `Networks -> Tunnels`.
3. Create tunnel (`Cloudflared` connector type).
4. Add public hostname:
   - Hostname: `api.othergirl.lol`
   - Service: `http://localhost:8080`
5. Copy the connector token.

Set token on VPS:

```bash
sudo cp /opt/othergirl/deploy/env/cloudflared.env.example /etc/othergirl/cloudflared.env
sudo nano /etc/othergirl/cloudflared.env
# set CLOUDFLARED_TUNNEL_TOKEN=...
```

Deploy/restart services:

```bash
sudo bash /opt/othergirl/deploy/scripts/deploy.sh
```

## Cloudflare Pages Build Settings

- Framework preset: `SvelteKit`
- Build command: `bun run build`
- Output directory: use Cloudflare Pages default for the SvelteKit preset

## Required Pages Environment Variables

- `PUBLIC_API_BASE_URL=https://api.othergirl.lol`

## Backend `.env` Values That Must Match Pages/Tunnel

- `CORS_ORIGIN=https://<pages-domain>`
- `PUBLIC_API_BASE_URL=https://api.othergirl.lol`
- `PUBLIC_WEB_BASE_URL=https://<pages-domain>`
- `STRIPE_SUCCESS_URL=https://<pages-domain>/settings`
- `STRIPE_CANCEL_URL=https://<pages-domain>/settings`
- `STRIPE_CONNECT_REFRESH_URL=https://<pages-domain>/settings`
- `STRIPE_CONNECT_RETURN_URL=https://<pages-domain>/settings`

## OAuth Callback URLs

Configure providers to call back to:

- `https://api.othergirl.lol/api/auth/oauth/google/callback`
- `https://api.othergirl.lol/api/auth/oauth/discord/callback`
- `https://api.othergirl.lol/api/auth/oauth/github/callback`
- `https://api.othergirl.lol/api/auth/oauth/telegram/callback`

## Operational Checks

```bash
sudo systemctl status othergirl-backend --no-pager
sudo systemctl status othergirl-cloudflared --no-pager
curl -fsS http://127.0.0.1:8080/health
```
