# Cashout System: Known Issues

Remaining findings from code review of the cashout race condition fix (PR #9). All items resolved.

## High

- ~~**No backoff on reconciler**: Hammers Stripe every 60s even when rate-limited (429). Should implement exponential backoff or stop the batch when rate-limited.~~ **Fixed**: Reconciler now uses exponential backoff (up to 15min) on 429s and stops the batch immediately.
- ~~**Stale threshold too aggressive**: `CASHOUT_RECONCILE_STALE_SECS` is 120s. Reconciler may retry while the original Stripe call is still in-flight. Consider increasing to 5-10 minutes.~~ **Fixed**: Increased to 300s (5 minutes).
- ~~**No rate limiting on cashout endpoint**: A malicious user can rapidly submit cashout requests that fail validation. Should have its own stricter rate limit.~~ **Fixed**: Added `cashout` rate-limit bucket (5 requests/60s) in `rate_limit.rs`.

## Medium

- ~~**Unbounded SELECT in manual reconciliation**: `mark_old_pending_cashouts_for_manual_reconciliation` has no `LIMIT`. Should use `CASHOUT_RECONCILE_BATCH_SIZE`.~~ **Fixed**: Added `LIMIT $3` bound to `CASHOUT_RECONCILE_BATCH_SIZE`.
- ~~**Multiple reconciler instances compete**: Each app replica spawns its own reconciler. Optimistic locking prevents double-processing but is wasteful. Consider `pg_try_advisory_lock`.~~ **Fixed**: Reconciler acquires `pg_try_advisory_lock` before each pass and releases it after.
- ~~**No `updated_at` trigger**: Application code manually sets `updated_at = NOW()` on every UPDATE. A forgotten update would silently break optimistic locking in `claim_pending_cashout_for_reconciliation`. Consider a `BEFORE UPDATE` trigger.~~ **Fixed**: Added `BEFORE UPDATE` trigger in migration `202602160002`.
- ~~**`AppError::Conflict` overloaded**: Used for config errors, unique violations, idempotency errors, and race condition results. Consider splitting into more specific variants.~~ **Fixed**: Added `AppError::ServiceUnavailable` (503) for config/service errors. `Conflict` now only used for true 409 business-logic conflicts.

## Low

- **Formatting-only changes in PR diff**: 5 unrelated files (`oauth.rs`, `websocket.rs`, `config.rs`, `emotes/handlers.rs`, `queue.rs`) have only rustfmt changes. Not actionable without rewriting git history.
- ~~**Status as string constants**: `CASHOUT_STATUS_PENDING` etc. should be a `CashoutStatus` enum with `Display`/`FromStr` for compile-time safety.~~ **Fixed**: Replaced with `CashoutStatus` enum with `as_str()`, `Display`, and `TryFrom<&str>`.
- ~~**No graceful shutdown on reconciler**: `tokio::spawn` + infinite loop with no `CancellationToken`. Cannot be stopped during server shutdown.~~ **Fixed**: Added `CancellationToken` from `tokio-util`, wired through `with_graceful_shutdown` on SIGINT.
- ~~**`reqwest::Client::new()` per Stripe request**: Pre-existing issue. Should reuse a single `Client` instance for connection pooling.~~ **Fixed**: Replaced with module-level `LazyLock<reqwest::Client>` for connection reuse.
