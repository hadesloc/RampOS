-- Add API version pinning column to tenants table
-- Allows tenants to pin their API integration to a specific version
ALTER TABLE tenants ADD COLUMN IF NOT EXISTS api_version VARCHAR(20) DEFAULT '2026-02-01';
