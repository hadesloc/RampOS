-- Down migration for 011_add_api_secret.sql
-- Removes api_secret_encrypted column from tenants

ALTER TABLE tenants DROP COLUMN IF EXISTS api_secret_encrypted;
