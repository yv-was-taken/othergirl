# Deferred Items

## Security

- [x] **Permissive CORS**: Restricted to specific origins (panics on wildcard) — fixed in h03
- [x] **Stripe webhook verification**: Uses `hmac` crate's `verify_slice()` which delegates to `subtle::ConstantTimeEq` — already timing-safe
- [x] **Security headers**: Added CSP, X-Frame-Options, X-Content-Type-Options, HSTS via `tower-http` SetResponseHeaderLayer

## Testing

- [ ] **Unit tests**: Auth password hashing, JWT issue/verify, ledger transactions, encryption round-trip
- [ ] **Integration tests**: API route handlers (register, login, matchmaking, payments, awards)
- [ ] **E2E tests**: Playwright or similar — login flow, matchmaking → chat → keep vote → history
