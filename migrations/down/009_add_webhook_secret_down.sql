-- Down migration for 009_add_webhook_secret.sql
-- Removes webhook_secret_encrypted column from tenants

ALTER TABLE tenants DROP COLUMN IF EXISTS webhook_secret_encrypted;
