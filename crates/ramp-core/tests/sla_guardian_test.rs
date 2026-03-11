use chrono::{Duration, TimeZone, Utc};
use ramp_core::service::{
    IncidentConfidenceMarker, IncidentRecommendationPriority, IncidentSignalSnapshot,
    IncidentTimelineAssembler, IncidentTimelineEntry, IncidentTimelineSourceKind,
    SlaGuardianOwnerLane, SlaGuardianRiskLevel, SlaGuardianService,
};
use serde_json::json;

#[test]
fn sla_guardian_routes_recommendations_to_owner_lanes_and_risk_level() {
    let base_time = Utc.with_ymd_and_hms(2026, 3, 10, 10, 0, 0).single().unwrap();
    let timeline = IncidentTimelineAssembler::assemble_with_signals(
        "incident_guardian_001",
        vec![
            IncidentTimelineEntry::new(
                IncidentTimelineSourceKind::Webhook,
                "evt_guardian_001",
                base_time,
                "Webhook delivery failed",
                "FAILED",
                IncidentConfidenceMarker::Confirmed,
                json!({ "intentId": "intent_guardian_001" }),
            ),
            IncidentTimelineEntry::new(
                IncidentTimelineSourceKind::Settlement,
                "stl_guardian_001",
                base_time + Duration::minutes(1),
                "Settlement processing",
                "PROCESSING",
                IncidentConfidenceMarker::Confirmed,
                json!({ "offrampIntentId": "intent_guardian_001" }),
            ),
        ],
        Vec::new(),
        IncidentSignalSnapshot {
            processing_settlements: 1,
            failed_settlements: 1,
            failed_webhooks: 1,
            critical_fraud_signals: 1,
        },
    );

    let snapshot = SlaGuardianService::new().summarize(
        &timeline,
        IncidentSignalSnapshot {
            processing_settlements: 1,
            failed_settlements: 1,
            failed_webhooks: 1,
            critical_fraud_signals: 1,
        },
    );

    assert_eq!(snapshot.action_mode, timeline.action_mode);
    assert_eq!(snapshot.predicted_breach_risk, SlaGuardianRiskLevel::Critical);
    assert!(snapshot.alerts.iter().any(|alert| {
        alert.code == "review_webhook_delivery"
            && alert.owner_lane == SlaGuardianOwnerLane::Webhooks
    }));
    assert!(snapshot.alerts.iter().any(|alert| {
        alert.code == "review_settlement_state"
            && alert.owner_lane == SlaGuardianOwnerLane::SettlementOps
            && alert.priority == IncidentRecommendationPriority::Immediate
    }));
    assert!(snapshot
        .owner_lanes
        .contains(&SlaGuardianOwnerLane::RiskOps));
}
