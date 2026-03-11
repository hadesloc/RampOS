use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use ramp_common::{types::TenantId, Result};
use ramp_core::event::InMemoryEventPublisher;
use ramp_core::repository::{LpReliabilitySnapshotRow, RfqBidRow, RfqRepository, RfqRequestRow};
use ramp_core::service::liquidity_reliability::{
    LiquidityReliabilityService, LiquidityReliabilitySnapshot, ReliabilityWindowKind,
};
use ramp_core::service::rfq::{CreateRfqRequest, RfqService, SubmitBidRequest};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use tokio::sync::RwLock;

#[derive(Default)]
struct TestRfqRepository {
    requests: Arc<RwLock<Vec<RfqRequestRow>>>,
    bids: Arc<RwLock<Vec<RfqBidRow>>>,
    snapshots: Arc<RwLock<Vec<LpReliabilitySnapshotRow>>>,
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
                    && direction.is_none_or(|value| request.direction == value)
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

    async fn upsert_reliability_snapshot(&self, snapshot: &LpReliabilitySnapshotRow) -> Result<()> {
        let mut snapshots = self.snapshots.write().await;
        if let Some(existing) = snapshots.iter_mut().find(|row| {
            row.tenant_id == snapshot.tenant_id
                && row.lp_id == snapshot.lp_id
                && row.direction == snapshot.direction
                && row.window_kind == snapshot.window_kind
                && row.window_started_at == snapshot.window_started_at
                && row.window_ended_at == snapshot.window_ended_at
                && row.snapshot_version == snapshot.snapshot_version
        }) {
            *existing = snapshot.clone();
        } else {
            snapshots.push(snapshot.clone());
        }
        Ok(())
    }

    async fn list_reliability_snapshots(
        &self,
        tenant_id: &TenantId,
        lp_id: &str,
        direction: Option<&str>,
        limit: i64,
    ) -> Result<Vec<LpReliabilitySnapshotRow>> {
        let mut snapshots: Vec<_> = self
            .snapshots
            .read()
            .await
            .iter()
            .filter(|row| {
                row.tenant_id == tenant_id.0
                    && row.lp_id == lp_id
                    && direction.is_none_or(|value| row.direction == value)
            })
            .cloned()
            .collect();
        snapshots.sort_by(|left, right| {
            right
                .window_ended_at
                .cmp(&left.window_ended_at)
                .then_with(|| right.updated_at.cmp(&left.updated_at))
        });
        snapshots.truncate(limit as usize);
        Ok(snapshots)
    }

    async fn get_latest_reliability_snapshot(
        &self,
        tenant_id: &TenantId,
        lp_id: &str,
        direction: &str,
        window_kind: &str,
    ) -> Result<Option<LpReliabilitySnapshotRow>> {
        Ok(self
            .list_reliability_snapshots(tenant_id, lp_id, Some(direction), i64::MAX)
            .await?
            .into_iter()
            .find(|row| row.window_kind == window_kind))
    }
}

fn tenant_id() -> TenantId {
    TenantId::new("tenant_liquidity_reliability")
}

fn rfq_service(repo: Arc<TestRfqRepository>) -> RfqService {
    RfqService::new(repo, Arc::new(InMemoryEventPublisher::new()))
}

fn reliability_service(repo: Arc<TestRfqRepository>) -> LiquidityReliabilityService {
    LiquidityReliabilityService::new(repo)
}

async fn create_offramp_rfq(
    service: &RfqService,
    tenant_id: &TenantId,
    user_id: &str,
) -> RfqRequestRow {
    service
        .create_rfq(CreateRfqRequest {
            tenant_id: tenant_id.clone(),
            user_id: user_id.to_string(),
            direction: "OFFRAMP".to_string(),
            offramp_id: None,
            crypto_asset: "USDT".to_string(),
            crypto_amount: dec!(100),
            vnd_amount: None,
            ttl_minutes: 5,
        })
        .await
        .unwrap()
}

