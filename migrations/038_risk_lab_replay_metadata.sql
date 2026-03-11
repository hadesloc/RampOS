-- Migration 038: Risk lab replay and explainability metadata
-- Extends existing AML rule versioning and risk score history tables with
-- bounded metadata for replay/simulation/explainability. No second rules
-- engine and no separate graph store are introduced here.

ALTER TABLE aml_rule_versions
    ADD COLUMN IF NOT EXISTS version_state TEXT,
    ADD COLUMN IF NOT EXISTS version_label TEXT,
    ADD COLUMN IF NOT EXISTS parent_version_id UUID REFERENCES aml_rule_versions(id),
    ADD COLUMN IF NOT EXISTS version_metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    ADD COLUMN IF NOT EXISTS scorer_config JSONB,
    ADD COLUMN IF NOT EXISTS decision_thresholds JSONB;

UPDATE aml_rule_versions
SET version_state = CASE
    WHEN is_active THEN 'ACTIVE'
    ELSE 'DRAFT'
END
WHERE version_state IS NULL;

ALTER TABLE aml_rule_versions
    ALTER COLUMN version_state SET DEFAULT 'DRAFT',
    ALTER COLUMN version_state SET NOT NULL;

ALTER TABLE aml_rule_versions
    ADD CONSTRAINT aml_rule_versions_version_state_check CHECK (
        version_state IN ('DRAFT', 'ACTIVE', 'SHADOW', 'ARCHIVED')
    );

ALTER TABLE aml_rule_versions
    ADD CONSTRAINT aml_rule_versions_version_metadata_object CHECK (
        jsonb_typeof(version_metadata) = 'object'
    );

ALTER TABLE aml_rule_versions
    ADD CONSTRAINT aml_rule_versions_scorer_config_object CHECK (
        scorer_config IS NULL OR jsonb_typeof(scorer_config) = 'object'
    );

ALTER TABLE aml_rule_versions
    ADD CONSTRAINT aml_rule_versions_decision_thresholds_object CHECK (
        decision_thresholds IS NULL OR jsonb_typeof(decision_thresholds) = 'object'
    );

CREATE INDEX IF NOT EXISTS idx_rule_versions_state
    ON aml_rule_versions(tenant_id, version_state, version_number DESC);

ALTER TABLE risk_score_history
    ADD COLUMN IF NOT EXISTS rule_version_id UUID REFERENCES aml_rule_versions(id),
    ADD COLUMN IF NOT EXISTS feature_vector JSONB,
    ADD COLUMN IF NOT EXISTS score_explanation JSONB,
    ADD COLUMN IF NOT EXISTS decision_snapshot JSONB,
    ADD COLUMN IF NOT EXISTS shadow_score DECIMAL(5,2),
    ADD COLUMN IF NOT EXISTS shadow_decision VARCHAR(50),
    ADD COLUMN IF NOT EXISTS replay_metadata JSONB NOT NULL DEFAULT '{}'::jsonb;

ALTER TABLE risk_score_history
    ADD CONSTRAINT risk_score_history_feature_vector_object CHECK (
        feature_vector IS NULL OR jsonb_typeof(feature_vector) = 'object'
    );

ALTER TABLE risk_score_history
    ADD CONSTRAINT risk_score_history_score_explanation_object CHECK (
        score_explanation IS NULL OR jsonb_typeof(score_explanation) = 'object'
    );

ALTER TABLE risk_score_history
    ADD CONSTRAINT risk_score_history_decision_snapshot_object CHECK (
        decision_snapshot IS NULL OR jsonb_typeof(decision_snapshot) = 'object'
    );

ALTER TABLE risk_score_history
    ADD CONSTRAINT risk_score_history_replay_metadata_object CHECK (
        jsonb_typeof(replay_metadata) = 'object'
    );

CREATE INDEX IF NOT EXISTS idx_risk_score_history_rule_version
    ON risk_score_history(rule_version_id, created_at DESC)
    WHERE rule_version_id IS NOT NULL;
