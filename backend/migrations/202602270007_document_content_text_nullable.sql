-- messages.content_text was originally NOT NULL (migration 001).
-- Migration 003 dropped NOT NULL to support encrypted messages where
-- content_text is NULL and content_encrypted holds the ciphertext.
-- Application code must handle both:
--   content_text IS NOT NULL  →  plaintext message
--   content_text IS NULL      →  check content_encrypted for decryption
-- No schema change needed; this migration documents the transition.
SELECT 1;
