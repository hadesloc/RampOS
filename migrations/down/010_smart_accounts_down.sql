-- Down migration for 010_smart_accounts.sql
-- Drops smart accounts table and related objects

-- Drop RLS policy
DROP POLICY IF EXISTS smart_accounts_tenant_isolation ON smart_accounts;

-- Drop trigger
DROP TRIGGER IF EXISTS trigger_smart_accounts_updated_at ON smart_accounts;

-- Drop indexes
DROP INDEX IF EXISTS idx_smart_accounts_chain;
DROP INDEX IF EXISTS idx_smart_accounts_owner;
DROP INDEX IF EXISTS idx_smart_accounts_tenant_address;
DROP INDEX IF EXISTS idx_smart_accounts_tenant_user;
DROP INDEX IF EXISTS idx_smart_accounts_address_chain;

-- Drop table
DROP TABLE IF EXISTS smart_accounts;
