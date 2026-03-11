use chrono::{Duration, Utc};
use ramp_common::types::{TenantId, UserId};
use ramp_core::repository::{settlement::SettlementRow, webhook::WebhookEventRow};
use ramp_core::service::{
    redact_replay_bundle,
    settlement::{Settlement, SettlementStatus},
    ReplayBundleAssembler, ReplayTimelineEntry, ReplayTimelineSource,
};
use ramp_core::test_utils::{sandbox_offramp_fixture, sandbox_payin_fixture};
use serde_json::json;

#[test]
fn sandbox_replay_bundle_stays_ordered_and_redacted() {
    let tenant_id = TenantId::new("tenant_sandbox");
    let user_id = UserId::new("user_sandbox");
    let payin = sandbox_payin_fixture(&tenant_id, &user_id);
    let offramp = sandbox_offramp_fixture(&tenant_id, &user_id);
    let base_time = payin.intent.created_at;

    let bundle = ReplayBundleAssembler::assemble(
        payin.intent.id.clone(),
        vec![
            ReplayTimelineEntry::from_settlement(Settlement {
                id: "stl_sandbox_001".to_string(),
                offramp_intent_id: offramp.intent.id.clone(),
                status: SettlementStatus::Completed,
                bank_reference: Some("SBX-STL-001".to_string()),
                error_message: None,
                created_at: base_time + Duration::minutes(4),
                updated_at: base_time + Duration::minutes(5),
            }),
            ReplayTimelineEntry::from_settlement_row(SettlementRow {
                id: "stl_sandbox_row_001".to_string(),
                offramp_intent_id: offramp.intent.id.clone(),
                status: "SETTLED".to_string(),
                bank_reference: Some("SBX-STL-ROW-001".to_string()),
                error_message: None,
                created_at: base_time + Duration::minutes(6),
                updated_at: base_time + Duration::minutes(7),
            }),
            ReplayTimelineEntry::from_webhook_event(WebhookEventRow {
                id: "evt_sandbox_001".to_string(),
                tenant_id: tenant_id.0.clone(),
                event_type: "intent.status.changed".to_string(),
                intent_id: Some(payin.intent.id.clone()),
                payload: json!({
                    "apiSecret": "ramp_secret_live",
                    "headers": {
                        "Authorization": "Bearer sandbox-token"
                    },
                    "state": payin.intent.state,
                }),
                status: "DELIVERED".to_string(),
                attempts: 1,
                max_attempts: 10,
                last_attempt_at: Some(base_time + Duration::minutes(1)),
                next_attempt_at: None,
                last_error: None,
                delivered_at: Some(base_time + Duration::minutes(1)),
                response_status: Some(200),
                created_at: base_time,
            }),
        ],
    );

    let redacted = redact_replay_bundle(&bundle);

    assert_eq!(redacted.entries.len(), 3);
    assert_eq!(redacted.entries[0].sequence, 1);
    assert_eq!(redacted.entries[0].source, ReplayTimelineSource::Webhook);
    assert_eq!(redacted.entries[1].source, ReplayTimelineSource::Settlement);
    assert_eq!(redacted.entries[2].source, ReplayTimelineSource::Settlement);
    assert_eq!(
        redacted.entries[0].payload["data"]["apiSecret"],
        json!("[REDACTED]")
    );
    assert_eq!(
        redacted.entries[0].payload["data"]["headers"]["Authorization"],
        json!("[REDACTED]")
    );
    assert_eq!(
        redacted.entries[2].payload["offrampIntentId"],
        json!(offramp.intent.id)
    );
    assert!(redacted.generated_at >= base_time);
    assert!(redacted.generated_at <= Utc::now());
}
