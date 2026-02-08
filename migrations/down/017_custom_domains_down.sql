-- Down migration for 017_custom_domains.sql
-- Drops custom domains table and related objects

-- Drop RLS policies
DROP POLICY IF EXISTS tenant_isolation_custom_domains ON custom_domains;

-- Drop indexes
DROP INDEX IF EXISTS idx_custom_domains_status;
DROP INDEX IF EXISTS idx_custom_domains_tenant_id;

-- Drop table
DROP TABLE IF EXISTS custom_domains;

-- Drop enum type
DROP TYPE IF EXISTS domain_status;
