-- ============================================================================
-- SECURITY: Add encrypted api_secret column for HMAC signature verification
-- Issue: SDK requests are signed with api_secret but backend cannot verify
-- ============================================================================

-- Add api_secret_encrypted column for storing encrypted API secrets
-- The api_key_hash is used for lookup (Bearer token)
-- The api_secret_encrypted is used for HMAC signature verification
ALTER TABLE tenants ADD COLUMN IF NOT EXISTS api_secret_encrypted BYTEA;

-- Comment for documentation
COMMENT ON COLUMN tenants.api_secret_encrypted
    IS 'Encrypted API secret used for HMAC signature verification. Encrypted using application-level encryption.';
COMMENT ON COLUMN tenants.api_key_hash
    IS 'Hash of API key (Bearer token) for tenant lookup.';
