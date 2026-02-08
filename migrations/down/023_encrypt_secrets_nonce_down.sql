-- Down migration for 023_encrypt_secrets_nonce.sql
-- Removes nonce columns from tenants table

ALTER TABLE tenants DROP COLUMN IF EXISTS webhook_secret_nonce;
ALTER TABLE tenants DROP COLUMN IF EXISTS api_secret_nonce;
