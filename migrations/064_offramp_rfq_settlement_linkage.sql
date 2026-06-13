-- Link OFFRAMP RFQ matches to settlement execution.

ALTER TABLE offramp_intents
    ADD COLUMN IF NOT EXISTS linked_rfq_id TEXT NULL REFERENCES rfq_requests(id),
    ADD COLUMN IF NOT EXISTS winning_lp_id TEXT NULL,
    ADD COLUMN IF NOT EXISTS matched_rate NUMERIC NULL,
    ADD COLUMN IF NOT EXISTS settlement_id TEXT NULL REFERENCES settlements(id);

ALTER TABLE settlements
    ADD COLUMN IF NOT EXISTS tenant_id TEXT NULL REFERENCES tenants(id),
    ADD COLUMN IF NOT EXISTS rfq_id TEXT NULL REFERENCES rfq_requests(id),
    ADD COLUMN IF NOT EXISTS lp_id TEXT NULL,
    ADD COLUMN IF NOT EXISTS final_rate NUMERIC NULL;

UPDATE settlements s
SET tenant_id = oi.tenant_id
FROM offramp_intents oi
WHERE s.offramp_intent_id = oi.id
  AND s.tenant_id IS NULL;

CREATE INDEX IF NOT EXISTS idx_offramp_intents_linked_rfq
    ON offramp_intents(tenant_id, linked_rfq_id)
    WHERE linked_rfq_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_offramp_intents_settlement
    ON offramp_intents(tenant_id, settlement_id)
    WHERE settlement_id IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_settlements_tenant_rfq_unique
    ON settlements(tenant_id, rfq_id)
    WHERE rfq_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_settlements_tenant_rfq
    ON settlements(tenant_id, rfq_id)
    WHERE rfq_id IS NOT NULL;
