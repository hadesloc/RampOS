-- ============================================================================
-- SECURITY FIX: Add encrypted webhook_secret column for proper webhook signing
-- Issue: HIGH - Webhook uses hash instead of secret for HMAC signing
-- ============================================================================

-- Add webhook_secret column for storing encrypted webhook secrets
-- The webhook_secret_hash is kept for backward compatibility (verification)
-- but webhook_secret stores the actual secret (encrypted) for signing
ALTER TABLE tenants ADD COLUMN IF NOT EXISTS webhook_secret_encrypted BYTEA;

-- Comment for documentation
COMMENT ON COLUMN tenants.webhook_secret_encrypted
    IS 'Encrypted webhook secret used for HMAC signing. Encrypted using application-level encryption.';
COMMENT ON COLUMN tenants.webhook_secret_hash
    IS 'Hash of webhook secret for verification only. DO NOT use for HMAC signing.';
