-- Create compliance_cases table
CREATE TABLE compliance_cases (
    id VARCHAR(50) PRIMARY KEY,
    tenant_id VARCHAR(50) NOT NULL,
    user_id VARCHAR(50),
    intent_id VARCHAR(50),
    case_type VARCHAR(50) NOT NULL,
    severity VARCHAR(20) NOT NULL,
    status VARCHAR(20) NOT NULL,
    detection_data JSONB NOT NULL DEFAULT '{}',
    assigned_to VARCHAR(50),
    resolution TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at TIMESTAMPTZ
);

-- Create index on user_id for faster lookups
CREATE INDEX idx_compliance_cases_user_id ON compliance_cases(user_id);
-- Create index on status for filtering open cases
CREATE INDEX idx_compliance_cases_status ON compliance_cases(status);
-- Create index on tenant_id for multi-tenancy
CREATE INDEX idx_compliance_cases_tenant_id ON compliance_cases(tenant_id);

-- Create case_notes table
CREATE TABLE case_notes (
    id UUID PRIMARY KEY,
    case_id VARCHAR(50) NOT NULL REFERENCES compliance_cases(id) ON DELETE CASCADE,
    author_id VARCHAR(50),
    content TEXT NOT NULL,
    note_type VARCHAR(20) NOT NULL,
    is_internal BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create index on case_id for faster lookups of notes
CREATE INDEX idx_case_notes_case_id ON case_notes(case_id);
