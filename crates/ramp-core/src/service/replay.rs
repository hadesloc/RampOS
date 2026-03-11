use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::repository::{settlement::SettlementRow, webhook::WebhookEventRow};
use crate::service::{
    reconciliation::{Discrepancy, ReconciliationReport},
    settlement::Settlement,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ReplayTimelineSource {
    Webhook,
    Settlement,
    Reconciliation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayTimelineEntry {
    pub sequence: usize,
    pub source: ReplayTimelineSource,
    pub reference_id: String,
    pub occurred_at: DateTime<Utc>,
    pub label: String,
    pub status: String,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayBundle {
    pub journey_id: String,
    pub generated_at: DateTime<Utc>,
    pub entries: Vec<ReplayTimelineEntry>,
}

pub struct ReplayBundleAssembler;

impl ReplayBundleAssembler {
    pub fn assemble(
        journey_id: impl Into<String>,
        mut entries: Vec<ReplayTimelineEntry>,
    ) -> ReplayBundle {
        entries.sort_by(|left, right| {
            left.occurred_at
                .cmp(&right.occurred_at)
                .then_with(|| left.source.cmp(&right.source))
                .then_with(|| left.reference_id.cmp(&right.reference_id))
        });

        for (idx, entry) in entries.iter_mut().enumerate() {
            entry.sequence = idx + 1;
        }

        ReplayBundle {
            journey_id: journey_id.into(),
            generated_at: Utc::now(),
            entries,
        }
    }
}

pub fn redact_replay_bundle(bundle: &ReplayBundle) -> ReplayBundle {
    let mut redacted = bundle.clone();
    for entry in &mut redacted.entries {
        redact_value(&mut entry.payload);
    }
    redacted
}

impl ReplayTimelineEntry {
    pub fn from_webhook_event(event: WebhookEventRow) -> Self {
        Self {
            sequence: 0,
            source: ReplayTimelineSource::Webhook,
            reference_id: event.id.clone(),
            occurred_at: event
                .delivered_at
                .or(event.last_attempt_at)
                .unwrap_or(event.created_at),
            label: format!("Webhook {}", event.event_type),
            status: event.status.clone(),
            payload: json!({
                "eventType": event.event_type,
                "intentId": event.intent_id,
                "attempts": event.attempts,
                "responseStatus": event.response_status,
                "lastError": event.last_error,
                "data": event.payload,
            }),
        }
    }

    pub fn from_settlement(settlement: Settlement) -> Self {
        Self {
            sequence: 0,
            source: ReplayTimelineSource::Settlement,
            reference_id: settlement.id.clone(),
            occurred_at: settlement.updated_at,
            label: "Settlement status".to_string(),
            status: settlement.status.as_db_str().to_string(),
            payload: json!({
                "offrampIntentId": settlement.offramp_intent_id,
                "bankReference": settlement.bank_reference,
                "errorMessage": settlement.error_message,
                "createdAt": settlement.created_at.to_rfc3339(),
            }),
        }
    }

    pub fn from_settlement_row(row: SettlementRow) -> Self {
        Self {
            sequence: 0,
            source: ReplayTimelineSource::Settlement,
            reference_id: row.id.clone(),
            occurred_at: row.updated_at,
            label: "Settlement status".to_string(),
            status: row.status.clone(),
            payload: json!({
                "offrampIntentId": row.offramp_intent_id,
                "bankReference": row.bank_reference,
                "errorMessage": row.error_message,
                "createdAt": row.created_at.to_rfc3339(),
            }),
        }
    }

    pub fn from_reconciliation_report(report: &ReconciliationReport) -> Vec<Self> {
        let mut entries = vec![Self {
            sequence: 0,
            source: ReplayTimelineSource::Reconciliation,
            reference_id: report.id.clone(),
            occurred_at: report.completed_at,
            label: "Reconciliation report".to_string(),
            status: format!("{:?}", report.status).to_uppercase(),
            payload: json!({
                "totalSettlementsChecked": report.total_settlements_checked,
                "totalOnChainTxsChecked": report.total_on_chain_txs_checked,
                "totalDiscrepancies": report.total_discrepancies,
                "criticalCount": report.critical_count,
            }),
        }];

        entries.extend(
            report
                .discrepancies
                .iter()
                .map(Self::from_reconciliation_discrepancy),
        );
        entries
    }

    pub fn from_reconciliation_discrepancy(discrepancy: &Discrepancy) -> Self {
        Self {
            sequence: 0,
            source: ReplayTimelineSource::Reconciliation,
            reference_id: discrepancy.id.clone(),
            occurred_at: discrepancy.detected_at,
            label: "Reconciliation discrepancy".to_string(),
            status: format!("{:?}", discrepancy.kind).to_uppercase(),
            payload: json!({
                "settlementId": discrepancy.settlement_id,
                "onChainTx": discrepancy.on_chain_tx,
                "expectedAmount": discrepancy.expected_amount,
                "actualAmount": discrepancy.actual_amount,
                "severity": format!("{:?}", discrepancy.severity).to_uppercase(),
                "details": discrepancy.details,
            }),
        }
    }
}

fn redact_value(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (key, nested) in map.iter_mut() {
                if should_redact_key(key) {
                    *nested = Value::String("[REDACTED]".to_string());
                } else {
                    redact_value(nested);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                redact_value(item);
            }
        }
        _ => {}
    }
}

fn should_redact_key(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase();
    normalized.contains("secret") || normalized == "authorization"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::reconciliation::{
        Discrepancy, DiscrepancyKind, ReconciliationReport, ReconciliationStatus, Severity,
    };
    use crate::test_utils::{sandbox_offramp_fixture, sandbox_payin_fixture};
    use chrono::{Duration, TimeZone};
    use ramp_common::types::{TenantId, UserId};

    #[test]
    fn assembles_ordered_timeline_from_existing_records() {
        let tenant_id = TenantId::new("tenant_sandbox");
        let user_id = UserId::new("user_sandbox");
        let payin = sandbox_payin_fixture(&tenant_id, &user_id);
        let offramp = sandbox_offramp_fixture(&tenant_id, &user_id);
        let base_time = payin.intent.created_at;

        let webhook_entry = ReplayTimelineEntry::from_webhook_event(WebhookEventRow {
            id: "evt_payin_pending".to_string(),
            tenant_id: tenant_id.0.clone(),
            event_type: "intent.status.changed".to_string(),
            intent_id: Some(payin.intent.id.clone()),
            payload: json!({"state": payin.intent.state}),
            status: "DELIVERED".to_string(),
            attempts: 1,
            max_attempts: 10,
            last_attempt_at: Some(base_time + Duration::minutes(1)),
            next_attempt_at: None,
            last_error: None,
            delivered_at: Some(base_time + Duration::minutes(1)),
            response_status: Some(200),
            created_at: base_time,
        });

        let settlement_entry = ReplayTimelineEntry::from_settlement(Settlement {
            id: "stl_sandbox_001".to_string(),
            offramp_intent_id: offramp.intent.id.clone(),
            status: crate::service::settlement::SettlementStatus::Processing,
            bank_reference: Some("RAMP-SBX1".to_string()),
            error_message: None,
            created_at: base_time + Duration::minutes(5),
            updated_at: base_time + Duration::minutes(6),
        });

        let discrepancy = Discrepancy {
            id: "disc_sandbox_001".to_string(),
            kind: DiscrepancyKind::StatusMismatch,
            settlement_id: Some("stl_sandbox_001".to_string()),
            on_chain_tx: Some("0xsandbox".to_string()),
            expected_amount: 100.0,
            actual_amount: 100.0,
            severity: Severity::Critical,
            detected_at: base_time + Duration::minutes(8),
            details: "Confirmation still pending".to_string(),
        };

        let report = ReconciliationReport {
            id: "recon_sandbox_001".to_string(),
            started_at: base_time + Duration::minutes(7),
            completed_at: base_time + Duration::minutes(9),
            total_settlements_checked: 1,
            total_on_chain_txs_checked: 1,
            discrepancies: vec![discrepancy],
            total_discrepancies: 1,
            critical_count: 1,
            status: ReconciliationStatus::CriticalIssues,
        };

        let bundle = ReplayBundleAssembler::assemble(
            payin.intent.id.clone(),
            vec![
                settlement_entry,
                webhook_entry,
                ReplayTimelineEntry::from_reconciliation_report(&report)
                    .into_iter()
                    .last()
                    .unwrap(),
            ],
        );

        assert_eq!(bundle.entries.len(), 3);
        assert_eq!(bundle.entries[0].sequence, 1);
        assert_eq!(bundle.entries[0].source, ReplayTimelineSource::Webhook);
        assert_eq!(bundle.entries[1].source, ReplayTimelineSource::Settlement);
        assert_eq!(
            bundle.entries[2].source,
            ReplayTimelineSource::Reconciliation
        );
        assert_eq!(
            bundle.entries[1].payload["offrampIntentId"],
            offramp.intent.id
        );
    }

    #[test]
    fn reconciliation_report_expands_to_summary_and_discrepancy_entries() {
        let timestamp = Utc.with_ymd_and_hms(2026, 3, 8, 10, 0, 0).single().unwrap();
        let report = ReconciliationReport {
            id: "recon_sandbox_002".to_string(),
            started_at: timestamp,
            completed_at: timestamp + Duration::minutes(2),
            total_settlements_checked: 2,
            total_on_chain_txs_checked: 1,
            discrepancies: vec![Discrepancy {
                id: "disc_sandbox_002".to_string(),
                kind: DiscrepancyKind::AmountMismatch,
                settlement_id: Some("stl_sandbox_002".to_string()),
                on_chain_tx: Some("0xhash".to_string()),
                expected_amount: 120.0,
                actual_amount: 118.5,
                severity: Severity::High,
                detected_at: timestamp + Duration::minutes(1),
                details: "Amounts drifted".to_string(),
            }],
            total_discrepancies: 1,
            critical_count: 0,
            status: ReconciliationStatus::DiscrepanciesFound,
        };

        let entries = ReplayTimelineEntry::from_reconciliation_report(&report);

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].reference_id, "recon_sandbox_002");
        assert_eq!(entries[0].payload["totalDiscrepancies"], 1);
        assert_eq!(entries[1].reference_id, "disc_sandbox_002");
        assert_eq!(entries[1].payload["severity"], "HIGH");
        assert_eq!(entries[1].payload["expectedAmount"], json!(120.0));
        assert_eq!(entries[1].payload["actualAmount"], json!(118.5));
    }
}
