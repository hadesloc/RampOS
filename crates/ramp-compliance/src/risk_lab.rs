use chrono::{DateTime, Utc};
use ramp_common::{
    types::{IntentId, TenantId, UserId},
    Error, Result,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskLabComparisonQuery {
    pub user_id: Option<String>,
    pub intent_id: Option<String>,
    pub changed_only: bool,
    pub limit: i64,
}

impl RiskLabComparisonQuery {
    fn capped_limit(&self) -> i64 {
        self.limit.clamp(1, 100)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskLabRuleVersion {
    pub id: Option<Uuid>,
    pub version_number: Option<i32>,
    pub version_state: Option<String>,
    pub version_label: Option<String>,
    pub parent_version_id: Option<Uuid>,
    pub is_active: Option<bool>,
    pub created_at: Option<DateTime<Utc>>,
    pub activated_at: Option<DateTime<Utc>>,
    pub version_metadata: Value,
    pub scorer_config: Value,
    pub decision_thresholds: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskLabReplayRecord {
    pub score_id: Uuid,
    pub tenant_id: TenantId,
    pub user_id: UserId,
    pub intent_id: Option<IntentId>,
    pub score: Decimal,
    pub shadow_score: Option<Decimal>,
    pub score_delta: Option<Decimal>,
    pub decision: Option<String>,
    pub shadow_decision: Option<String>,
    pub decision_changed: bool,
    pub action_taken: Option<String>,
    pub triggered_rules: Value,
    pub feature_vector: Value,
    pub score_explanation: Value,
    pub decision_snapshot: Value,
    pub replay_metadata: Value,
    pub created_at: DateTime<Utc>,
    pub rule_version: RiskLabRuleVersion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskLabComparisonRecord {
    pub score_id: Uuid,
    pub user_id: UserId,
    pub intent_id: Option<IntentId>,
    pub created_at: DateTime<Utc>,
    pub current_version_id: Option<Uuid>,
    pub current_version_label: Option<String>,
    pub current_version_state: Option<String>,
    pub candidate_version_id: Option<String>,
    pub candidate_version_label: Option<String>,
    pub replay_mode: Option<String>,
    pub score: Decimal,
    pub shadow_score: Option<Decimal>,
    pub score_delta: Option<Decimal>,
    pub decision: Option<String>,
    pub shadow_decision: Option<String>,
    pub decision_changed: bool,
    pub triggered_rule_count: usize,
    pub factor_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskLabComparisonStats {
    pub compared_scores: usize,
    pub changed_decisions: usize,
    pub avg_abs_score_delta: Option<Decimal>,
    pub max_abs_score_delta: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskLabComparisonView {
    pub data: Vec<RiskLabComparisonRecord>,
    pub total: usize,
    pub stats: RiskLabComparisonStats,
}

pub struct RiskLabManager {
    pool: PgPool,
}

impl RiskLabManager {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_replay(
        &self,
        tenant_id: &TenantId,
        score_id: Uuid,
    ) -> Result<RiskLabReplayRecord> {
        let row = sqlx::query(
            r#"
            SELECT
                h.id AS score_id,
                h.tenant_id,
                h.user_id,
                h.intent_id,
                h.score,
                h.shadow_score,
                h.shadow_decision,
                h.triggered_rules,
                h.action_taken,
                h.feature_vector,
                h.score_explanation,
                h.decision_snapshot,
                h.replay_metadata,
                h.created_at,
                v.id AS rule_version_id,
                v.version_number,
                v.version_state,
                v.version_label,
                v.parent_version_id,
                v.is_active,
                v.created_at AS rule_version_created_at,
                v.activated_at,
                v.version_metadata,
                v.scorer_config,
                v.decision_thresholds
            FROM risk_score_history h
            LEFT JOIN aml_rule_versions v ON v.id = h.rule_version_id
            WHERE h.tenant_id = $1 AND h.id = $2
            "#,
        )
        .bind(tenant_id.to_string())
        .bind(score_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(Error::from)?;

        let row = row.ok_or_else(|| Error::NotFound(format!("Risk lab replay {}", score_id)))?;
        Ok(map_replay_row(row)?)
    }

    pub async fn list_comparisons(
        &self,
        tenant_id: &TenantId,
        query: &RiskLabComparisonQuery,
    ) -> Result<RiskLabComparisonView> {
        let rows = sqlx::query(
            r#"
            SELECT
                h.id AS score_id,
                h.tenant_id,
                h.user_id,
                h.intent_id,
                h.score,
                h.shadow_score,
                h.shadow_decision,
                h.triggered_rules,
                h.action_taken,
                h.score_explanation,
                h.decision_snapshot,
                h.replay_metadata,
                h.created_at,
                v.id AS rule_version_id,
                v.version_number,
                v.version_state,
                v.version_label,
                v.parent_version_id,
                v.is_active,
                v.created_at AS rule_version_created_at,
                v.activated_at,
                v.version_metadata,
                v.scorer_config,
                v.decision_thresholds
            FROM risk_score_history h
            LEFT JOIN aml_rule_versions v ON v.id = h.rule_version_id
            WHERE h.tenant_id = $1
              AND ($2::text IS NULL OR h.user_id = $2)
              AND ($3::text IS NULL OR h.intent_id = $3)
            ORDER BY h.created_at DESC
            LIMIT $4
            "#,
        )
        .bind(tenant_id.to_string())
        .bind(query.user_id.as_deref())
        .bind(query.intent_id.as_deref())
        .bind(query.capped_limit())
        .fetch_all(&self.pool)
        .await
        .map_err(Error::from)?;

        let mut data = rows
            .into_iter()
            .map(map_comparison_row)
            .collect::<Result<Vec<_>>>()?;

        if query.changed_only {
            data.retain(|row| row.decision_changed);
        }

        let stats = summarize_comparisons(&data);
        let total = data.len();

        Ok(RiskLabComparisonView { data, total, stats })
    }
}

fn map_replay_row(row: sqlx::postgres::PgRow) -> Result<RiskLabReplayRecord> {
    let score: Decimal = row.try_get("score").map_err(Error::from)?;
    let shadow_score: Option<Decimal> = row.try_get("shadow_score").map_err(Error::from)?;
    let decision_snapshot = normalize_json(row.try_get("decision_snapshot").map_err(Error::from)?);
    let replay_metadata = normalize_json(row.try_get("replay_metadata").map_err(Error::from)?);
    let action_taken: Option<String> = row.try_get("action_taken").map_err(Error::from)?;
    let decision = extract_current_decision(&decision_snapshot, action_taken.clone());
    let shadow_decision = row
        .try_get::<Option<String>, _>("shadow_decision")
        .map_err(Error::from)?
        .or_else(|| extract_shadow_decision(&decision_snapshot, &replay_metadata));

    Ok(RiskLabReplayRecord {
        score_id: row.try_get("score_id").map_err(Error::from)?,
        tenant_id: TenantId::new(row.try_get::<String, _>("tenant_id").map_err(Error::from)?),
        user_id: UserId::new(row.try_get::<String, _>("user_id").map_err(Error::from)?),
        intent_id: row
            .try_get::<Option<String>, _>("intent_id")
            .map_err(Error::from)?
            .map(IntentId::new),
        score,
        shadow_score,
        score_delta: shadow_score.map(|candidate| candidate - score),
        decision_changed: decision_changed(decision.as_deref(), shadow_decision.as_deref()),
        decision,
        shadow_decision,
        action_taken,
        triggered_rules: normalize_json(row.try_get("triggered_rules").map_err(Error::from)?),
        feature_vector: normalize_json(row.try_get("feature_vector").map_err(Error::from)?),
        score_explanation: normalize_json(row.try_get("score_explanation").map_err(Error::from)?),
        decision_snapshot,
        replay_metadata,
        created_at: row.try_get("created_at").map_err(Error::from)?,
        rule_version: map_rule_version(&row)?,
    })
}

fn map_comparison_row(row: sqlx::postgres::PgRow) -> Result<RiskLabComparisonRecord> {
    let score: Decimal = row.try_get("score").map_err(Error::from)?;
    let shadow_score: Option<Decimal> = row.try_get("shadow_score").map_err(Error::from)?;
    let score_explanation = normalize_json(row.try_get("score_explanation").map_err(Error::from)?);
    let decision_snapshot = normalize_json(row.try_get("decision_snapshot").map_err(Error::from)?);
    let replay_metadata = normalize_json(row.try_get("replay_metadata").map_err(Error::from)?);
    let action_taken: Option<String> = row.try_get("action_taken").map_err(Error::from)?;
    let decision = extract_current_decision(&decision_snapshot, action_taken);
    let shadow_decision = row
        .try_get::<Option<String>, _>("shadow_decision")
        .map_err(Error::from)?
        .or_else(|| extract_shadow_decision(&decision_snapshot, &replay_metadata));

    Ok(RiskLabComparisonRecord {
        score_id: row.try_get("score_id").map_err(Error::from)?,
        user_id: UserId::new(row.try_get::<String, _>("user_id").map_err(Error::from)?),
        intent_id: row
            .try_get::<Option<String>, _>("intent_id")
            .map_err(Error::from)?
            .map(IntentId::new),
        created_at: row.try_get("created_at").map_err(Error::from)?,
        current_version_id: row.try_get("rule_version_id").map_err(Error::from)?,
        current_version_label: resolve_rule_version_label(
            row.try_get("version_label").map_err(Error::from)?,
            row.try_get("version_number").map_err(Error::from)?,
        ),
        current_version_state: row.try_get("version_state").map_err(Error::from)?,
        candidate_version_id: extract_candidate_version_id(&replay_metadata),
        candidate_version_label: extract_candidate_version_label(&replay_metadata),
        replay_mode: extract_replay_mode(&replay_metadata),
        score,
        shadow_score,
        score_delta: shadow_score.map(|candidate| candidate - score),
        decision_changed: decision_changed(decision.as_deref(), shadow_decision.as_deref()),
        decision,
        shadow_decision,
        triggered_rule_count: count_triggered_rules(&normalize_json(
            row.try_get("triggered_rules").map_err(Error::from)?,
        )),
        factor_count: count_explained_factors(&score_explanation),
    })
}

fn map_rule_version(row: &sqlx::postgres::PgRow) -> Result<RiskLabRuleVersion> {
    let version_number: Option<i32> = row.try_get("version_number").map_err(Error::from)?;

    Ok(RiskLabRuleVersion {
        id: row.try_get("rule_version_id").map_err(Error::from)?,
        version_number,
        version_state: row.try_get("version_state").map_err(Error::from)?,
        version_label: resolve_rule_version_label(
            row.try_get("version_label").map_err(Error::from)?,
            version_number,
        ),
        parent_version_id: row.try_get("parent_version_id").map_err(Error::from)?,
        is_active: row.try_get("is_active").map_err(Error::from)?,
        created_at: row
            .try_get("rule_version_created_at")
            .map_err(Error::from)?,
        activated_at: row.try_get("activated_at").map_err(Error::from)?,
        version_metadata: normalize_json(row.try_get("version_metadata").map_err(Error::from)?),
        scorer_config: normalize_json(row.try_get("scorer_config").map_err(Error::from)?),
        decision_thresholds: normalize_json(
            row.try_get("decision_thresholds").map_err(Error::from)?,
        ),
    })
}

fn resolve_rule_version_label(
    version_label: Option<String>,
    version_number: Option<i32>,
) -> Option<String> {
    version_label.or_else(|| version_number.map(|value| format!("v{}", value)))
}

fn normalize_json(value: Option<Value>) -> Value {
    match value {
        Some(Value::Null) | None => serde_json::json!({}),
        Some(other) => other,
    }
}

fn extract_current_decision(
    decision_snapshot: &Value,
    action_taken: Option<String>,
) -> Option<String> {
    extract_string(
        decision_snapshot,
        &["currentDecision", "decision", "action", "current_action"],
    )
    .or(action_taken)
}

fn extract_shadow_decision(decision_snapshot: &Value, replay_metadata: &Value) -> Option<String> {
    extract_string(
        decision_snapshot,
        &["shadowDecision", "candidateDecision", "simulatedDecision"],
    )
    .or_else(|| {
        extract_string(
            replay_metadata,
            &["shadowDecision", "candidateDecision", "simulatedDecision"],
        )
    })
}

fn extract_candidate_version_id(replay_metadata: &Value) -> Option<String> {
    extract_string(
        replay_metadata,
        &[
            "candidateVersionId",
            "shadowVersionId",
            "compareVersionId",
            "replayVersionId",
        ],
    )
}

fn extract_candidate_version_label(replay_metadata: &Value) -> Option<String> {
    extract_string(
        replay_metadata,
        &[
            "candidateVersionLabel",
            "shadowVersionLabel",
            "compareVersionLabel",
            "replayVersionLabel",
        ],
    )
}

fn extract_replay_mode(replay_metadata: &Value) -> Option<String> {
    extract_string(
        replay_metadata,
        &["replayMode", "mode", "simulationMode", "compareMode"],
    )
}

fn extract_string(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        value
            .get(*key)
            .and_then(|item| item.as_str())
            .map(|item| item.to_string())
    })
}

fn count_triggered_rules(triggered_rules: &Value) -> usize {
    triggered_rules
        .as_array()
        .map(|items| items.len())
        .unwrap_or_default()
}

fn count_explained_factors(score_explanation: &Value) -> usize {
    score_explanation
        .get("riskFactors")
        .or_else(|| score_explanation.get("risk_factors"))
        .and_then(|value| value.as_array())
        .map(|items| items.len())
        .unwrap_or_default()
}

fn decision_changed(current: Option<&str>, candidate: Option<&str>) -> bool {
    current.map(normalize_decision) != candidate.map(normalize_decision)
}

fn normalize_decision(value: &str) -> String {
    value.trim().to_ascii_uppercase()
}

fn summarize_comparisons(data: &[RiskLabComparisonRecord]) -> RiskLabComparisonStats {
    let deltas: Vec<Decimal> = data
        .iter()
        .filter_map(|row| row.score_delta.map(|delta| delta.abs()))
        .collect();

    let avg_abs_score_delta = if deltas.is_empty() {
        None
    } else {
        let sum: Decimal = deltas.iter().copied().sum();
        Some(sum / Decimal::from(deltas.len() as i64))
    };

    let max_abs_score_delta = deltas.iter().copied().max();

    RiskLabComparisonStats {
        compared_scores: data.len(),
        changed_decisions: data.iter().filter(|row| row.decision_changed).count(),
        avg_abs_score_delta,
        max_abs_score_delta,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn sample_comparison(
        score_id: &str,
        score: i64,
        shadow_score: Option<i64>,
        decision: Option<&str>,
        shadow_decision: Option<&str>,
    ) -> RiskLabComparisonRecord {
        RiskLabComparisonRecord {
            score_id: Uuid::parse_str(score_id).expect("valid uuid"),
            user_id: UserId::new("user_123"),
            intent_id: Some(IntentId::new("intent_123")),
            created_at: Utc.with_ymd_and_hms(2026, 3, 8, 12, 0, 0).single().unwrap(),
            current_version_id: None,
            current_version_label: Some("v3".to_string()),
            current_version_state: Some("ACTIVE".to_string()),
            candidate_version_id: Some("candidate-v4".to_string()),
            candidate_version_label: Some("shadow-v4".to_string()),
            replay_mode: Some("SHADOW".to_string()),
            score: Decimal::from(score),
            shadow_score: shadow_score.map(Decimal::from),
            score_delta: shadow_score.map(|candidate| Decimal::from(candidate - score)),
            decision: decision.map(|value| value.to_string()),
            shadow_decision: shadow_decision.map(|value| value.to_string()),
            decision_changed: decision_changed(decision, shadow_decision),
            triggered_rule_count: 2,
            factor_count: 1,
        }
    }

    #[test]
    fn summarize_comparisons_tracks_changed_decisions_and_abs_delta() {
        let comparisons = vec![
            sample_comparison(
                "11111111-1111-1111-1111-111111111111",
                41,
                Some(52),
                Some("Review"),
                Some("Block"),
            ),
            sample_comparison(
                "22222222-2222-2222-2222-222222222222",
                20,
                Some(18),
                Some("Allow"),
                Some("Allow"),
            ),
            sample_comparison(
                "33333333-3333-3333-3333-333333333333",
                77,
                None,
                Some("Block"),
                None,
            ),
        ];

        let summary = summarize_comparisons(&comparisons);

        assert_eq!(summary.compared_scores, 3);
        assert_eq!(summary.changed_decisions, 2);
        assert_eq!(summary.avg_abs_score_delta, Some(Decimal::new(65, 1)));
        assert_eq!(summary.max_abs_score_delta, Some(Decimal::from(11)));
    }

    #[test]
    fn candidate_version_helpers_support_shadow_aliases() {
        let replay_metadata = serde_json::json!({
            "shadowVersionId": "rule-shadow-02",
            "shadowVersionLabel": "shadow-v2",
            "mode": "shadow"
        });

        assert_eq!(
            extract_candidate_version_id(&replay_metadata).as_deref(),
            Some("rule-shadow-02")
        );
        assert_eq!(
            extract_candidate_version_label(&replay_metadata).as_deref(),
            Some("shadow-v2")
        );
        assert_eq!(
            extract_replay_mode(&replay_metadata).as_deref(),
            Some("shadow")
        );
    }

    #[test]
    fn current_decision_prefers_snapshot_then_action_taken() {
        let snapshot = serde_json::json!({
            "currentDecision": "Review",
            "shadowDecision": "Block"
        });

        assert_eq!(
            extract_current_decision(&snapshot, Some("Allow".to_string())).as_deref(),
            Some("Review")
        );
        assert_eq!(
            extract_shadow_decision(&snapshot, &serde_json::json!({})).as_deref(),
            Some("Block")
        );
    }
}
