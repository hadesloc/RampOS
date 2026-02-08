-- Down migration for 024_refresh_tokens.sql
-- Drops refresh tokens table

DROP INDEX IF EXISTS idx_refresh_tokens_expires;
DROP INDEX IF EXISTS idx_refresh_tokens_family_id;
DROP INDEX IF EXISTS idx_refresh_tokens_user_id;
DROP INDEX IF EXISTS idx_refresh_tokens_hash;
DROP TABLE IF EXISTS refresh_tokens;
