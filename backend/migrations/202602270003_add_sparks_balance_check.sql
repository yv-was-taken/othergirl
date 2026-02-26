-- Prevent sparks_balance from ever going negative at the database level.
ALTER TABLE users ADD CONSTRAINT check_sparks_balance_non_negative CHECK (sparks_balance >= 0);
