use chrono::{Duration, Utc};
use ramp_compliance::rescreening::{
    RescreeningEngine, RescreeningPriority, RescreeningRunStatus, RescreeningSubject,
    RescreeningTriggerKind, RestrictionStatus,
};
use serde_json::json;

#[test]
fn scheduled_rescreening_flags_due_users_and_document_expiry_alerts() {
    let engine = RescreeningEngine::default();
    let now = Utc::now();
    let subjects = vec![
        RescreeningSubject {
            tenant_id: "tenant_due".to_string(),
            user_id: "user_due".to_string(),
            status: "ACTIVE".to_string(),
            kyc_verified_at: Some(now - Duration::days(220)),
            risk_flags: json!({}),
        },
        RescreeningSubject {
            tenant_id: "tenant_doc".to_string(),
            user_id: "user_doc".to_string(),
            status: "ACTIVE".to_string(),
            kyc_verified_at: Some(now - Duration::days(10)),
            risk_flags: json!({
                "rescreening": {
                    "nextRunAt": (now + Duration::days(20)).to_rfc3339(),
                    "documentExpiryAt": (now + Duration::days(7)).to_rfc3339()
                }
            }),
        },
    ];

    let runs = engine.build_scheduled_runs(&subjects, now);
    assert_eq!(runs.len(), 2);
    assert!(runs.iter().any(|run| {
        run.user_id == "user_due"
            && run.trigger_kind == RescreeningTriggerKind::Scheduled
            && run.priority == RescreeningPriority::Medium
    }));
    assert!(runs.iter().any(|run| {
        run.user_id == "user_doc"
            && run.trigger_kind == RescreeningTriggerKind::DocumentExpiry
            && run.status == RescreeningRunStatus::Alerted
            && run.restriction_status == RestrictionStatus::ReviewRequired
    }));
}

#[test]
fn watchlist_delta_escalates_to_restriction_only_for_high_priority_hits() {
    let engine = RescreeningEngine::default();
    let now = Utc::now();
    let subject = RescreeningSubject {
        tenant_id: "tenant_alert".to_string(),
        user_id: "user_alert".to_string(),
        status: "ACTIVE".to_string(),
        kyc_verified_at: Some(now - Duration::days(30)),
        risk_flags: json!({}),
    };

    let critical = engine.evaluate_watchlist_delta(&subject, now, true, false, false);
    assert_eq!(critical.priority, RescreeningPriority::Critical);
    assert_eq!(critical.status, RescreeningRunStatus::Restricted);
    assert_eq!(critical.restriction_status, RestrictionStatus::Restricted);

    let review = engine.evaluate_watchlist_delta(&subject, now, false, true, false);
    assert_eq!(review.priority, RescreeningPriority::High);
    assert_eq!(review.status, RescreeningRunStatus::Alerted);
    assert_eq!(review.restriction_status, RestrictionStatus::ReviewRequired);
}
