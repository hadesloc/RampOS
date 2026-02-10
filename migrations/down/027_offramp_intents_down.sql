-- Down migration 027: Remove off-ramp intents table
DROP POLICY IF EXISTS offramp_intents_tenant_isolation ON offramp_intents;
DROP TABLE IF EXISTS offramp_intents;
