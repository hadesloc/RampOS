-- Down migration for 019_usage_billing.sql
-- Drops usage billing tables and related objects

-- Drop RLS policies
DROP POLICY IF EXISTS tenant_isolation_invoices ON invoices;
DROP POLICY IF EXISTS tenant_isolation_daily_usage ON daily_usage;
DROP POLICY IF EXISTS tenant_isolation_usage_events ON usage_events;

-- Drop indexes
DROP INDEX IF EXISTS idx_pricing_plans_name;
DROP INDEX IF EXISTS idx_invoices_tenant_period;
DROP INDEX IF EXISTS idx_usage_events_tenant_meter_time;
DROP INDEX IF EXISTS idx_usage_events_tenant_time;

-- Drop tables (in dependency order)
DROP TABLE IF EXISTS invoices;
DROP TABLE IF EXISTS daily_usage;
DROP TABLE IF EXISTS usage_events;
DROP TABLE IF EXISTS billing_meters;
DROP TABLE IF EXISTS pricing_plans;

-- Drop enum types
DROP TYPE IF EXISTS invoice_status;
DROP TYPE IF EXISTS aggregation_type;
DROP TYPE IF EXISTS meter_type;
DROP TYPE IF EXISTS billing_period;
