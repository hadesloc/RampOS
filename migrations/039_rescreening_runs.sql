-- Migration 039: Continuous compliance rescreening runs
-- Adds bounded scheduler/alert/restriction state for ongoing KYC/PEP/adverse-media checks.

ALTER TYPE compliance_event_type ADD VALUE IF NOT EXISTS 'rescreening_run_completed';
ALTER TYPE compliance_event_type ADD VALUE IF NOT EXISTS 'rescreening_alert_queued';
ALTER TYPE compliance_event_type ADD VALUE IF NOT EXISTS 'rescreening_restriction_applied';

CREATE TABLE compliance_rescreening_runs (
    id TEXT PRIMARY KEY,                                -- "rsr_..." prefix
    tenant_id TEXT NOT NULL REFERENCES tenants(id),
    user_id TEXT NOT NULL,
    trigger_kind TEXT NOT NULL DEFAULT 'SCHEDULED',
    status TEXT NOT NULL DEFAULT 'PENDING',
    priority TEXT NOT NULL DEFAULT 'MEDIUM',
    restriction_status TEXT NOT NULL DEFAULT 'NONE',
    alert_codes JSONB NOT NULL DEFAULT '[]'::jsonb,
    details JSONB NOT NULL DEFAULT '{}'::jsonb,
    scheduled_for TIMESTAMPTZ NOT NULL,
    executed_at TIMESTAMPTZ,
    next_run_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT compliance_rescreening_runs_trigger_kind_check CHECK (
        trigger_kind IN ('SCHEDULED', 'WATCHLIST_DELTA', 'DOCUMENT_EXPIRY')
    ),
    CONSTRAINT compliance_rescreening_runs_status_check CHECK (
        status IN ('PENDING', 'ALERTED', 'RESTRICTED', 'CLEARED')
    ),
    CONSTRAINT compliance_rescreening_runs_priority_check CHECK (
        priority IN ('LOW', 'MEDIUM', 'HIGH', 'CRITICAL')
    ),
    CONSTRAINT compliance_rescreening_runs_restriction_check CHECK (
        restriction_status IN ('NONE', 'REVIEW_REQUIRED', 'RESTRICTED')
    ),
    CONSTRAINT compliance_rescreening_runs_alert_codes_array CHECK (
        jsonb_typeof(alert_codes) = 'array'
    ),
    CONSTRAINT compliance_rescreening_runs_details_object CHECK (
        jsonb_typeof(details) = 'object'
    )
);

CREATE INDEX idx_rescreening_runs_tenant_status
    ON compliance_rescreening_runs(tenant_id, status, priority, scheduled_for DESC);

CREATE INDEX idx_rescreening_runs_user
    ON compliance_rescreening_runs(tenant_id, user_id, scheduled_for DESC);

CREATE INDEX idx_rescreening_runs_next_due
    ON compliance_rescreening_runs(tenant_id, next_run_at)
    WHERE next_run_at IS NOT NULL;

CREATE INDEX idx_rescreening_runs_alert_codes
    ON compliance_rescreening_runs USING GIN (alert_codes);

ALTER TABLE compliance_rescreening_runs ENABLE ROW LEVEL SECURITY;

CREATE POLICY compliance_rescreening_runs_tenant_isolation ON compliance_rescreening_runs
    USING (tenant_id = current_setting('app.current_tenant', true));

CREATE TRIGGER trigger_compliance_rescreening_runs_updated_at
    BEFORE UPDATE ON compliance_rescreening_runs
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();
