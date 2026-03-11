use chrono::{Duration, TimeZone, Utc};
use ramp_core::service::{
    redact_replay_bundle, ReplayBundle, ReplayTimelineEntry, ReplayTimelineSource,
};
use serde_json::json;

#[test]
fn replay_export_redacts_secret_fields_recursively() {
    let timestamp = Utc.with_ymd_and_hms(2026, 3, 8, 11, 0, 0).single().unwrap();
    let bundle = ReplayBundle {
        journey_id: "intent_sandbox_001".to_string(),
        generated_at: timestamp + Duration::minutes(1),
        entries: vec![ReplayTimelineEntry {
            sequence: 1,
            source: ReplayTimelineSource::Webhook,
            reference_id: "evt_sandbox_001".to_string(),
            occurred_at: timestamp,
            label: "Webhook replay".to_string(),
            status: "DELIVERED".to_string(),
            payload: json!({
                "eventType": "intent.status.changed",
                "apiSecret": "ramp_secret_live",
                "webhookSecret": "whsec_live",
                "headers": {
                    "Authorization": "Bearer secret-token"
                },
                "data": {
                    "nestedSecret": "do-not-export",
                    "safeValue": "keep-me"
                }
            }),
        }],
    };

    let redacted = redact_replay_bundle(&bundle);
    let payload = &redacted.entries[0].payload;

    assert_eq!(payload["apiSecret"], "[REDACTED]");
    assert_eq!(payload["webhookSecret"], "[REDACTED]");
    assert_eq!(payload["headers"]["Authorization"], "[REDACTED]");
    assert_eq!(payload["data"]["nestedSecret"], "[REDACTED]");
    assert_eq!(payload["data"]["safeValue"], "keep-me");
}
