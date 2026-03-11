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
    TenantId::new("tenant_liquidity_policy")
}

fn rfq_service(repo: Arc<TestRfqRepository>) -> RfqService {
    RfqService::new(repo, Arc::new(InMemoryEventPublisher::new()))
}

fn reliability_service(repo: Arc<TestRfqRepository>) -> LiquidityReliabilityService {
    LiquidityReliabilityService::new(repo)
}

async fn create_rfq(service: &RfqService, tenant_id: &TenantId, direction: &str) -> RfqRequestRow {
    service
        .create_rfq(CreateRfqRequest {
            tenant_id: tenant_id.clone(),
            user_id: format!("user_{direction}"),
            direction: direction.to_string(),
            offramp_id: None,
            crypto_asset: "USDT".to_string(),
            crypto_amount: dec!(100),
            vnd_amount: if direction == "ONRAMP" {
                Some(dec!(2600000))
            } else {
                None
            },
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

async fn seed_snapshot(
    service: &LiquidityReliabilityService,
    tenant_id: &TenantId,
    lp_id: &str,
    direction: &str,
    quote_count: i32,
    fill_count: i32,
    reject_count: i32,
    reliability_score: Decimal,
    fill_rate: Decimal,
    reject_rate: Decimal,
    dispute_rate: Decimal,
    avg_slippage_bps: Decimal,
    p95_settlement_latency_seconds: i32,
) {
    let now = Utc::now();
    service
        .upsert_snapshot(&LiquidityReliabilitySnapshot {
            id: format!("snap_{direction}_{lp_id}"),
            tenant_id: tenant_id.0.clone(),
            lp_id: lp_id.to_string(),
            direction: direction.to_string(),
            window_kind: ReliabilityWindowKind::Rolling30d,
            window_started_at: now - chrono::Duration::days(30),
            window_ended_at: now,
            snapshot_version: "v1".to_string(),
            quote_count,
            fill_count,
            reject_count,
            settlement_count: fill_count,
            dispute_count: 0,
            fill_rate,
            reject_rate,
            dispute_rate,
            avg_slippage_bps,
            p95_settlement_latency_seconds,
            reliability_score: Some(reliability_score),
            metadata: serde_json::json!({ "seeded": true }),
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn policy_ranking_prefers_more_reliable_offramp_lp_over_better_price() {
    let repo = Arc::new(TestRfqRepository::new());
    let rfq_service = rfq_service(repo.clone());
    let reliability_service = reliability_service(repo);
    let tenant_id = tenant_id();

    let rfq = create_rfq(&rfq_service, &tenant_id, "OFFRAMP").await;
    let reliable_bid = submit_bid(
        &rfq_service,
        &tenant_id,
        &rfq.id,
        "lp_reliable",
        dec!(25900),
    )
    .await;
    submit_bid(
        &rfq_service,
        &tenant_id,
        &rfq.id,
        "lp_price_only",
        dec!(26000),
    )
    .await;

    seed_snapshot(
        &reliability_service,
        &tenant_id,
        "lp_reliable",
        "OFFRAMP",
        8,
        8,
        0,
        dec!(0.95),
        dec!(0.98),
        dec!(0.01),
        dec!(0.01),
        dec!(4),
        180,
    )
    .await;
    seed_snapshot(
        &reliability_service,
        &tenant_id,
        "lp_price_only",
        "OFFRAMP",
        8,
        4,
        2,
        dec!(0.40),
        dec!(0.55),
        dec!(0.25),
        dec!(0.15),
        dec!(25),
        1800,
    )
    .await;

    let best = rfq_service
        .get_best_bid(&tenant_id, &rfq.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(best.id, reliable_bid.id);
    assert_eq!(best.lp_id, "lp_reliable");
}

#[tokio::test]
async fn policy_falls_back_to_best_price_for_offramp_when_reliability_is_below_threshold() {
    let repo = Arc::new(TestRfqRepository::new());
    let rfq_service = rfq_service(repo.clone());
    let reliability_service = reliability_service(repo);
    let tenant_id = tenant_id();

    let rfq = create_rfq(&rfq_service, &tenant_id, "OFFRAMP").await;
    submit_bid(
        &rfq_service,
        &tenant_id,
        &rfq.id,
        "lp_reliable_but_thin",
        dec!(25900),
    )
    .await;
    let best_price_bid = submit_bid(
        &rfq_service,
        &tenant_id,
        &rfq.id,
        "lp_best_price",
        dec!(26000),
    )
    .await;

    seed_snapshot(
        &reliability_service,
        &tenant_id,
        "lp_reliable_but_thin",
        "OFFRAMP",
        2,
        2,
        0,
        dec!(0.99),
        dec!(1),
        dec!(0),
        dec!(0),
        dec!(1),
        60,
    )
    .await;
    seed_snapshot(
        &reliability_service,
        &tenant_id,
        "lp_best_price",
        "OFFRAMP",
        2,
        0,
        2,
        dec!(0.10),
        dec!(0.2),
        dec!(0.7),
        dec!(0.1),
        dec!(40),
        2400,
    )
    .await;

    let best = rfq_service
        .get_best_bid(&tenant_id, &rfq.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(best.id, best_price_bid.id);
    assert_eq!(best.lp_id, "lp_best_price");
}

#[tokio::test]
async fn policy_falls_back_to_lowest_price_for_onramp_when_reliability_is_missing() {
    let repo = Arc::new(TestRfqRepository::new());
    let rfq_service = rfq_service(repo);
    let tenant_id = tenant_id();

    let rfq = create_rfq(&rfq_service, &tenant_id, "ONRAMP").await;
    submit_bid(
        &rfq_service,
        &tenant_id,
        &rfq.id,
        "lp_expensive",
        dec!(26000),
    )
    .await;
    let cheapest_bid = submit_bid(
        &rfq_service,
        &tenant_id,
        &rfq.id,
        "lp_cheapest",
        dec!(25200),
    )
    .await;

    let best = rfq_service
        .get_best_bid(&tenant_id, &rfq.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(best.id, cheapest_bid.id);
    assert_eq!(best.lp_id, "lp_cheapest");
}
