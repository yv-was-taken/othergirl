# Cashout System: Known Issues

Remaining findings from code review of the cashout race condition fix (PR #9). None are exploitable security vulnerabilities.

## High

- **No backoff on reconciler**: Hammers Stripe every 60s even when rate-limited (429). Should implement exponential backoff or stop the batch when rate-limited.
- **Stale threshold too aggressive**: `CASHOUT_RECONCILE_STALE_SECS` is 120s. Reconciler may retry while the original Stripe call is still in-flight. Consider increasing to 5-10 minutes.
- **No rate limiting on cashout endpoint**: A malicious user can rapidly submit cashout requests that fail validation. Should have its own stricter rate limit.

## Medium

- **Unbounded SELECT in manual reconciliation**: `mark_old_pending_cashouts_for_manual_reconciliation` has no `LIMIT`. Should use `CASHOUT_RECONCILE_BATCH_SIZE`.
- **Multiple reconciler instances compete**: Each app replica spawns its own reconciler. Optimistic locking prevents double-processing but is wasteful. Consider `pg_try_advisory_lock`.
- **No `updated_at` trigger**: Application code manually sets `updated_at = NOW()` on every UPDATE. A forgotten update would silently break optimistic locking in `claim_pending_cashout_for_reconciliation`. Consider a `BEFORE UPDATE` trigger.
- **`AppError::Conflict` overloaded**: Used for config errors, unique violations, idempotency errors, and race condition results. Consider splitting into more specific variants.

## Low

- **Formatting-only changes in PR diff**: 5 unrelated files (`oauth.rs`, `websocket.rs`, `config.rs`, `emotes/handlers.rs`, `queue.rs`) have only rustfmt changes. Could be separated into their own commit.
- **Status as string constants**: `CASHOUT_STATUS_PENDING` etc. should be a `CashoutStatus` enum with `Display`/`FromStr` for compile-time safety.
- **No graceful shutdown on reconciler**: `tokio::spawn` + infinite loop with no `CancellationToken`. Cannot be stopped during server shutdown.
- **`reqwest::Client::new()` per Stripe request**: Pre-existing issue. Should reuse a single `Client` instance for connection pooling.
