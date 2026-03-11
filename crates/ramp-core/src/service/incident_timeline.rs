use std::collections::BTreeSet;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::repository::{
    rfq::{RfqBidRow, RfqRequestRow},
    settlement::SettlementRow,
    webhook::WebhookEventRow,
};
use crate::service::{
    metrics::IncidentSignalSnapshot,
    reconciliation::{Discrepancy, ReconciliationReport},
    settlement::Settlement,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum IncidentTimelineSourceKind {
    Webhook,
    Settlement,
    Reconciliation,
    Rfq,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IncidentConfidenceMarker {
    Confirmed,
    Correlated,
    NeedsReview,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IncidentActionMode {
    RecommendationOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IncidentRecommendationPriority {
    Immediate,
    High,
    Normal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IncidentTimelineEntry {
    pub sequence: usize,
    pub source_kind: IncidentTimelineSourceKind,
    pub source_reference_id: String,
    pub occurred_at: DateTime<Utc>,
    pub label: String,
    pub status: String,
    pub confidence: IncidentConfidenceMarker,
    pub related_reference_ids: Vec<String>,
    pub details: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IncidentActionRecommendation {
    pub code: String,
    pub title: String,
    pub summary: String,
    pub confidence: IncidentConfidenceMarker,
    pub priority: IncidentRecommendationPriority,
    pub mode: IncidentActionMode,
    pub related_entry_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IncidentTimeline {
    pub incident_id: String,
    pub generated_at: DateTime<Utc>,
    pub action_mode: IncidentActionMode,
    pub entries: Vec<IncidentTimelineEntry>,
    pub recommendations: Vec<IncidentActionRecommendation>,
}

pub struct IncidentTimelineAssembler;

impl IncidentTimelineEntry {
    pub fn new(
        source_kind: IncidentTimelineSourceKind,
        source_reference_id: impl Into<String>,
        occurred_at: DateTime<Utc>,
        label: impl Into<String>,
        status: impl Into<String>,
        confidence: IncidentConfidenceMarker,
        details: Value,
    ) -> Self {
        Self {
            sequence: 0,
            source_kind,
            source_reference_id: source_reference_id.into(),
            occurred_at,
            label: label.into(),
            status: status.into(),
            confidence,
            related_reference_ids: Vec::new(),
            details,
        }
    }

    pub fn with_related_references(
        mut self,
        related_reference_ids: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.related_reference_ids = related_reference_ids.into_iter().map(Into::into).collect();
        self
    }

    pub fn from_webhook_event(event: WebhookEventRow) -> Self {
        let occurred_at = event
            .delivered_at
            .or(event.last_attempt_at)
            .unwrap_or(event.created_at);

        Self::new(
            IncidentTimelineSourceKind::Webhook,
            event.id.clone(),
            occurred_at,
            format!("Webhook {}", event.event_type),
            event.status.clone(),
            IncidentConfidenceMarker::Confirmed,
            json!({
                "eventType": event.event_type,
                "intentId": event.intent_id,
                "attempts": event.attempts,
                "maxAttempts": event.max_attempts,
                "nextAttemptAt": event.next_attempt_at,
                "lastError": event.last_error,
                "responseStatus": event.response_status,
                "payload": event.payload,
            }),
        )
        .with_related_references(event.intent_id.into_iter())
    }

    pub fn from_settlement(settlement: Settlement) -> Self {
        let mut related = vec![settlement.offramp_intent_id.clone()];
        if let Some(bank_reference) = &settlement.bank_reference {
            related.push(bank_reference.clone());
        }

        Self::new(
            IncidentTimelineSourceKind::Settlement,
            settlement.id.clone(),
            settlement.updated_at,
            "Settlement status",
            settlement.status.as_db_str(),
            IncidentConfidenceMarker::Confirmed,
            json!({
                "offrampIntentId": settlement.offramp_intent_id,
                "bankReference": settlement.bank_reference,
                "errorMessage": settlement.error_message,
                "createdAt": settlement.created_at.to_rfc3339(),
            }),
        )
        .with_related_references(related)
    }

    pub fn from_settlement_row(row: SettlementRow) -> Self {
        let mut related = vec![row.offramp_intent_id.clone()];
        if let Some(bank_reference) = &row.bank_reference {
            related.push(bank_reference.clone());
        }

        Self::new(
            IncidentTimelineSourceKind::Settlement,
            row.id.clone(),
            row.updated_at,
            "Settlement status",
            row.status.clone(),
            IncidentConfidenceMarker::Confirmed,
            json!({
                "offrampIntentId": row.offramp_intent_id,
                "bankReference": row.bank_reference,
                "errorMessage": row.error_message,
                "createdAt": row.created_at.to_rfc3339(),
            }),
        )
        .with_related_references(related)
    }

    pub fn from_reconciliation_report(report: &ReconciliationReport) -> Vec<Self> {
        let mut entries = vec![Self::new(
            IncidentTimelineSourceKind::Reconciliation,
            report.id.clone(),
            report.completed_at,
            "Reconciliation report",
            format!("{:?}", report.status).to_uppercase(),
            IncidentConfidenceMarker::Correlated,
            json!({
                "totalSettlementsChecked": report.total_settlements_checked,
                "totalOnChainTxsChecked": report.total_on_chain_txs_checked,
                "totalDiscrepancies": report.total_discrepancies,
                "criticalCount": report.critical_count,
            }),
        )
        .with_related_references(
            report
                .discrepancies
                .iter()
                .filter_map(|discrepancy| discrepancy.settlement_id.clone()),
        )];

        entries.extend(
            report
                .discrepancies
                .iter()
                .map(Self::from_reconciliation_discrepancy),
        );
        entries
    }

    pub fn from_reconciliation_discrepancy(discrepancy: &Discrepancy) -> Self {
        Self::new(
            IncidentTimelineSourceKind::Reconciliation,
            discrepancy.id.clone(),
            discrepancy.detected_at,
            "Reconciliation discrepancy",
            format!("{:?}", discrepancy.kind).to_uppercase(),
            IncidentConfidenceMarker::NeedsReview,
            json!({
                "settlementId": discrepancy.settlement_id,
                "onChainTx": discrepancy.on_chain_tx,
                "expectedAmount": discrepancy.expected_amount,
                "actualAmount": discrepancy.actual_amount,
                "severity": format!("{:?}", discrepancy.severity).to_uppercase(),
                "details": discrepancy.details,
            }),
        )
        .with_related_references(
            discrepancy
                .settlement_id
                .iter()
                .cloned()
                .chain(discrepancy.on_chain_tx.iter().cloned()),
        )
    }

    pub fn from_rfq_request(request: RfqRequestRow) -> Self {
        Self::new(
            IncidentTimelineSourceKind::Rfq,
            request.id.clone(),
            request.updated_at,
            "RFQ request",
            request.state.clone(),
            IncidentConfidenceMarker::Confirmed,
            json!({
                "direction": request.direction,
                "offrampId": request.offramp_id,
                "cryptoAsset": request.crypto_asset,
                "cryptoAmount": request.crypto_amount,
                "vndAmount": request.vnd_amount,
                "winningBidId": request.winning_bid_id,
                "winningLpId": request.winning_lp_id,
                "finalRate": request.final_rate,
                "expiresAt": request.expires_at.to_rfc3339(),
            }),
        )
        .with_related_references(
            request
                .offramp_id
                .iter()
                .cloned()
                .chain(request.winning_bid_id.iter().cloned())
                .chain(request.winning_lp_id.iter().cloned()),
        )
    }

    pub fn from_rfq_bid(bid: RfqBidRow) -> Self {
        Self::new(
            IncidentTimelineSourceKind::Rfq,
            bid.id.clone(),
            bid.created_at,
            "RFQ bid",
            bid.state.clone(),
            IncidentConfidenceMarker::Confirmed,
            json!({
                "rfqId": bid.rfq_id,
                "lpId": bid.lp_id,
                "lpName": bid.lp_name,
                "exchangeRate": bid.exchange_rate,
                "vndAmount": bid.vnd_amount,
                "validUntil": bid.valid_until.to_rfc3339(),
            }),
        )
        .with_related_references([bid.rfq_id, bid.lp_id])
    }
}

impl IncidentActionRecommendation {
    pub fn recommendation_only(
        code: impl Into<String>,
        title: impl Into<String>,
        summary: impl Into<String>,
        priority: IncidentRecommendationPriority,
        confidence: IncidentConfidenceMarker,
        related_entry_ids: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            code: code.into(),
            title: title.into(),
            summary: summary.into(),
            confidence,
            priority,
            mode: IncidentActionMode::RecommendationOnly,
            related_entry_ids: related_entry_ids.into_iter().map(Into::into).collect(),
        }
    }
}

impl IncidentTimelineAssembler {
    pub fn assemble(
        incident_id: impl Into<String>,
        entries: Vec<IncidentTimelineEntry>,
        recommendations: Vec<IncidentActionRecommendation>,
    ) -> IncidentTimeline {
        Self::assemble_with_signals(
            incident_id,
            entries,
            recommendations,
            IncidentSignalSnapshot::default(),
        )
    }

    pub fn assemble_with_signals(
        incident_id: impl Into<String>,
        mut entries: Vec<IncidentTimelineEntry>,
        recommendations: Vec<IncidentActionRecommendation>,
        signals: IncidentSignalSnapshot,
    ) -> IncidentTimeline {
        entries.sort_by(|left, right| {
            left.occurred_at
                .cmp(&right.occurred_at)
                .then_with(|| left.source_kind.cmp(&right.source_kind))
                .then_with(|| left.source_reference_id.cmp(&right.source_reference_id))
        });

        for (index, entry) in entries.iter_mut().enumerate() {
            entry.sequence = index + 1;
            entry.related_reference_ids =
                dedupe_related_reference_ids(&entry.related_reference_ids);
        }

        let mut recommendations: Vec<_> = recommendations
            .into_iter()
            .map(|mut recommendation| {
                recommendation.related_entry_ids =
                    dedupe_related_reference_ids(&recommendation.related_entry_ids);
                recommendation
            })
            .collect();
        let mut recommendation_codes: BTreeSet<String> =
            recommendations.iter().map(|rec| rec.code.clone()).collect();
        for recommendation in build_guarded_recommendations(&entries, &signals) {
            if recommendation_codes.insert(recommendation.code.clone()) {
                recommendations.push(recommendation);
            }
        }

        IncidentTimeline {
            incident_id: incident_id.into(),
            generated_at: Utc::now(),
            action_mode: IncidentActionMode::RecommendationOnly,
            entries,
            recommendations,
        }
    }
}

fn dedupe_related_reference_ids(ids: &[String]) -> Vec<String> {
    let mut deduped = Vec::new();
    for id in ids {
        if !deduped.contains(id) {
            deduped.push(id.clone());
        }
    }
    deduped
}

fn build_guarded_recommendations(
    entries: &[IncidentTimelineEntry],
    signals: &IncidentSignalSnapshot,
) -> Vec<IncidentActionRecommendation> {
    let mut recommendations = Vec::new();

    let webhook_ids = matching_entry_ids(entries, IncidentTimelineSourceKind::Webhook);
    let settlement_ids = matching_entry_ids(entries, IncidentTimelineSourceKind::Settlement);
    let reconciliation_ids =
        matching_entry_ids(entries, IncidentTimelineSourceKind::Reconciliation);

    let webhook_signal = entries.iter().any(|entry| {
        entry.source_kind == IncidentTimelineSourceKind::Webhook
            && entry.status.to_ascii_uppercase().contains("FAIL")
    });
    if webhook_signal || signals.failed_webhooks > 0 {
        recommendations.push(IncidentActionRecommendation::recommendation_only(
            "review_webhook_delivery",
            "Review webhook delivery",
            if signals.failed_webhooks > 0 {
                format!(
                    "Webhook delivery failures are currently elevated ({} observed); validate endpoint health before replay.",
                    signals.failed_webhooks
                )
            } else {
                "Validate endpoint health before replaying webhook delivery.".to_string()
            },
            IncidentRecommendationPriority::High,
            signal_confidence(webhook_signal, signals.failed_webhooks > 0),
            webhook_ids.clone(),
        ));
    }

    let settlement_processing_signal = entries.iter().any(|entry| {
        entry.source_kind == IncidentTimelineSourceKind::Settlement
            && entry.status.to_ascii_uppercase().contains("PROCESSING")
    });
    let settlement_failure_signal = entries.iter().any(|entry| {
        entry.source_kind == IncidentTimelineSourceKind::Settlement
            && entry.status.to_ascii_uppercase().contains("FAIL")
    });
    if settlement_processing_signal
        || settlement_failure_signal
        || signals.processing_settlements > 0
        || signals.failed_settlements > 0
    {
        let summary = if settlement_failure_signal || signals.failed_settlements > 0 {
            format!(
                "Settlement failure signals are present ({} failed, {} processing); confirm bank rail state before any manual retry.",
                signals.failed_settlements,
                signals.processing_settlements
            )
        } else {
            format!(
                "Settlements are still processing ({} observed); wait for rail confirmation before changing intent state.",
                signals.processing_settlements.max(1)
            )
        };

        recommendations.push(IncidentActionRecommendation::recommendation_only(
            "review_settlement_state",
            "Review settlement state",
            summary,
            IncidentRecommendationPriority::Immediate,
            signal_confidence(
                settlement_processing_signal || settlement_failure_signal,
                signals.processing_settlements > 0 || signals.failed_settlements > 0,
            ),
            settlement_ids.clone(),
        ));
    }

    let reconciliation_signal = entries.iter().any(|entry| {
        entry.source_kind == IncidentTimelineSourceKind::Reconciliation
            && entry.status.to_ascii_uppercase().contains("CRITICAL")
    });
    if reconciliation_signal {
        recommendations.push(IncidentActionRecommendation::recommendation_only(
            "inspect_reconciliation_mismatch",
            "Inspect reconciliation mismatch",
            "Cross-check settlement and on-chain evidence before taking any operator action."
                .to_string(),
            IncidentRecommendationPriority::Immediate,
            IncidentConfidenceMarker::Confirmed,
            reconciliation_ids,
        ));
    }

    if signals.critical_fraud_signals > 0 {
        recommendations.push(IncidentActionRecommendation::recommendation_only(
            "keep_risk_review_in_loop",
            "Keep risk review in the loop",
            format!(
                "Critical fraud signals are active ({} observed); keep actions recommendation-only until operator review completes.",
                signals.critical_fraud_signals
            ),
            IncidentRecommendationPriority::High,
            IncidentConfidenceMarker::Confirmed,
            Vec::<String>::new(),
        ));
    }

    recommendations
}

fn signal_confidence(has_entry_signal: bool, has_metric_signal: bool) -> IncidentConfidenceMarker {
    match (has_entry_signal, has_metric_signal) {
        (true, true) => IncidentConfidenceMarker::Confirmed,
        (true, false) | (false, true) => IncidentConfidenceMarker::Correlated,
        (false, false) => IncidentConfidenceMarker::NeedsReview,
    }
}

fn matching_entry_ids(
    entries: &[IncidentTimelineEntry],
    source_kind: IncidentTimelineSourceKind,
) -> Vec<String> {
    entries
        .iter()
        .filter(|entry| entry.source_kind == source_kind)
        .map(|entry| entry.source_reference_id.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::settlement::SettlementStatus;
    use chrono::{Duration, TimeZone};
    use rust_decimal::Decimal;
    use serde_json::json;

    #[test]
    fn assembles_correlated_entries_in_recommendation_only_mode() {
        let base_time = Utc.with_ymd_and_hms(2026, 3, 8, 9, 0, 0).single().unwrap();

        let webhook_entry = IncidentTimelineEntry::new(
            IncidentTimelineSourceKind::Webhook,
            "evt_001",
            base_time + Duration::minutes(2),
            "Webhook intent.status.changed",
            "DELIVERED",
            IncidentConfidenceMarker::Confirmed,
            json!({ "intentId": "intent_001" }),
        )
        .with_related_references(["stl_001", "rfq_001"]);

        let settlement_entry = IncidentTimelineEntry::new(
            IncidentTimelineSourceKind::Settlement,
            "stl_001",
            base_time + Duration::minutes(1),
            "Settlement status",
            "PROCESSING",
            IncidentConfidenceMarker::Confirmed,
            json!({ "offrampIntentId": "ofr_001" }),
        )
        .with_related_references(["evt_001"]);

        let recommendation = IncidentActionRecommendation::recommendation_only(
            "review_settlement_processing",
            "Review settlement processing",
            "Inspect the bank partner status before replaying any webhook delivery.",
            IncidentRecommendationPriority::Immediate,
            IncidentConfidenceMarker::Confirmed,
            ["stl_001", "evt_001"],
        );

        let timeline = IncidentTimelineAssembler::assemble(
            "incident_001",
            vec![webhook_entry, settlement_entry],
            vec![recommendation],
        );

        assert_eq!(timeline.incident_id, "incident_001");
        assert_eq!(timeline.action_mode, IncidentActionMode::RecommendationOnly);
        assert_eq!(timeline.entries.len(), 2);
        assert_eq!(timeline.entries[0].sequence, 1);
        assert_eq!(timeline.entries[0].source_reference_id, "stl_001");
        assert_eq!(timeline.entries[1].sequence, 2);
        assert_eq!(timeline.entries[1].source_reference_id, "evt_001");
        assert_eq!(
            timeline.recommendations[0].mode,
            IncidentActionMode::RecommendationOnly
        );
        assert_eq!(
            timeline.recommendations[0].confidence,
            IncidentConfidenceMarker::Confirmed
        );
        assert_eq!(
            timeline.recommendations[0].related_entry_ids,
            vec!["stl_001".to_string(), "evt_001".to_string()]
        );
    }

    #[test]
    fn guarded_recommendations_use_metrics_snapshot_to_raise_confidence() {
        let base_time = Utc.with_ymd_and_hms(2026, 3, 8, 12, 0, 0).single().unwrap();
        let entries = vec![
            IncidentTimelineEntry::new(
                IncidentTimelineSourceKind::Webhook,
                "evt_guard_001",
                base_time,
                "Webhook delivery failed",
                "FAILED",
                IncidentConfidenceMarker::Confirmed,
                json!({ "intentId": "intent_guard_001" }),
            ),
            IncidentTimelineEntry::new(
                IncidentTimelineSourceKind::Settlement,
                "stl_guard_001",
                base_time + Duration::minutes(1),
                "Settlement processing",
                "PROCESSING",
                IncidentConfidenceMarker::Confirmed,
                json!({ "offrampIntentId": "intent_guard_001" }),
            ),
        ];

        let timeline = IncidentTimelineAssembler::assemble_with_signals(
            "incident_guard_001",
            entries,
            Vec::new(),
            IncidentSignalSnapshot {
                processing_settlements: 2,
                failed_settlements: 0,
                failed_webhooks: 1,
                critical_fraud_signals: 1,
            },
        );

        assert!(timeline
            .recommendations
            .iter()
            .any(|rec| rec.code == "review_webhook_delivery"
                && rec.confidence == IncidentConfidenceMarker::Confirmed));
        assert!(
            timeline
                .recommendations
                .iter()
                .any(|rec| rec.code == "review_settlement_state"
                    && rec.summary.contains("processing"))
        );
        assert!(timeline
            .recommendations
            .iter()
            .any(|rec| rec.code == "keep_risk_review_in_loop"
                && rec.mode == IncidentActionMode::RecommendationOnly));
    }

    #[test]
    fn builds_entries_from_existing_webhook_settlement_reconciliation_and_rfq_terms() {
        let base_time = Utc.with_ymd_and_hms(2026, 3, 8, 11, 0, 0).single().unwrap();

        let webhook_entry = IncidentTimelineEntry::from_webhook_event(WebhookEventRow {
            id: "evt_002".to_string(),
            tenant_id: "tenant_001".to_string(),
            event_type: "rfq.matched".to_string(),
            intent_id: Some("intent_002".to_string()),
            payload: json!({ "state": "MATCHED" }),
            status: "FAILED".to_string(),
            attempts: 3,
            max_attempts: 10,
            last_attempt_at: Some(base_time + Duration::minutes(1)),
            next_attempt_at: Some(base_time + Duration::minutes(6)),
            last_error: Some("upstream timeout".to_string()),
            delivered_at: None,
            response_status: Some(504),
            created_at: base_time,
        });

        let settlement_entry = IncidentTimelineEntry::from_settlement(Settlement {
            id: "stl_002".to_string(),
            offramp_intent_id: "ofr_002".to_string(),
            status: SettlementStatus::Failed,
            bank_reference: Some("RAMP-FAIL2".to_string()),
            error_message: Some("bank partner timeout".to_string()),
            created_at: base_time + Duration::minutes(2),
            updated_at: base_time + Duration::minutes(3),
        });

        let report = ReconciliationReport {
            id: "recon_002".to_string(),
            started_at: base_time + Duration::minutes(4),
            completed_at: base_time + Duration::minutes(5),
            total_settlements_checked: 1,
            total_on_chain_txs_checked: 1,
            discrepancies: vec![Discrepancy {
                id: "disc_002".to_string(),
                kind: crate::service::reconciliation::DiscrepancyKind::StatusMismatch,
                settlement_id: Some("stl_002".to_string()),
                on_chain_tx: Some("0xdeadbeef".to_string()),
                expected_amount: 500.0,
                actual_amount: 500.0,
                severity: crate::service::reconciliation::Severity::Critical,
                detected_at: base_time + Duration::minutes(5),
                details: "Settlement marked failed while webhook retries continue".to_string(),
            }],
            total_discrepancies: 1,
            critical_count: 1,
            status: crate::service::reconciliation::ReconciliationStatus::CriticalIssues,
        };

        let rfq_entry = IncidentTimelineEntry::from_rfq_request(RfqRequestRow {
            id: "rfq_002".to_string(),
            tenant_id: "tenant_001".to_string(),
            user_id: "user_001".to_string(),
            direction: "OFFRAMP".to_string(),
            offramp_id: Some("ofr_002".to_string()),
            crypto_asset: "USDT".to_string(),
            crypto_amount: Decimal::new(500, 0),
            vnd_amount: None,
            state: "MATCHED".to_string(),
            winning_bid_id: Some("bid_002".to_string()),
            winning_lp_id: Some("lp_002".to_string()),
            final_rate: Some(Decimal::new(25_900, 0)),
            expires_at: base_time + Duration::minutes(7),
            created_at: base_time + Duration::minutes(1),
            updated_at: base_time + Duration::minutes(6),
        });

        let rfq_bid_entry = IncidentTimelineEntry::from_rfq_bid(RfqBidRow {
            id: "bid_002".to_string(),
            rfq_id: "rfq_002".to_string(),
            tenant_id: "tenant_001".to_string(),
            lp_id: "lp_002".to_string(),
            lp_name: Some("LP 2".to_string()),
            exchange_rate: Decimal::new(25_900, 0),
            vnd_amount: Decimal::new(12_950_000, 0),
            valid_until: base_time + Duration::minutes(7),
            state: "ACCEPTED".to_string(),
            created_at: base_time + Duration::minutes(2),
        });

        let reconciliation_entries = IncidentTimelineEntry::from_reconciliation_report(&report);

        assert_eq!(
            webhook_entry.source_kind,
            IncidentTimelineSourceKind::Webhook
        );
        assert_eq!(
            webhook_entry.confidence,
            IncidentConfidenceMarker::Confirmed
        );
        assert_eq!(webhook_entry.details["eventType"], "rfq.matched");
        assert_eq!(
            settlement_entry.source_kind,
            IncidentTimelineSourceKind::Settlement
        );
        assert_eq!(settlement_entry.details["offrampIntentId"], "ofr_002");
        assert_eq!(reconciliation_entries.len(), 2);
        assert_eq!(
            reconciliation_entries[0].source_kind,
            IncidentTimelineSourceKind::Reconciliation
        );
        assert_eq!(
            reconciliation_entries[1].confidence,
            IncidentConfidenceMarker::NeedsReview
        );
        assert_eq!(rfq_entry.source_kind, IncidentTimelineSourceKind::Rfq);
        assert_eq!(rfq_entry.details["direction"], "OFFRAMP");
        assert_eq!(rfq_bid_entry.details["rfqId"], "rfq_002");
    }
}
