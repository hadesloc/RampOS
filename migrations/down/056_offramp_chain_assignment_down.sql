DROP INDEX IF EXISTS idx_offramp_intents_tenant_state_chain;

ALTER TABLE offramp_intents
    DROP COLUMN IF EXISTS chain_id;
