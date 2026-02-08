-- Down migration for 023_magic_link_tokens.sql
-- Drops magic link tokens table

DROP INDEX IF EXISTS idx_magic_link_tokens_expires;
DROP INDEX IF EXISTS idx_magic_link_tokens_email_created;
DROP INDEX IF EXISTS idx_magic_link_tokens_hash;
DROP TABLE IF EXISTS magic_link_tokens;
