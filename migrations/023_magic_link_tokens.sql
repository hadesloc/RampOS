-- Migration: Magic Link Tokens
-- Description: Secure magic link token storage for passwordless authentication

CREATE TABLE IF NOT EXISTS magic_link_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    token_hash BYTEA NOT NULL,
    email VARCHAR(255) NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    used BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index on token_hash for fast lookup during verification
CREATE UNIQUE INDEX IF NOT EXISTS idx_magic_link_tokens_hash ON magic_link_tokens(token_hash);

-- Index on email + created_at for rate limiting queries
CREATE INDEX IF NOT EXISTS idx_magic_link_tokens_email_created ON magic_link_tokens(email, created_at);

-- Index on expires_at for cleanup of expired tokens
CREATE INDEX IF NOT EXISTS idx_magic_link_tokens_expires ON magic_link_tokens(expires_at);
