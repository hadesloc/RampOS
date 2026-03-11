use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use ramp_common::{
    types::{IntentId, TenantId},
    Result,
};
use ramp_core::event::InMemoryEventPublisher;
use ramp_core::repository::rfq::{RfqBidRow, RfqRepository, RfqRequestRow};
use ramp_core::repository::{InMemorySettlementRepository, WebhookRepository};
use ramp_core::service::{
    rfq::{CreateRfqRequest, RfqService, SubmitBidRequest},
    webhook::WebhookService,
    IncidentActionMode, IncidentRecommendationPriority, IncidentTimelineAssembler,
    IncidentTimelineSourceKind, MetricsRegistry, SettlementService,
};
use ramp_core::test_utils::{MockTenantRepository, MockWebhookRepository};
use rust_decimal::Decimal;
use serde_json::json;
use tokio::sync::RwLock;

#[derive(Default)]
struct TestRfqRepository {
    requests: Arc<RwLock<Vec<RfqRequestRow>>>,
    bids: Arc<RwLock<Vec<RfqBidRow>>>,
}

impl TestRfqRepository {
    fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl RfqRepository for TestRfqRepository {
    async fn create_request(&self, req: &RfqRequestRow) -> Result<()> {
        self.requests.write().await.push(req.clone());
        Ok(())
    }

    async fn get_request(&self, _tenant_id: &TenantId, id: &str) -> Result<Option<RfqRequestRow>> {
        Ok(self
            .requests
            .read()
            .await
            .iter()
            .find(|request| request.id == id)
            .cloned())
    }

    async fn list_open_requests(
        &self,
        tenant_id: &TenantId,
        direction: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<RfqRequestRow>> {
        Ok(self
            .requests
            .read()
            .await
            .iter()
            .filter(|request| {
                request.tenant_id == tenant_id.0
                    && request.state == "OPEN"
                    && direction.map_or(true, |value| request.direction == value)
            })
            .skip(offset as usize)
            .take(limit as usize)
            .cloned()
            .collect())
    }

    async fn update_request(&self, req: &RfqRequestRow) -> Result<()> {
        let mut requests = self.requests.write().await;
        if let Some(existing) = requests.iter_mut().find(|request| request.id == req.id) {
            *existing = req.clone();
        }
        Ok(())
    }

    async fn create_bid(&self, bid: &RfqBidRow) -> Result<()> {
        self.bids.write().await.push(bid.clone());
        Ok(())
    }

    async fn list_bids_for_request(
        &self,
        _tenant_id: &TenantId,
        rfq_id: &str,
    ) -> Result<Vec<RfqBidRow>> {
        Ok(self
            .bids
            .read()
            .await
            .iter()
            .filter(|bid| bid.rfq_id == rfq_id)
            .cloned()
            .collect())
    }

    async fn get_best_bid(
        &self,
        _tenant_id: &TenantId,
        rfq_id: &str,
        direction: &str,
    ) -> Result<Option<RfqBidRow>> {
        let now = Utc::now();
        let mut bids: Vec<_> = self
            .bids
            .read()
            .await
            .iter()
            .filter(|bid| bid.rfq_id == rfq_id && bid.state == "PENDING" && bid.valid_until > now)
            .cloned()
            .collect();

        if direction == "ONRAMP" {
            bids.sort_by(|left, right| left.exchange_rate.cmp(&right.exchange_rate));
        } else {
            bids.sort_by(|left, right| right.exchange_rate.cmp(&left.exchange_rate));
        }

        Ok(bids.into_iter().next())
    }

    async fn update_bid_state(
        &self,
        _tenant_id: &TenantId,
        bid_id: &str,
        state: &str,
    ) -> Result<()> {
        let mut bids = self.bids.write().await;
        if let Some(existing) = bids.iter_mut().find(|bid| bid.id == bid_id) {
            existing.state = state.to_string();
        }
        Ok(())
    }

    async fn upsert_reliability_snapshot(
        &self,
        _snapshot: &ramp_core::repository::LpReliabilitySnapshotRow,
    ) -> Result<()> {
        Ok(())
    }

    async fn list_reliability_snapshots(
        &self,
        _tenant_id: &TenantId,
        _lp_id: &str,
        _direction: Option<&str>,
        _limit: i64,
    ) -> Result<Vec<ramp_core::repository::LpReliabilitySnapshotRow>> {
        Ok(Vec::new())
    }

    async fn get_latest_reliability_snapshot(
        &self,
        _tenant_id: &TenantId,
        _lp_id: &str,
        _direction: &str,
        _window_kind: &str,
    ) -> Result<Option<ramp_core::repository::LpReliabilitySnapshotRow>> {
        Ok(None)
    }
}

#[tokio::test]
async fn incident_timeline_correlates_core_signals_and_surfaces_guarded_recommendations() {
    let tenant_id = TenantId::new("tenant_incident_e2e");
    let intent_id = IntentId::new("intent_incident_e2e");

    let webhook_repo = Arc::new(MockWebhookRepository::new());
    webhook_repo
        .queue_event(&ramp_core::repository::webhook::WebhookEventRow {
            id: "evt_incident_e2e".to_string(),
            tenant_id: tenant_id.0.clone(),
            event_type: "intent.status.changed".to_string(),
            intent_id: Some(intent_id.0.clone()),
            payload: json!({
                "intentId": intent_id.0,
                "newStatus": "FUNDS_PENDING",
            }),
            status: "FAILED".to_string(),
            attempts: 1,
            max_attempts: 10,
            last_attempt_at: Some(Utc::now()),
            next_attempt_at: Some(Utc::now()),
            last_error: Some("destination timeout".to_string()),
            delivered_at: None,
            response_status: Some(504),
            created_at: Utc::now(),
        })
        .await
        .unwrap();

    let webhook_service =
        WebhookService::new(webhook_repo.clone(), Arc::new(MockTenantRepository::new())).unwrap();

    let settlement_service =
        SettlementService::with_repository(Arc::new(InMemorySettlementRepository::new()));
    let settlement = settlement_service
        .trigger_settlement_async(&intent_id.0)
        .await
        .unwrap();

    let rfq_service = RfqService::new(
        Arc::new(TestRfqRepository::new()),
        Arc::new(InMemoryEventPublisher::new()),
    );
    let rfq = rfq_service
        .create_rfq(CreateRfqRequest {
            tenant_id: tenant_id.clone(),
            user_id: "user_incident_e2e".to_string(),
            direction: "OFFRAMP".to_string(),
            offramp_id: Some(intent_id.0.clone()),
            crypto_asset: "USDT".to_string(),
            crypto_amount: Decimal::new(250, 0),
            vnd_amount: None,
            ttl_minutes: 5,
        })
        .await
        .unwrap();
    let bid = rfq_service
        .submit_bid(SubmitBidRequest {
            tenant_id: tenant_id.clone(),
            rfq_id: rfq.id.clone(),
            lp_id: "lp_incident_e2e".to_string(),
            lp_name: Some("LP Incident".to_string()),
            exchange_rate: Decimal::new(25_800, 0),
            vnd_amount: Decimal::new(6_450_000, 0),
            valid_minutes: 5,
        })
        .await
        .unwrap();

    let metrics = MetricsRegistry::new();
    metrics.record_webhook("fail");
    metrics.record_settlement("PROCESSING");
    metrics.record_fraud_score(0.99);

    let mut entries = webhook_service
        .incident_timeline_entries_for_intent(&tenant_id, &intent_id)
        .await
        .unwrap();
    entries.extend(
        settlement_service
            .incident_timeline_entries_for_offramp_async(&intent_id.0)
            .await
            .unwrap(),
    );
    entries.extend(
        rfq_service
            .incident_timeline_entries_for_request(&tenant_id, &rfq.id)
            .await
            .unwrap(),
    );

    let timeline = IncidentTimelineAssembler::assemble_with_signals(
        format!("incident_intent_{}", intent_id.0),
        entries,
        Vec::new(),
        metrics.incident_signal_snapshot(),
    );

    assert_eq!(timeline.action_mode, IncidentActionMode::RecommendationOnly);
    assert!(timeline.entries.iter().any(|entry| {
        entry.source_kind == IncidentTimelineSourceKind::Webhook
            && entry.source_reference_id == "evt_incident_e2e"
    }));
    assert!(timeline.entries.iter().any(|entry| {
        entry.source_kind == IncidentTimelineSourceKind::Settlement
            && entry.source_reference_id == settlement.id
    }));
    assert!(timeline.entries.iter().any(|entry| {
        entry.source_kind == IncidentTimelineSourceKind::Rfq && entry.source_reference_id == rfq.id
    }));
    assert!(timeline.entries.iter().any(|entry| {
        entry.source_kind == IncidentTimelineSourceKind::Rfq && entry.source_reference_id == bid.id
    }));
    assert!(timeline.recommendations.iter().any(|recommendation| {
        recommendation.code == "review_webhook_delivery"
            && recommendation.priority == IncidentRecommendationPriority::High
    }));
    assert!(timeline
        .recommendations
        .iter()
        .any(|recommendation| recommendation.code == "keep_risk_review_in_loop"));
}
