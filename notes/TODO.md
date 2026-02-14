# Deferred Items

## Security

- [ ] **Permissive CORS**: Backend currently allows all origins — restrict to production domain(s)
- [ ] **Stripe webhook verification**: Verify `stripe-signature` using constant-time comparison (currently uses `hmac` crate — audit for timing safety)
- [ ] **Security headers**: Add CSP, X-Frame-Options, HSTS via `tower-http` or reverse proxy config

## Testing

- [ ] **Unit tests**: Auth password hashing, JWT issue/verify, ledger transactions, encryption round-trip
- [ ] **Integration tests**: API route handlers (register, login, matchmaking, payments, awards)
- [ ] **E2E tests**: Playwright or similar — login flow, matchmaking → chat → keep vote → history
