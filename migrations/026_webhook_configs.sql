-- Migration: Webhook Endpoint Configurations
-- Task: C5
-- Description: Add table for webhook endpoint configurations (URL, events, secret, active)
--              Add config_id column to webhook_events for linking deliveries to configs

CREATE TABLE IF NOT EXISTS webhook_configs (
    id VARCHAR(64) PRIMARY KEY,
    tenant_id VARCHAR(64) NOT NULL REFERENCES tenants(id),

    -- Endpoint configuration
    url VARCHAR(1024) NOT NULL,
    events JSONB NOT NULL DEFAULT '[]'::jsonb,
    active BOOLEAN NOT NULL DEFAULT true,

    -- Secret for HMAC signing of webhook payloads
    secret VARCHAR(255) NOT NULL,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_webhook_configs_tenant ON webhook_configs(tenant_id);
CREATE INDEX idx_webhook_configs_active ON webhook_configs(tenant_id, active) WHERE active = true;

-- Trigger for updated_at
CREATE TRIGGER trigger_webhook_configs_updated_at
    BEFORE UPDATE ON webhook_configs
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- RLS
ALTER TABLE webhook_configs ENABLE ROW LEVEL SECURITY;

CREATE POLICY webhook_configs_tenant_isolation ON webhook_configs
    FOR ALL
    USING (tenant_id = current_setting('app.current_tenant', true)::VARCHAR)
    WITH CHECK (tenant_id = current_setting('app.current_tenant', true)::VARCHAR);

COMMENT ON TABLE webhook_configs IS 'Webhook endpoint configurations for tenants';

-- Add config_id to webhook_events to link deliveries to webhook configs
ALTER TABLE webhook_events ADD COLUMN IF NOT EXISTS config_id VARCHAR(64) REFERENCES webhook_configs(id);
CREATE INDEX IF NOT EXISTS idx_webhooks_config ON webhook_events(config_id) WHERE config_id IS NOT NULL;
