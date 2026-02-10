-- Tenant-specific rate limit overrides
-- Allows per-tenant, per-route-group rate limit configuration
CREATE TABLE IF NOT EXISTS tenant_rate_limits (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id),
    route_group VARCHAR(50) NOT NULL DEFAULT 'default',
    requests_per_minute INT NOT NULL DEFAULT 600,
    burst_limit INT NOT NULL DEFAULT 100,
    daily_quota INT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(tenant_id, route_group)
);
CREATE INDEX idx_tenant_rate_limits_tenant ON tenant_rate_limits(tenant_id);
