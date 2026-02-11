-- Settlement persistence (F13)
CREATE TABLE IF NOT EXISTS settlements (
    id TEXT PRIMARY KEY,
    offramp_intent_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'PENDING',
    bank_reference TEXT,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_settlements_status ON settlements(status);
CREATE INDEX IF NOT EXISTS idx_settlements_offramp ON settlements(offramp_intent_id);
