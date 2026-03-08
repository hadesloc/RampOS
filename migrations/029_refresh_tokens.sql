-- Migration: Refresh Tokens
-- Description: Secure refresh token storage with rotation and family tracking
-- Replaces stateless JWT refresh tokens with opaque tokens stored in DB

CREATE TABLE IF NOT EXISTS refresh_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    token_hash BYTEA NOT NULL,
    user_id VARCHAR(64) NOT NULL,
    device_info VARCHAR(512),
    expires_at TIMESTAMPTZ NOT NULL,
    family_id UUID NOT NULL,
    revoked BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Unique index on token_hash for fast lookup during refresh
CREATE UNIQUE INDEX IF NOT EXISTS idx_refresh_tokens_hash ON refresh_tokens(token_hash);

-- Index on user_id for listing/revoking user's tokens
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_user_id ON refresh_tokens(user_id);

-- Index on family_id for token family invalidation (theft detection)
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_family_id ON refresh_tokens(family_id);

-- Index on expires_at for cleanup of expired tokens
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_expires ON refresh_tokens(expires_at);
