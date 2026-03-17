-- Migration 050: Passkey credentials — PostgreSQL-backed storage
-- Replaces in-memory HashMap storage for passkey credentials

CREATE TABLE IF NOT EXISTS passkey_credentials (
    credential_id   TEXT PRIMARY KEY,
    user_id         TEXT NOT NULL,
    public_key_x    TEXT NOT NULL,
    public_key_y    TEXT NOT NULL,
    smart_account_address TEXT,
    display_name    TEXT NOT NULL DEFAULT '',
    is_active       BOOLEAN NOT NULL DEFAULT true,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at    TIMESTAMPTZ
);

-- Fast lookup by user_id (most common query pattern)
CREATE INDEX idx_passkey_credentials_user_id ON passkey_credentials(user_id);

-- Fast lookup for active credentials per user
CREATE INDEX idx_passkey_credentials_user_active ON passkey_credentials(user_id, is_active)
    WHERE is_active = true;
