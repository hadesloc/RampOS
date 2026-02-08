-- ============================================================================
-- SECURITY: Add nonce columns for AES-256-GCM encryption of secrets
-- These nonces are required for proper authenticated encryption.
-- Each encrypted value has its own random 96-bit nonce.
-- ============================================================================

-- Nonce for api_secret_encrypted (AES-256-GCM, 12 bytes)
ALTER TABLE tenants ADD COLUMN IF NOT EXISTS api_secret_nonce BYTEA;

-- Nonce for webhook_secret_encrypted (AES-256-GCM, 12 bytes)
ALTER TABLE tenants ADD COLUMN IF NOT EXISTS webhook_secret_nonce BYTEA;

COMMENT ON COLUMN tenants.api_secret_nonce
    IS 'AES-256-GCM nonce (12 bytes) used to encrypt api_secret_encrypted.';
COMMENT ON COLUMN tenants.webhook_secret_nonce
    IS 'AES-256-GCM nonce (12 bytes) used to encrypt webhook_secret_encrypted.';
