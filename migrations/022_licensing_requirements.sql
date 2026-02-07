-- Migration: 015d_licensing_requirements.sql
-- Vietnam Licensing Requirements - Additional Tables
-- NOTE: license_requirements table is created in 015_license_management.sql

-- Tenant License Status table (tenant-specific status per requirement)
CREATE TABLE IF NOT EXISTS tenant_license_status (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants(id),
    requirement_id TEXT NOT NULL REFERENCES license_requirements(id),
    status TEXT NOT NULL DEFAULT 'PENDING',
    license_number TEXT,
    issue_date TIMESTAMPTZ,
    expiry_date TIMESTAMPTZ,
    last_submission_id TEXT,
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(tenant_id, requirement_id)
);

-- Indexes for tenant license status
CREATE INDEX IF NOT EXISTS idx_tenant_license_status_tenant
    ON tenant_license_status(tenant_id);
CREATE INDEX IF NOT EXISTS idx_tenant_license_status_expiry
    ON tenant_license_status(expiry_date)
    WHERE expiry_date IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_tenant_license_status_status
    ON tenant_license_status(status);

-- License Submissions table (document submissions)
CREATE TABLE IF NOT EXISTS license_submissions (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants(id),
    requirement_id TEXT NOT NULL REFERENCES license_requirements(id),
    documents JSONB NOT NULL DEFAULT '[]',
    status TEXT NOT NULL DEFAULT 'SUBMITTED',
    submitted_by TEXT NOT NULL,
    submitted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    reviewed_at TIMESTAMPTZ,
    reviewer_notes TEXT
);

-- Indexes for license submissions
CREATE INDEX IF NOT EXISTS idx_license_submissions_tenant
    ON license_submissions(tenant_id);
CREATE INDEX IF NOT EXISTS idx_license_submissions_requirement
    ON license_submissions(requirement_id);
CREATE INDEX IF NOT EXISTS idx_license_submissions_status
    ON license_submissions(status);

-- Enable RLS on tables
ALTER TABLE tenant_license_status ENABLE ROW LEVEL SECURITY;
ALTER TABLE license_submissions ENABLE ROW LEVEL SECURITY;

-- RLS Policies for tenant_license_status (tenant isolation)
CREATE POLICY tenant_license_status_select ON tenant_license_status
    FOR SELECT USING (
        tenant_id = current_setting('app.current_tenant', true)
    );

CREATE POLICY tenant_license_status_insert ON tenant_license_status
    FOR INSERT WITH CHECK (
        tenant_id = current_setting('app.current_tenant', true)
    );

CREATE POLICY tenant_license_status_update ON tenant_license_status
    FOR UPDATE USING (
        tenant_id = current_setting('app.current_tenant', true)
    );

-- RLS Policies for license_submissions (tenant isolation)
CREATE POLICY license_submissions_select ON license_submissions
    FOR SELECT USING (
        tenant_id = current_setting('app.current_tenant', true)
    );

CREATE POLICY license_submissions_insert ON license_submissions
    FOR INSERT WITH CHECK (
        tenant_id = current_setting('app.current_tenant', true)
    );

CREATE POLICY license_submissions_update ON license_submissions
    FOR UPDATE USING (
        tenant_id = current_setting('app.current_tenant', true)
    );

-- Comments for documentation
COMMENT ON TABLE tenant_license_status IS 'Tracks each tenant''s compliance status for licensing requirements';
COMMENT ON TABLE license_submissions IS 'Document submissions from tenants for licensing applications';
