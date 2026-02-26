-- Drop indexes that are redundant with existing UNIQUE column constraints.
-- PostgreSQL automatically creates a unique index for every UNIQUE constraint,
-- so manually-created indexes on those same columns are unnecessary duplicates.

-- users.username already has a UNIQUE constraint (init migration line 5)
DROP INDEX IF EXISTS idx_users_username;

-- users.email already has a UNIQUE constraint (init migration line 6)
DROP INDEX IF EXISTS idx_users_email;

-- categories.slug already has a UNIQUE constraint (init migration line 25)
DROP INDEX IF EXISTS idx_categories_slug;
