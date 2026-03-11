use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::risk_lab::RiskLabReplayRecord;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskGraphNode {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskGraphEdge {
    pub source: String,
    pub target: String,
    pub relationship: String,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskGraphSummary {
    pub score_id: String,
    pub generated_at: DateTime<Utc>,
    pub decision_changed: bool,
    pub node_count: usize,
    pub edge_count: usize,
    pub factor_count: usize,
    pub feature_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskGraphView {
    pub summary: RiskGraphSummary,
    pub nodes: Vec<RiskGraphNode>,
    pub edges: Vec<RiskGraphEdge>,
}

pub struct RiskGraphAssembler;

impl RiskGraphAssembler {
    pub fn assemble(replay: &RiskLabReplayRecord) -> RiskGraphView {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        let score_node_id = format!("score:{}", replay.score_id);
        nodes.push(RiskGraphNode {
            id: score_node_id.clone(),
            label: format!("Risk score {}", replay.score),
            kind: "SCORE".to_string(),
            metadata: serde_json::json!({
                "score": replay.score,
                "shadowScore": replay.shadow_score,
                "scoreDelta": replay.score_delta,
                "createdAt": replay.created_at,
            }),
        });

        if let Some(version_id) = replay.rule_version.id {
            let version_node_id = format!("rule-version:{}", version_id);
            nodes.push(RiskGraphNode {
                id: version_node_id.clone(),
                label: replay
                    .rule_version
                    .version_label
                    .clone()
                    .unwrap_or_else(|| version_id.to_string()),
                kind: "RULE_VERSION".to_string(),
                metadata: serde_json::json!({
                    "versionNumber": replay.rule_version.version_number,
                    "versionState": replay.rule_version.version_state,
                    "isActive": replay.rule_version.is_active,
                    "metadata": replay.rule_version.version_metadata,
                    "scorerConfig": replay.rule_version.scorer_config,
                    "decisionThresholds": replay.rule_version.decision_thresholds,
                }),
            });
            edges.push(RiskGraphEdge {
                source: version_node_id,
                target: score_node_id.clone(),
                relationship: "produced".to_string(),
                metadata: serde_json::json!({}),
            });
        }

        if let Some(decision) = &replay.decision {
            let decision_node_id = format!("decision:{}", replay.score_id);
            nodes.push(RiskGraphNode {
                id: decision_node_id.clone(),
                label: decision.clone(),
                kind: "DECISION".to_string(),
                metadata: replay.decision_snapshot.clone(),
            });
            edges.push(RiskGraphEdge {
                source: score_node_id.clone(),
                target: decision_node_id,
                relationship: "resolved_to".to_string(),
                metadata: serde_json::json!({}),
            });
        }

        if replay.shadow_score.is_some() || replay.shadow_decision.is_some() {
            let shadow_node_id = format!("shadow:{}", replay.score_id);
            nodes.push(RiskGraphNode {
                id: shadow_node_id.clone(),
                label: replay
                    .shadow_decision
                    .clone()
                    .or_else(|| replay.shadow_score.map(|score| score.to_string()))
                    .unwrap_or_else(|| "Shadow".to_string()),
                kind: "SHADOW_OUTCOME".to_string(),
                metadata: serde_json::json!({
                    "score": replay.shadow_score,
                    "decision": replay.shadow_decision,
                    "metadata": replay.replay_metadata,
                }),
            });
            edges.push(RiskGraphEdge {
                source: score_node_id.clone(),
                target: shadow_node_id,
                relationship: "compared_with".to_string(),
                metadata: serde_json::json!({
                    "decisionChanged": replay.decision_changed,
                    "scoreDelta": replay.score_delta,
                }),
            });
        }

        let factor_count = append_factor_nodes(replay, &score_node_id, &mut nodes, &mut edges);
        let feature_count = append_feature_nodes(replay, &score_node_id, &mut nodes, &mut edges);
        append_triggered_rule_nodes(replay, &score_node_id, &mut nodes, &mut edges);

        let summary = RiskGraphSummary {
            score_id: replay.score_id.to_string(),
            generated_at: Utc::now(),
            decision_changed: replay.decision_changed,
            node_count: nodes.len(),
            edge_count: edges.len(),
            factor_count,
            feature_count,
        };

        RiskGraphView {
            summary,
            nodes,
            edges,
        }
    }
}

fn append_factor_nodes(
    replay: &RiskLabReplayRecord,
    score_node_id: &str,
    nodes: &mut Vec<RiskGraphNode>,
    edges: &mut Vec<RiskGraphEdge>,
) -> usize {
    let factors = replay
        .score_explanation
        .get("riskFactors")
        .or_else(|| replay.score_explanation.get("risk_factors"))
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();

    for (idx, factor) in factors.iter().enumerate() {
        let node_id = format!("factor:{}:{}", replay.score_id, idx);
        let label = factor
            .get("ruleName")
            .or_else(|| factor.get("rule_name"))
            .and_then(|value| value.as_str())
            .unwrap_or("risk-factor")
            .to_string();
        nodes.push(RiskGraphNode {
            id: node_id.clone(),
            label,
            kind: "RISK_FACTOR".to_string(),
            metadata: factor.clone(),
        });
        edges.push(RiskGraphEdge {
            source: node_id,
            target: score_node_id.to_string(),
            relationship: "contributes_to".to_string(),
            metadata: serde_json::json!({}),
        });
    }

    factors.len()
}

fn append_feature_nodes(
    replay: &RiskLabReplayRecord,
    score_node_id: &str,
    nodes: &mut Vec<RiskGraphNode>,
    edges: &mut Vec<RiskGraphEdge>,
) -> usize {
    let Some(features) = replay.feature_vector.as_object() else {
        return 0;
    };

    for (key, value) in features {
        let node_id = format!("feature:{}:{}", replay.score_id, key);
        nodes.push(RiskGraphNode {
            id: node_id.clone(),
            label: key.clone(),
            kind: "FEATURE".to_string(),
            metadata: serde_json::json!({ "value": value }),
        });
        edges.push(RiskGraphEdge {
            source: node_id,
            target: score_node_id.to_string(),
            relationship: "observed_in".to_string(),
            metadata: serde_json::json!({}),
        });
    }

    features.len()
}

fn append_triggered_rule_nodes(
    replay: &RiskLabReplayRecord,
    score_node_id: &str,
    nodes: &mut Vec<RiskGraphNode>,
    edges: &mut Vec<RiskGraphEdge>,
) {
    let Some(triggered_rules) = replay.triggered_rules.as_array() else {
        return;
    };

    for (idx, rule) in triggered_rules.iter().enumerate() {
        let Some(rule_name) = rule.as_str() else {
            continue;
        };

        let node_id = format!("triggered-rule:{}:{}", replay.score_id, idx);
        nodes.push(RiskGraphNode {
            id: node_id.clone(),
            label: rule_name.to_string(),
            kind: "TRIGGERED_RULE".to_string(),
            metadata: serde_json::json!({}),
        });
        edges.push(RiskGraphEdge {
            source: node_id,
            target: score_node_id.to_string(),
            relationship: "matched".to_string(),
            metadata: serde_json::json!({}),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::risk_lab::{RiskLabReplayRecord, RiskLabRuleVersion};
    use chrono::TimeZone;
    use ramp_common::types::{IntentId, TenantId, UserId};
    use rust_decimal::Decimal;
    use uuid::Uuid;

    #[test]
    fn assemble_graph_from_relational_and_json_backed_replay() {
        let replay = RiskLabReplayRecord {
            score_id: Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap(),
            tenant_id: TenantId::new("tenant_risk_lab"),
            user_id: UserId::new("user_01"),
            intent_id: Some(IntentId::new("intent_01")),
            score: Decimal::from(72),
            shadow_score: Some(Decimal::from(83)),
            score_delta: Some(Decimal::from(11)),
            decision: Some("Review".to_string()),
            shadow_decision: Some("Block".to_string()),
            decision_changed: true,
            action_taken: Some("Review".to_string()),
            triggered_rules: serde_json::json!(["velocity_1h", "device_change"]),
            feature_vector: serde_json::json!({
                "velocity1h": 8,
                "deviceMismatch": true
            }),
            score_explanation: serde_json::json!({
                "riskFactors": [
                    {"ruleName": "velocity_1h", "contribution": 18},
                    {"ruleName": "device_change", "contribution": 12}
                ]
            }),
            decision_snapshot: serde_json::json!({
                "decision": "Review",
                "threshold": 70
            }),
            replay_metadata: serde_json::json!({
                "candidateVersionId": "rule-v4",
                "replayMode": "SHADOW"
            }),
            created_at: Utc
                .with_ymd_and_hms(2026, 3, 8, 12, 30, 0)
                .single()
                .unwrap(),
            rule_version: RiskLabRuleVersion {
                id: Some(Uuid::parse_str("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb").unwrap()),
                version_number: Some(4),
                version_state: Some("ACTIVE".to_string()),
                version_label: Some("v4".to_string()),
                parent_version_id: None,
                is_active: Some(true),
                created_at: Some(Utc.with_ymd_and_hms(2026, 3, 1, 8, 0, 0).single().unwrap()),
                activated_at: None,
                version_metadata: serde_json::json!({"owner": "fraud-ops"}),
                scorer_config: serde_json::json!({"model": "fraud-v4"}),
                decision_thresholds: serde_json::json!({"review": 70, "block": 85}),
            },
        };

        let graph = RiskGraphAssembler::assemble(&replay);

        assert!(graph.summary.decision_changed);
        assert_eq!(graph.summary.factor_count, 2);
        assert_eq!(graph.summary.feature_count, 2);
        assert!(graph.nodes.iter().any(|node| node.kind == "RULE_VERSION"));
        assert!(graph.nodes.iter().any(|node| node.kind == "SHADOW_OUTCOME"));
        assert!(graph
            .edges
            .iter()
            .any(|edge| edge.relationship == "contributes_to"));
        assert!(graph
            .edges
            .iter()
            .any(|edge| edge.relationship == "matched"));
    }
}
