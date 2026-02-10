-- Revert API version pinning column from tenants table
ALTER TABLE tenants DROP COLUMN IF EXISTS api_version;
