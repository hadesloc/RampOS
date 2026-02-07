-- ============================================================================
-- Migration 015: Compliance Audit Trail
-- Immutable audit log with hash chain for regulatory inspections
-- ============================================================================

-- Create enum for compliance event types
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'compliance_event_type') THEN
        CREATE TYPE compliance_event_type AS ENUM (
            'compliance_decision',
            'document_submitted',
            'rule_changed',
            'user_action',
            'kyc_tier_change',
            'transaction_approval',
            'transaction_rejection',
            'aml_rule_modification',
            'sar_submission',
            'ctr_submission',
            'license_status_change',
            'sanctions_check',
            'pep_check'
        );
    END IF;
END $$;

-- Create compliance audit log table (append-only with hash chain)
CREATE TABLE IF NOT EXISTS compliance_audit_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id VARCHAR(64) NOT NULL REFERENCES tenants(id),

    -- Event classification
    event_type compliance_event_type NOT NULL,

    -- Actor (who performed the action)
    actor_id VARCHAR(64),
    actor_type VARCHAR(32) NOT NULL DEFAULT 'SYSTEM', -- SYSTEM, USER, ADMIN, API

    -- Action details (JSONB for flexibility)
    action_details JSONB NOT NULL DEFAULT '{}',

    -- Resource reference (what was affected)
    resource_type VARCHAR(64),
    resource_id VARCHAR(64),

    -- Hash chain for integrity verification
    sequence_number BIGSERIAL,
    previous_hash VARCHAR(64), -- SHA256 of previous record (NULL for first record per tenant)
    current_hash VARCHAR(64) NOT NULL, -- SHA256 of this record + previous_hash

    -- Request context
    ip_address INET,
    user_agent TEXT,
    request_id VARCHAR(64),

    -- Immutable timestamp
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_compliance_audit_tenant
    ON compliance_audit_log(tenant_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_compliance_audit_event_type
    ON compliance_audit_log(tenant_id, event_type, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_compliance_audit_actor
    ON compliance_audit_log(tenant_id, actor_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_compliance_audit_resource
    ON compliance_audit_log(tenant_id, resource_type, resource_id);
CREATE INDEX IF NOT EXISTS idx_compliance_audit_sequence
    ON compliance_audit_log(tenant_id, sequence_number);

-- Prevent updates and deletes on compliance_audit_log (append-only)
CREATE OR REPLACE FUNCTION prevent_audit_modification()
RETURNS TRIGGER AS $$
BEGIN
    RAISE EXCEPTION 'Compliance audit log is immutable - updates and deletes are not allowed';
END;
$$ LANGUAGE plpgsql;

-- Apply the trigger only if it doesn't exist
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_trigger
        WHERE tgname = 'trigger_prevent_audit_update'
    ) THEN
        CREATE TRIGGER trigger_prevent_audit_update
            BEFORE UPDATE ON compliance_audit_log
            FOR EACH ROW EXECUTE FUNCTION prevent_audit_modification();
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_trigger
        WHERE tgname = 'trigger_prevent_audit_delete'
    ) THEN
        CREATE TRIGGER trigger_prevent_audit_delete
            BEFORE DELETE ON compliance_audit_log
            FOR EACH ROW EXECUTE FUNCTION prevent_audit_modification();
    END IF;
END $$;

-- Enable RLS on compliance_audit_log
ALTER TABLE compliance_audit_log ENABLE ROW LEVEL SECURITY;

-- RLS policy: tenant isolation
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_policies
        WHERE tablename = 'compliance_audit_log'
        AND policyname = 'compliance_audit_tenant_isolation'
    ) THEN
        CREATE POLICY compliance_audit_tenant_isolation ON compliance_audit_log
            FOR ALL
            USING (tenant_id = current_setting('app.current_tenant', true));
    END IF;
END $$;

-- RLS policy: insert only (no updates/deletes even if triggers are bypassed)
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_policies
        WHERE tablename = 'compliance_audit_log'
        AND policyname = 'compliance_audit_insert_only'
    ) THEN
        CREATE POLICY compliance_audit_insert_only ON compliance_audit_log
            FOR INSERT
            WITH CHECK (tenant_id = current_setting('app.current_tenant', true));
    END IF;
END $$;

-- Grant permissions
GRANT SELECT, INSERT ON compliance_audit_log TO ramp_app;
REVOKE UPDATE, DELETE ON compliance_audit_log FROM ramp_app;

-- Comment for documentation
COMMENT ON TABLE compliance_audit_log IS
    'Immutable compliance audit trail with hash chain for regulatory inspections.
     This table is append-only - updates and deletes are prevented by triggers and RLS.';

COMMENT ON COLUMN compliance_audit_log.current_hash IS
    'SHA256 hash of (event_type + actor_id + action_details + resource_id + created_at + previous_hash)';

COMMENT ON COLUMN compliance_audit_log.previous_hash IS
    'Hash of the previous record in the chain for this tenant. NULL for the first record.';
