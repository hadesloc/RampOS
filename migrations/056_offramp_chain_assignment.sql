-- Migration 056: Persist optional chain assignment for off-ramp deposit targets
-- This keeps off-ramp watch scope additive so future detect jobs know which chain
-- a CRYPTO_PENDING deposit address belongs to.

ALTER TABLE offramp_intents
    ADD COLUMN IF NOT EXISTS chain_id BIGINT;

CREATE INDEX IF NOT EXISTS idx_offramp_intents_tenant_state_chain
    ON offramp_intents(tenant_id, state, chain_id)
    WHERE chain_id IS NOT NULL;
