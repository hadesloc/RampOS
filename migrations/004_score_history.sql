CREATE TABLE risk_score_history (
  id UUID PRIMARY KEY,
  user_id VARCHAR(255) NOT NULL,
  intent_id VARCHAR(255),
  score DECIMAL(5,2) NOT NULL,
  triggered_rules JSONB,
  action_taken VARCHAR(50),
  created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_score_history_user ON risk_score_history(user_id, created_at DESC);