async fn submit_bid(
    service: &RfqService,
    tenant_id: &TenantId,
    rfq_id: &str,
    lp_id: &str,
    exchange_rate: Decimal,
) -> RfqBidRow {
    service
        .submit_bid(SubmitBidRequest {
            tenant_id: tenant_id.clone(),
            rfq_id: rfq_id.to_string(),
            lp_id: lp_id.to_string(),
            lp_name: None,
            exchange_rate,
            vnd_amount: exchange_rate * dec!(100),
            valid_minutes: 5,
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn reliability_snapshot_ingestion_accumulates_quote_fill_and_reject_outcomes() {
    let repo = Arc::new(TestRfqRepository::new());
    let rfq_service = rfq_service(repo.clone());
    let reliability_service = reliability_service(repo);
    let tenant_id = tenant_id();

    let first_rfq = create_offramp_rfq(&rfq_service, &tenant_id, "user_one").await;
    submit_bid(
        &rfq_service,
        &tenant_id,
        &first_rfq.id,
        "lp_alpha",
        dec!(25900),
    )
    .await;
    submit_bid(
        &rfq_service,
        &tenant_id,
        &first_rfq.id,
        "lp_beta",
        dec!(25800),
    )
    .await;
    rfq_service
        .finalize_rfq(&tenant_id, &first_rfq.id)
        .await
        .unwrap();

    let second_rfq = create_offramp_rfq(&rfq_service, &tenant_id, "user_two").await;
    submit_bid(
        &rfq_service,
        &tenant_id,
        &second_rfq.id,
        "lp_alpha",
        dec!(25850),
    )
    .await;
    submit_bid(
        &rfq_service,
        &tenant_id,
        &second_rfq.id,
        "lp_gamma",
        dec!(26050),
    )
    .await;
    rfq_service
        .finalize_rfq(&tenant_id, &second_rfq.id)
        .await
        .unwrap();

    let latest = reliability_service
        .latest_snapshot(
            &tenant_id,
            "lp_alpha",
            "OFFRAMP",
            ReliabilityWindowKind::Rolling30d,
        )
        .await
        .unwrap()
        .expect("latest snapshot should exist");

    assert_eq!(latest.quote_count, 2);
    assert_eq!(latest.fill_count, 1);
    assert_eq!(latest.reject_count, 1);
    assert_eq!(latest.fill_rate, dec!(0.5));
    assert_eq!(latest.reject_rate, dec!(0.5));
    assert_eq!(latest.metadata["lastOutcome"], "rfq_rejected");

    let history = reliability_service
        .snapshot_history(&tenant_id, "lp_alpha", Some("OFFRAMP"), 10)
        .await
        .unwrap();

    assert_eq!(history.len(), 4);
    assert_eq!(history[0].id, latest.id);
    assert!(history
        .windows(2)
        .all(|window| { window[0].window_ended_at >= window[1].window_ended_at }));
}

#[tokio::test]
async fn reliability_service_filters_history_by_direction_and_latest_window_kind() {
    let repo = Arc::new(TestRfqRepository::new());
    let reliability_service = reliability_service(repo);
    let tenant_id = tenant_id();
    let now = Utc::now();

    let offramp_snapshot = LiquidityReliabilitySnapshot {
        id: "snap_offramp".to_string(),
        tenant_id: tenant_id.0.clone(),
        lp_id: "lp_windowed".to_string(),
        direction: "OFFRAMP".to_string(),
        window_kind: ReliabilityWindowKind::Rolling24h,
        window_started_at: now - chrono::Duration::hours(24),
        window_ended_at: now,
        snapshot_version: "v1".to_string(),
        quote_count: 5,
        fill_count: 4,
        reject_count: 1,
        settlement_count: 4,
        dispute_count: 0,
        fill_rate: dec!(0.8),
        reject_rate: dec!(0.2),
        dispute_rate: dec!(0),
        avg_slippage_bps: dec!(6),
        p95_settlement_latency_seconds: 240,
        reliability_score: Some(dec!(0.91)),
        metadata: serde_json::json!({ "source": "manual-offramp" }),
    };
    let onramp_snapshot = LiquidityReliabilitySnapshot {
        id: "snap_onramp".to_string(),
        tenant_id: tenant_id.0.clone(),
        lp_id: "lp_windowed".to_string(),
        direction: "ONRAMP".to_string(),
        window_kind: ReliabilityWindowKind::Rolling30d,
        window_started_at: now - chrono::Duration::days(30),
        window_ended_at: now + chrono::Duration::seconds(1),
        snapshot_version: "v1".to_string(),
        quote_count: 7,
        fill_count: 6,
        reject_count: 1,
        settlement_count: 6,
        dispute_count: 0,
        fill_rate: dec!(0.8571428571),
        reject_rate: dec!(0.1428571429),
        dispute_rate: dec!(0),
        avg_slippage_bps: dec!(4),
        p95_settlement_latency_seconds: 180,
        reliability_score: Some(dec!(0.95)),
        metadata: serde_json::json!({ "source": "manual-onramp" }),
    };

    reliability_service
        .upsert_snapshot(&offramp_snapshot)
        .await
        .unwrap();
    reliability_service
        .upsert_snapshot(&onramp_snapshot)
        .await
        .unwrap();

    let offramp_history = reliability_service
        .snapshot_history(&tenant_id, "lp_windowed", Some("OFFRAMP"), 10)
        .await
        .unwrap();
    let onramp_latest = reliability_service
        .latest_snapshot(
            &tenant_id,
            "lp_windowed",
            "ONRAMP",
            ReliabilityWindowKind::Rolling30d,
        )
        .await
        .unwrap()
        .expect("onramp snapshot should exist");

    assert_eq!(offramp_history.len(), 1);
    assert_eq!(offramp_history[0].direction, "OFFRAMP");
    assert_eq!(offramp_history[0].metadata["source"], "manual-offramp");
    assert_eq!(onramp_latest.direction, "ONRAMP");
    assert_eq!(onramp_latest.window_kind, ReliabilityWindowKind::Rolling30d);
    assert_eq!(onramp_latest.reliability_score, Some(dec!(0.95)));
}
