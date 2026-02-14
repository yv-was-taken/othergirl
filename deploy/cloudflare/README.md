# Cloudflare Pages Deployment

Frontend is deployed on Cloudflare Pages. Backend remains on Hetzner (`api.othergirl.com`).

## Build Settings

- Framework preset: `SvelteKit`
- Build command: `bun run build`
- Output directory: `.svelte-kit/cloudflare`
- Node compatibility flag: enabled

## Required Pages Environment Variables

- `PUBLIC_API_BASE_URL=https://api.othergirl.com`

## OAuth and Stripe Redirect URLs

Backend `.env` should use Cloudflare Pages origin for all browser-facing redirects:

- `PUBLIC_WEB_BASE_URL=https://othergirl.pages.dev` (or your custom domain)
- `PUBLIC_API_BASE_URL=https://api.othergirl.com`
- `STRIPE_SUCCESS_URL=https://othergirl.pages.dev/settings`
- `STRIPE_CANCEL_URL=https://othergirl.pages.dev/settings`
- `STRIPE_CONNECT_REFRESH_URL=https://othergirl.pages.dev/settings`
- `STRIPE_CONNECT_RETURN_URL=https://othergirl.pages.dev/settings`

## Notes

- Do not run frontend as a systemd service on the VPS.
- Keep backend CORS aligned with your Pages domain(s).
