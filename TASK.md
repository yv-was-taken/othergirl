# M18: No Redis Connection Validation at Startup

## Severity: MEDIUM

## Location
`backend/src/redis_client.rs:1-5`

## Description
`Client::open()` only parses the URL -- it does NOT actually connect to Redis. If Redis is down at startup, the app will start successfully but fail on the first Redis call. This is a silent-failure footgun.

## Suggested Fix
After `Client::open()`, do a `client.get_connection()` or `client.get_multiplexed_tokio_connection().await` to verify connectivity, or at least log a warning.

## Branch
`fix/m18-redis-no-startup-validation`
