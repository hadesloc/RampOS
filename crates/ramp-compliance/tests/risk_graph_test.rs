use chrono::Utc;
use rust_decimal::Decimal;
use serde_json::json;
use uuid::Uuid;

use ramp_common::types::{IntentId, TenantId, UserId};
use ramp_compliance::{
    RiskGraphAssembler, RiskLabReplayRecord, RiskLabRuleVersion,
};

fn sample_replay() -> RiskLabReplayRecord {
    RiskLabReplayRecord {
        score_id: Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap(),
        tenant_id: TenantId::new("tenant_risk_graph"),
        user_id: UserId::new("user_risk_graph"),
        intent_id: Some(IntentId::new("intent_risk_graph")),
        score: Decimal::from(78),
        shadow_score: Some(Decimal::from(84)),
        score_delta: Some(Decimal::from(6)),
        decision: Some("Review".to_string()),
        shadow_decision: Some("Block".to_string()),
        decision_changed: true,
        action_taken: Some("Review".to_string()),
        triggered_rules: json!(["velocity_1h_exceeded", "new_device_high_value"]),
        feature_vector: json!({
            "velocity1h": 8,
            "amountUsd": 24000,
            "isCrossBorder": 1
        }),
        score_explanation: json!({
            "riskFactors": [
                {
                    "ruleName": "velocity_1h_exceeded",
                    "contribution": 15,
                    "description": "8 txns in 1h (limit 5)"
                }
            ]
        }),
        decision_snapshot: json!({
            "currentDecision": "Review",
            "shadowDecision": "Block"
        }),
        replay_metadata: json!({
            "candidateVersionId": "fraud-rules-v5",
            "candidateVersionLabel": "shadow-v5",
            "replayMode": "SHADOW"
        }),
        created_at: Utc::now(),
        rule_version: RiskLabRuleVersion {
            id: Some(Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap()),
            version_number: Some(4),
            version_state: Some("ACTIVE".to_string()),
            version_label: Some("v4".to_string()),
            parent_version_id: None,
            is_active: Some(true),
            created_at: Some(Utc::now()),
            activated_at: Some(Utc::now()),
            version_metadata: json!({}),
            scorer_config: json!({ "velocity1hLimit": 5 }),
            decision_thresholds: json!({ "allowBelow": 30, "blockAbove": 80 }),
        },
    }
}

#[test]
fn risk_graph_assembler_builds_nodes_edges_and_summary_from_replay() {
    let graph = RiskGraphAssembler::assemble(&sample_replay());

    assert!(graph.summary.decision_changed);
    assert!(graph.summary.node_count >= 5);
    assert!(graph.summary.edge_count >= 4);
    assert_eq!(graph.summary.factor_count, 1);
    assert_eq!(graph.summary.feature_count, 3);
    assert!(graph
        .nodes
        .iter()
        .any(|node| node.kind == "RULE_VERSION"));
    assert!(graph
        .edges
        .iter()
        .any(|edge| edge.relationship == "compared_with"));
}
