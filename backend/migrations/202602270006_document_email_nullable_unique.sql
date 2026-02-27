-- email is intentionally nullable + UNIQUE:
-- - NULL: OAuth-only users who did not provide an email
-- - Non-NULL: must be unique across all users
-- PostgreSQL UNIQUE allows multiple NULLs by default (SQL standard).
-- No schema change needed; this migration documents the design decision.
SELECT 1;
