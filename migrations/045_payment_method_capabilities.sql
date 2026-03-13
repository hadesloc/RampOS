CREATE TABLE IF NOT EXISTS payment_method_capabilities (
    id TEXT PRIMARY KEY,
    corridor_pack_id TEXT NOT NULL REFERENCES corridor_packs(id) ON DELETE CASCADE,
    partner_capability_id TEXT NULL REFERENCES partner_capabilities(id) ON DELETE SET NULL,
    method_family TEXT NOT NULL,
    funding_source TEXT NULL,
    settlement_direction TEXT NOT NULL,
    presentment_model TEXT NULL,
    card_funding_enabled BOOLEAN NOT NULL DEFAULT false,
    policy_flags JSONB NOT NULL DEFAULT '{}'::jsonb,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_payment_method_capabilities_corridor
    ON payment_method_capabilities (corridor_pack_id, method_family, settlement_direction);

CREATE INDEX IF NOT EXISTS idx_payment_method_capabilities_partner_capability
    ON payment_method_capabilities (partner_capability_id, method_family)
    WHERE partner_capability_id IS NOT NULL;

CREATE TRIGGER trigger_payment_method_capabilities_updated_at
    BEFORE UPDATE ON payment_method_capabilities
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();
