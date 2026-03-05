# Deferred Items

## Security

- [x] **Permissive CORS**: Restricted to specific origins (panics on wildcard) — fixed in h03
- [x] **Stripe webhook verification**: Uses `hmac` crate's `verify_slice()` which delegates to `subtle::ConstantTimeEq` — already timing-safe
- [x] **Security headers**: Added CSP, X-Frame-Options, X-Content-Type-Options, HSTS via `tower-http` SetResponseHeaderLayer

## Testing

- [x] **Unit tests**: 95 tests across 9 modules (auth, jwt, encryption, safety, stripe, config, error, users, reputation)
- [x] **Integration tests**: 14 tests against real Postgres/Redis (auth, users, categories, health) via docker-compose.test.yml
- [x] **E2E tests**: 15 Playwright tests (auth flow, navigation, store, theme) with backend+frontend webServer config
