//! RFQ (Request For Quote) Service
//!
//! Orchestrates the bidirectional auction mechanism for competitive pricing:
//! - Off-ramp (USDT→VND): LP competing to pay most VND, user picks best rate
//! - On-ramp (VND→USDT): LP competing to sell cheapest, user picks lowest rate

use chrono::Utc;
use ramp_common::{types::TenantId, Error, Result};
use rust_decimal::Decimal;
use serde_json::json;
use std::sync::Arc;
use tracing::{info, instrument, warn};

use crate::event::EventPublisher;
use crate::repository::rfq::{LpReliabilitySnapshotRow, RfqBidRow, RfqRepository, RfqRequestRow};
use crate::service::incident_timeline::IncidentTimelineEntry;
use crate::service::liquidity_policy::{
    LiquidityPolicyCandidate, LiquidityPolicyConfig, LiquidityPolicyDirection,
    LiquidityPolicyEvaluator, LiquidityPolicyWeights,
};

pub fn lp_counterparty_pressure_score(snapshot: &LpReliabilitySnapshotRow) -> Decimal {
    let reliability = snapshot.reliability_score.unwrap_or(Decimal::from(75));
    let dispute_penalty = snapshot.dispute_rate * Decimal::from(100);
    let reject_penalty = snapshot.reject_rate * Decimal::from(100);
    let latency_penalty = Decimal::from(snapshot.p95_settlement_latency_seconds / 60);
    (Decimal::from(100) - reliability + dispute_penalty + reject_penalty + latency_penalty)
        .round_dp(2)
}

pub fn lp_counterparty_pressure_label(score: &Decimal) -> &'static str {
    if *score >= Decimal::from(55) {
        "high"
    } else if *score >= Decimal::from(30) {
        "medium"
    } else {
        "low"
    }
}

// ============================================================================
// Input / Output structs
// ============================================================================

pub struct CreateRfqRequest {
    pub tenant_id: TenantId,
    pub user_id: String,
    pub direction: String,          // "OFFRAMP" | "ONRAMP"
    pub offramp_id: Option<String>, // for OFFRAMP, link to existing offramp_intent
    pub crypto_asset: String,
    pub crypto_amount: Decimal,      // for OFFRAMP: amount to sell
    pub vnd_amount: Option<Decimal>, // for ONRAMP: budget in VND
    pub ttl_minutes: i64,            // how long LPs have to submit bids
}

pub struct SubmitBidRequest {
    pub tenant_id: TenantId,
    pub rfq_id: String,
    pub lp_id: String,
    pub lp_name: Option<String>,
    pub exchange_rate: Decimal, // VND per 1 unit of crypto
    pub vnd_amount: Decimal,    // total VND in the deal
    pub valid_minutes: i64,     // how long this bid stays valid
}

pub struct FinalizeResult {
    pub rfq: RfqRequestRow,
    pub winning_bid: RfqBidRow,
}

#[derive(Debug, Clone)]
pub struct CounterpartyExposureSignal {
    pub counterparty_id: String,
    pub asset: String,
    pub gross_exposure: Decimal,
    pub won_count: i32,
    pub quote_count: i32,
    pub reliability_score: Option<Decimal>,
    pub dispute_rate: Option<Decimal>,
}

// ============================================================================
// Service
// ============================================================================

pub struct RfqService {
    rfq_repo: Arc<dyn RfqRepository>,
    event_publisher: Arc<dyn EventPublisher>,
}

impl RfqService {
    pub fn new(rfq_repo: Arc<dyn RfqRepository>, event_publisher: Arc<dyn EventPublisher>) -> Self {
        Self {
            rfq_repo,
            event_publisher,
        }
    }

    /// User creates an RFQ → broadcasts to the market.
    /// LPs will receive a webhook event and may respond with bids.
    #[instrument(skip(self, req), fields(
        tenant_id = %req.tenant_id.0,
        user_id = %req.user_id,
        direction = %req.direction
    ))]
    pub async fn create_rfq(&self, req: CreateRfqRequest) -> Result<RfqRequestRow> {
        // Validate direction
        if req.direction != "OFFRAMP" && req.direction != "ONRAMP" {
            return Err(Error::Validation(format!(
                "Invalid direction '{}'. Must be OFFRAMP or ONRAMP",
                req.direction
            )));
        }

        // Validate amounts
        if req.crypto_amount <= Decimal::ZERO {
            return Err(Error::Validation(
                "crypto_amount must be positive".to_string(),
            ));
        }
        if req.ttl_minutes < 1 || req.ttl_minutes > 60 {
            return Err(Error::Validation(
                "ttl_minutes must be between 1 and 60".to_string(),
            ));
        }

        let now = Utc::now();
        let rfq_id = format!("rfq_{}", uuid::Uuid::now_v7());

        let rfq = RfqRequestRow {
            id: rfq_id.clone(),
            tenant_id: req.tenant_id.0.clone(),
            user_id: req.user_id.clone(),
            direction: req.direction.clone(),
            offramp_id: req.offramp_id,
            crypto_asset: req.crypto_asset.clone(),
            crypto_amount: req.crypto_amount,
            vnd_amount: req.vnd_amount,
            state: "OPEN".to_string(),
            winning_bid_id: None,
            winning_lp_id: None,
            final_rate: None,
            expires_at: now + chrono::Duration::minutes(req.ttl_minutes),
            created_at: now,
            updated_at: now,
        };

        self.rfq_repo.create_request(&rfq).await?;

        // Notify LPs via event
        if let Err(e) = self
            .event_publisher
            .publish_rfq_created(&rfq_id, &req.tenant_id)
            .await
        {
            warn!(
                rfq_id = %rfq_id,
                error = %e,
                "Failed to publish rfq_created event (non-fatal)"
            );
        }

        info!(
            rfq_id = %rfq_id,
            direction = %req.direction,
            asset = %req.crypto_asset,
            "RFQ created"
        );

        Ok(rfq)
    }

    /// A Liquidity Provider submits a bid for an open RFQ.
    #[instrument(skip(self, req), fields(
        tenant_id = %req.tenant_id.0,
        rfq_id = %req.rfq_id,
        lp_id = %req.lp_id
    ))]
    pub async fn submit_bid(&self, req: SubmitBidRequest) -> Result<RfqBidRow> {
        // Verify RFQ exists and is still open
        let rfq = self
            .rfq_repo
            .get_request(&req.tenant_id, &req.rfq_id)
            .await?
            .ok_or_else(|| Error::NotFound(format!("RFQ {} not found", req.rfq_id)))?;

        if rfq.state != "OPEN" {
            return Err(Error::Conflict(format!(
                "RFQ {} is not open (current state: {})",
                req.rfq_id, rfq.state
            )));
        }

        if Utc::now() >= rfq.expires_at {
            return Err(Error::Gone(format!("RFQ {} has expired", req.rfq_id)));
        }

        if req.exchange_rate <= Decimal::ZERO {
            return Err(Error::Validation(
                "exchange_rate must be positive".to_string(),
            ));
        }

        let now = Utc::now();
        let bid_id = format!("bid_{}", uuid::Uuid::now_v7());

        let bid = RfqBidRow {
            id: bid_id.clone(),
            rfq_id: req.rfq_id.clone(),
            tenant_id: req.tenant_id.0.clone(),
            lp_id: req.lp_id.clone(),
            lp_name: req.lp_name,
            exchange_rate: req.exchange_rate,
            vnd_amount: req.vnd_amount,
            valid_until: now + chrono::Duration::minutes(req.valid_minutes.max(1)),
            state: "PENDING".to_string(),
            created_at: now,
        };

        self.rfq_repo.create_bid(&bid).await?;
        self.ingest_quote_outcome(&req.tenant_id, &rfq.direction, &bid)
            .await?;

        info!(
            bid_id = %bid_id,
            rfq_id = %req.rfq_id,
            lp_id = %req.lp_id,
            rate = %req.exchange_rate,
            "RFQ bid submitted"
        );

        Ok(bid)
    }

    /// Retrieve the current best bid for an RFQ without finalizing.
    /// Useful for showing user the best available rate before they accept.
    pub async fn get_best_bid(
        &self,
        tenant_id: &TenantId,
        rfq_id: &str,
    ) -> Result<Option<RfqBidRow>> {
        let rfq = self
            .rfq_repo
            .get_request(tenant_id, rfq_id)
            .await?
            .ok_or_else(|| Error::NotFound(format!("RFQ {} not found", rfq_id)))?;

        self.select_best_bid(tenant_id, &rfq).await
    }

    /// Build bounded exposure signals for treasury planning using existing RFQ outcomes.
    pub fn build_counterparty_exposure_signals(
        requests: &[RfqRequestRow],
        bids: &[RfqBidRow],
        snapshots: &[LpReliabilitySnapshotRow],
    ) -> Vec<CounterpartyExposureSignal> {
        let mut exposures: std::collections::HashMap<String, CounterpartyExposureSignal> =
            std::collections::HashMap::new();

        for bid in bids {
            let request = requests.iter().find(|row| row.id == bid.rfq_id);
            let entry = exposures
                .entry(bid.lp_id.clone())
                .or_insert_with(|| CounterpartyExposureSignal {
                    counterparty_id: bid.lp_id.clone(),
                    asset: request
                        .map(|row| row.crypto_asset.clone())
                        .unwrap_or_else(|| "USDT".to_string()),
                    gross_exposure: Decimal::ZERO,
                    won_count: 0,
                    quote_count: 0,
                    reliability_score: None,
                    dispute_rate: None,
                });

            entry.quote_count += 1;
            entry.gross_exposure += bid.vnd_amount;

            if bid.state == "ACCEPTED" {
                entry.won_count += 1;
            }

            if let Some(snapshot) = snapshots
                .iter()
                .find(|row| row.lp_id == bid.lp_id && row.direction == "OFFRAMP")
            {
                entry.reliability_score = snapshot.reliability_score;
                entry.dispute_rate = Some(snapshot.dispute_rate);
            }
        }

        let mut values = exposures.into_values().collect::<Vec<_>>();
        values.sort_by(|left, right| right.gross_exposure.cmp(&left.gross_exposure));
        values
    }

    /// Finalize an RFQ by selecting the best bid and marking it MATCHED.
    /// Also marks all other PENDING bids as REJECTED.
    /// Can be triggered by user accept or admin manually.
    #[instrument(skip(self), fields(tenant_id = %tenant_id.0, rfq_id = %rfq_id))]
    pub async fn finalize_rfq(&self, tenant_id: &TenantId, rfq_id: &str) -> Result<FinalizeResult> {
        let mut rfq = self
            .rfq_repo
            .get_request(tenant_id, rfq_id)
            .await?
            .ok_or_else(|| Error::NotFound(format!("RFQ {} not found", rfq_id)))?;

        if rfq.state != "OPEN" {
            return Err(Error::Conflict(format!(
                "RFQ {} cannot be finalized from state {}",
                rfq_id, rfq.state
            )));
        }

        // Select best bid based on direction
        let best_bid = self
            .select_best_bid(tenant_id, &rfq)
            .await?
            .ok_or_else(|| Error::Conflict(format!("No valid bids found for RFQ {}", rfq_id)))?;

        // Mark winning bid as ACCEPTED
        self.rfq_repo
            .update_bid_state(tenant_id, &best_bid.id, "ACCEPTED")
            .await?;

        // Reject all other PENDING bids
        let all_bids = self
            .rfq_repo
            .list_bids_for_request(tenant_id, rfq_id)
            .await?;
        for bid in &all_bids {
            if bid.id != best_bid.id && bid.state == "PENDING" {
                if let Err(e) = self
                    .rfq_repo
                    .update_bid_state(tenant_id, &bid.id, "REJECTED")
                    .await
                {
                    warn!(bid_id = %bid.id, error = %e, "Failed to reject losing bid");
                } else if let Err(e) = self
                    .ingest_reliability_delta(
                        tenant_id,
                        &bid.lp_id,
                        &rfq.direction,
                        0,
                        0,
                        1,
                        0,
                        0,
                        None,
                        json!({
                            "lastOutcome": "rfq_rejected",
                            "rfqId": rfq_id,
                            "bidId": bid.id,
                        }),
                    )
                    .await
                {
                    warn!(bid_id = %bid.id, error = %e, "Failed to ingest reliability reject outcome");
                }
            }
        }

        // Mark RFQ as MATCHED
        rfq.state = "MATCHED".to_string();
        rfq.winning_bid_id = Some(best_bid.id.clone());
        rfq.winning_lp_id = Some(best_bid.lp_id.clone());
        rfq.final_rate = Some(best_bid.exchange_rate);
        rfq.updated_at = Utc::now();
        self.rfq_repo.update_request(&rfq).await?;
        self.ingest_fill_outcome(tenant_id, &rfq, &best_bid).await?;

        // Publish matched event
        if let Err(e) = self
            .event_publisher
            .publish_rfq_matched(rfq_id, tenant_id, &best_bid.exchange_rate.to_string())
            .await
        {
            warn!(rfq_id = %rfq_id, error = %e, "Failed to publish rfq_matched event");
        }

        info!(
            rfq_id = %rfq_id,
            winning_bid = %best_bid.id,
            lp_id = %best_bid.lp_id,
            final_rate = %best_bid.exchange_rate,
            "RFQ finalized"
        );

        Ok(FinalizeResult {
            rfq,
            winning_bid: best_bid,
        })
    }

    /// Cancel an open RFQ (user-initiated).
    #[instrument(skip(self), fields(tenant_id = %tenant_id.0, rfq_id = %rfq_id))]
    pub async fn cancel_rfq(&self, tenant_id: &TenantId, rfq_id: &str) -> Result<RfqRequestRow> {
        let mut rfq = self
            .rfq_repo
            .get_request(tenant_id, rfq_id)
            .await?
            .ok_or_else(|| Error::NotFound(format!("RFQ {} not found", rfq_id)))?;

        if rfq.state != "OPEN" {
            return Err(Error::Conflict(format!(
                "RFQ {} cannot be cancelled from state {}",
                rfq_id, rfq.state
            )));
        }

        rfq.state = "CANCELLED".to_string();
        rfq.updated_at = Utc::now();
        self.rfq_repo.update_request(&rfq).await?;

        info!(rfq_id = %rfq_id, "RFQ cancelled by user");
        Ok(rfq)
    }

    /// Build incident-timeline entries for an RFQ request and its bids.
    pub async fn incident_timeline_entries_for_request(
        &self,
        tenant_id: &TenantId,
        rfq_id: &str,
    ) -> Result<Vec<IncidentTimelineEntry>> {
        let request = self
            .rfq_repo
            .get_request(tenant_id, rfq_id)
            .await?
            .ok_or_else(|| Error::NotFound(format!("RFQ {} not found", rfq_id)))?;
        let bids = self
            .rfq_repo
            .list_bids_for_request(tenant_id, rfq_id)
            .await?;

        let mut entries = vec![IncidentTimelineEntry::from_rfq_request(request)];
        entries.extend(bids.into_iter().map(IncidentTimelineEntry::from_rfq_bid));
        Ok(entries)
    }

    async fn ingest_quote_outcome(
        &self,
        tenant_id: &TenantId,
        direction: &str,
        bid: &RfqBidRow,
    ) -> Result<()> {
        self.ingest_reliability_delta(
            tenant_id,
            &bid.lp_id,
            direction,
            1,
            0,
            0,
            0,
            0,
            None,
            json!({
                "lastOutcome": "rfq_quote_submitted",
                "rfqId": bid.rfq_id,
                "bidId": bid.id,
            }),
        )
        .await
    }

    async fn ingest_fill_outcome(
        &self,
        tenant_id: &TenantId,
        rfq: &RfqRequestRow,
        winning_bid: &RfqBidRow,
    ) -> Result<()> {
        self.ingest_reliability_delta(
            tenant_id,
            &winning_bid.lp_id,
            &rfq.direction,
            0,
            1,
            0,
            0,
            0,
            Some(Decimal::ZERO),
            json!({
                "lastOutcome": "rfq_matched",
                "rfqId": rfq.id,
                "winningBidId": winning_bid.id,
            }),
        )
        .await
    }

    async fn ingest_reliability_delta(
        &self,
        tenant_id: &TenantId,
        lp_id: &str,
        direction: &str,
        quote_delta: i32,
        fill_delta: i32,
        reject_delta: i32,
        settlement_delta: i32,
        dispute_delta: i32,
        avg_slippage_bps_sample: Option<Decimal>,
        metadata: serde_json::Value,
    ) -> Result<()> {
        let now = Utc::now();
        let window_started_at = now - chrono::Duration::days(30);
        let existing = self
            .rfq_repo
            .get_latest_reliability_snapshot(tenant_id, lp_id, direction, "ROLLING_30D")
            .await?;

        let mut snapshot = existing.unwrap_or(LpReliabilitySnapshotRow {
            id: format!("lprs_{}", uuid::Uuid::now_v7()),
            tenant_id: tenant_id.0.clone(),
            lp_id: lp_id.to_string(),
            direction: direction.to_string(),
            window_kind: "ROLLING_30D".to_string(),
            window_started_at,
            window_ended_at: now,
            snapshot_version: "v1".to_string(),
            quote_count: 0,
            fill_count: 0,
            reject_count: 0,
            settlement_count: 0,
            dispute_count: 0,
            fill_rate: Decimal::ZERO,
            reject_rate: Decimal::ZERO,
            dispute_rate: Decimal::ZERO,
            avg_slippage_bps: Decimal::ZERO,
            p95_settlement_latency_seconds: 0,
            reliability_score: None,
            metadata: json!({}),
            created_at: now,
            updated_at: now,
        });

        let previous_fill_count = snapshot.fill_count.max(0);
        snapshot.quote_count += quote_delta;
        snapshot.fill_count += fill_delta;
        snapshot.reject_count += reject_delta;
        snapshot.settlement_count += settlement_delta;
        snapshot.dispute_count += dispute_delta;
        snapshot.window_ended_at = now;
        snapshot.updated_at = now;
        snapshot.fill_rate = ratio(snapshot.fill_count, snapshot.quote_count);
        snapshot.reject_rate = ratio(snapshot.reject_count, snapshot.quote_count);
        snapshot.dispute_rate = ratio(snapshot.dispute_count, snapshot.fill_count.max(1));

        if let Some(sample) = avg_slippage_bps_sample {
            snapshot.avg_slippage_bps = weighted_average(
                snapshot.avg_slippage_bps,
                previous_fill_count,
                sample,
                fill_delta.max(1),
            );
        }

        snapshot.metadata = metadata;

        self.rfq_repo.upsert_reliability_snapshot(&snapshot).await
    }

    async fn select_best_bid(
        &self,
        tenant_id: &TenantId,
        rfq: &RfqRequestRow,
    ) -> Result<Option<RfqBidRow>> {
        let now = Utc::now();
        let pending_bids: Vec<RfqBidRow> = self
            .rfq_repo
            .list_bids_for_request(tenant_id, &rfq.id)
            .await?
            .into_iter()
            .filter(|bid| bid.state == "PENDING" && bid.valid_until > now)
            .collect();

        if pending_bids.is_empty() {
            return Ok(None);
        }

        let policy = default_liquidity_policy(&rfq.direction);
        let mut policy_candidates = Vec::with_capacity(pending_bids.len());
        for bid in &pending_bids {
            let snapshot = self
                .rfq_repo
                .get_latest_reliability_snapshot(
                    tenant_id,
                    &bid.lp_id,
                    &rfq.direction,
                    &policy.reliability_window_kind,
                )
                .await?;
            policy_candidates.push(policy_candidate_from_bid(bid, snapshot.as_ref()));
        }

        let decision = LiquidityPolicyEvaluator::evaluate(&policy_candidates, Some(&policy));
        let selected_candidate_id = decision
            .as_ref()
            .map(|value| value.selected_candidate_id.as_str());

        Ok(selected_candidate_id
            .and_then(|selected_id| {
                pending_bids
                    .iter()
                    .find(|bid| bid.id == selected_id)
                    .cloned()
            })
            .or_else(|| {
                pending_bids
                    .into_iter()
                    .reduce(best_price_bid(&rfq.direction))
            }))
    }
}

fn ratio(numerator: i32, denominator: i32) -> Decimal {
    if denominator <= 0 {
        Decimal::ZERO
    } else {
        Decimal::from(numerator.max(0)) / Decimal::from(denominator)
    }
}

fn weighted_average(
    previous_average: Decimal,
    previous_count: i32,
    sample_average: Decimal,
    sample_count: i32,
) -> Decimal {
    let total_count = previous_count.max(0) + sample_count.max(0);
    if total_count <= 0 {
        return Decimal::ZERO;
    }

    let weighted_total = (previous_average * Decimal::from(previous_count.max(0)))
        + (sample_average * Decimal::from(sample_count.max(0)));
    weighted_total / Decimal::from(total_count)
}

fn default_liquidity_policy(direction: &str) -> LiquidityPolicyConfig {
    LiquidityPolicyConfig {
        version: "liquidity-policy-default-v1".to_string(),
        direction: if direction == "ONRAMP" {
            LiquidityPolicyDirection::Onramp
        } else {
            LiquidityPolicyDirection::Offramp
        },
        reliability_window_kind: "ROLLING_30D".to_string(),
        min_reliability_observations: 3,
        weights: LiquidityPolicyWeights {
            price_weight: Decimal::new(20, 2),
            reliability_weight: Decimal::new(40, 2),
            fill_rate_weight: Decimal::new(20, 2),
            reject_rate_weight: Decimal::new(10, 2),
            dispute_rate_weight: Decimal::new(5, 2),
            slippage_weight: Decimal::new(3, 2),
            settlement_latency_weight: Decimal::new(2, 2),
        },
    }
}

fn policy_candidate_from_bid(
    bid: &RfqBidRow,
    snapshot: Option<&LpReliabilitySnapshotRow>,
) -> LiquidityPolicyCandidate {
    LiquidityPolicyCandidate {
        candidate_id: bid.id.clone(),
        lp_id: bid.lp_id.clone(),
        quoted_rate: bid.exchange_rate,
        quoted_vnd_amount: bid.vnd_amount,
        quote_count: snapshot.map(|value| value.quote_count).unwrap_or(0),
        reliability_score: snapshot.and_then(|value| value.reliability_score),
        fill_rate: snapshot.map(|value| value.fill_rate),
        reject_rate: snapshot.map(|value| value.reject_rate),
        dispute_rate: snapshot.map(|value| value.dispute_rate),
        avg_slippage_bps: snapshot.map(|value| value.avg_slippage_bps),
        p95_settlement_latency_seconds: snapshot.map(|value| value.p95_settlement_latency_seconds),
    }
}

fn best_price_bid(direction: &str) -> impl FnMut(RfqBidRow, RfqBidRow) -> RfqBidRow + '_ {
    move |left, right| {
        let choose_right = if direction == "ONRAMP" {
            right.exchange_rate.cmp(&left.exchange_rate).is_lt()
                || (right.exchange_rate == left.exchange_rate
                    && right.vnd_amount.cmp(&left.vnd_amount).is_lt())
        } else {
            right.exchange_rate.cmp(&left.exchange_rate).is_gt()
                || (right.exchange_rate == left.exchange_rate
                    && right.vnd_amount.cmp(&left.vnd_amount).is_gt())
        };

        if choose_right {
            right
        } else {
            left
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::InMemoryEventPublisher;
    use crate::repository::rfq::{InMemoryRfqRepository, RfqRepository};
    use rust_decimal_macros::dec;

    fn make_service() -> RfqService {
        RfqService::new(
            Arc::new(InMemoryRfqRepository::new()),
            Arc::new(InMemoryEventPublisher::new()),
        )
    }

    fn tenant() -> TenantId {
        TenantId("tenant_test".to_string())
    }

    #[tokio::test]
    async fn test_create_offramp_rfq() {
        let svc = make_service();
        let rfq = svc
            .create_rfq(CreateRfqRequest {
                tenant_id: tenant(),
                user_id: "user_1".to_string(),
                direction: "OFFRAMP".to_string(),
                offramp_id: None,
                crypto_asset: "USDT".to_string(),
                crypto_amount: Decimal::new(100, 0),
                vnd_amount: None,
                ttl_minutes: 5,
            })
            .await
            .unwrap();

        assert_eq!(rfq.state, "OPEN");
        assert_eq!(rfq.direction, "OFFRAMP");
        assert!(rfq.id.starts_with("rfq_"));
    }

    #[tokio::test]
    async fn test_create_onramp_rfq() {
        let svc = make_service();
        let rfq = svc
            .create_rfq(CreateRfqRequest {
                tenant_id: tenant(),
                user_id: "user_2".to_string(),
                direction: "ONRAMP".to_string(),
                offramp_id: None,
                crypto_asset: "USDT".to_string(),
                crypto_amount: Decimal::new(100, 0),
                vnd_amount: Some(Decimal::new(2_600_000, 0)),
                ttl_minutes: 5,
            })
            .await
            .unwrap();

        assert_eq!(rfq.state, "OPEN");
        assert_eq!(rfq.direction, "ONRAMP");
    }

    #[tokio::test]
    async fn test_finalize_offramp_picks_highest_rate() {
        let svc = make_service();
        let rfq = svc
            .create_rfq(CreateRfqRequest {
                tenant_id: tenant(),
                user_id: "u".to_string(),
                direction: "OFFRAMP".to_string(),
                offramp_id: None,
                crypto_asset: "USDT".to_string(),
                crypto_amount: Decimal::new(100, 0),
                vnd_amount: None,
                ttl_minutes: 5,
            })
            .await
            .unwrap();

        svc.submit_bid(SubmitBidRequest {
            tenant_id: tenant(),
            rfq_id: rfq.id.clone(),
            lp_id: "lp_a".to_string(),
            lp_name: None,
            exchange_rate: Decimal::new(25_800, 0),
            vnd_amount: Decimal::new(2_580_000, 0),
            valid_minutes: 5,
        })
        .await
        .unwrap();

        svc.submit_bid(SubmitBidRequest {
            tenant_id: tenant(),
            rfq_id: rfq.id.clone(),
            lp_id: "lp_b".to_string(),
            lp_name: None,
            exchange_rate: Decimal::new(26_000, 0), // best for OFFRAMP
            vnd_amount: Decimal::new(2_600_000, 0),
            valid_minutes: 5,
        })
        .await
        .unwrap();

        let result = svc.finalize_rfq(&tenant(), &rfq.id).await.unwrap();

        assert_eq!(result.rfq.state, "MATCHED");
        assert_eq!(result.winning_bid.lp_id, "lp_b");
        assert_eq!(result.winning_bid.exchange_rate, Decimal::new(26_000, 0));
    }

    #[tokio::test]
    async fn test_finalize_onramp_picks_lowest_rate() {
        let svc = make_service();
        let rfq = svc
            .create_rfq(CreateRfqRequest {
                tenant_id: tenant(),
                user_id: "u".to_string(),
                direction: "ONRAMP".to_string(),
                offramp_id: None,
                crypto_asset: "USDT".to_string(),
                crypto_amount: Decimal::new(100, 0),
                vnd_amount: Some(Decimal::new(2_600_000, 0)),
                ttl_minutes: 5,
            })
            .await
            .unwrap();

        svc.submit_bid(SubmitBidRequest {
            tenant_id: tenant(),
            rfq_id: rfq.id.clone(),
            lp_id: "lp_x".to_string(),
            lp_name: None,
            exchange_rate: Decimal::new(26_000, 0),
            vnd_amount: Decimal::new(2_600_000, 0),
            valid_minutes: 5,
        })
        .await
        .unwrap();

        svc.submit_bid(SubmitBidRequest {
            tenant_id: tenant(),
            rfq_id: rfq.id.clone(),
            lp_id: "lp_y".to_string(),
            lp_name: None,
            exchange_rate: Decimal::new(25_200, 0), // best for ONRAMP
            vnd_amount: Decimal::new(2_520_000, 0),
            valid_minutes: 5,
        })
        .await
        .unwrap();

        let result = svc.finalize_rfq(&tenant(), &rfq.id).await.unwrap();

        assert_eq!(result.rfq.state, "MATCHED");
        assert_eq!(result.winning_bid.lp_id, "lp_y");
        assert_eq!(result.winning_bid.exchange_rate, Decimal::new(25_200, 0));
    }

    #[tokio::test]
    async fn test_incident_timeline_entries_for_request_include_request_and_bids() {
        let svc = make_service();
        let rfq = svc
            .create_rfq(CreateRfqRequest {
                tenant_id: tenant(),
                user_id: "user_incident".to_string(),
                direction: "OFFRAMP".to_string(),
                offramp_id: Some("ofr_incident_rfq".to_string()),
                crypto_asset: "USDT".to_string(),
                crypto_amount: Decimal::new(250, 0),
                vnd_amount: None,
                ttl_minutes: 5,
            })
            .await
            .unwrap();

        let bid = svc
            .submit_bid(SubmitBidRequest {
                tenant_id: tenant(),
                rfq_id: rfq.id.clone(),
                lp_id: "lp_incident".to_string(),
                lp_name: Some("LP Incident".to_string()),
                exchange_rate: Decimal::new(25_700, 0),
                vnd_amount: Decimal::new(6_425_000, 0),
                valid_minutes: 5,
            })
            .await
            .unwrap();

        let entries = svc
            .incident_timeline_entries_for_request(&tenant(), &rfq.id)
            .await
            .unwrap();

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].source_reference_id, rfq.id);
        assert_eq!(entries[0].details["direction"], "OFFRAMP");
        assert_eq!(entries[1].source_reference_id, bid.id);
        assert_eq!(entries[1].details["rfqId"], entries[0].source_reference_id);
    }

    #[tokio::test]
    async fn test_submit_bid_records_quote_reliability_snapshot() {
        let repo = Arc::new(InMemoryRfqRepository::new());
        let svc = RfqService::new(repo.clone(), Arc::new(InMemoryEventPublisher::new()));
        let rfq = svc
            .create_rfq(CreateRfqRequest {
                tenant_id: tenant(),
                user_id: "user_quote".to_string(),
                direction: "OFFRAMP".to_string(),
                offramp_id: Some("ofr_quote".to_string()),
                crypto_asset: "USDT".to_string(),
                crypto_amount: Decimal::new(100, 0),
                vnd_amount: None,
                ttl_minutes: 5,
            })
            .await
            .unwrap();

        let bid = svc
            .submit_bid(SubmitBidRequest {
                tenant_id: tenant(),
                rfq_id: rfq.id,
                lp_id: "lp_quote".to_string(),
                lp_name: None,
                exchange_rate: Decimal::new(25_600, 0),
                vnd_amount: Decimal::new(2_560_000, 0),
                valid_minutes: 5,
            })
            .await
            .unwrap();

        let snapshot = repo
            .get_latest_reliability_snapshot(&tenant(), &bid.lp_id, "OFFRAMP", "ROLLING_30D")
            .await
            .unwrap()
            .expect("snapshot should exist");

        assert_eq!(snapshot.quote_count, 1);
        assert_eq!(snapshot.fill_count, 0);
        assert_eq!(snapshot.reject_count, 0);
        assert_eq!(snapshot.fill_rate, Decimal::ZERO);
        assert_eq!(snapshot.metadata["lastOutcome"], "rfq_quote_submitted");
    }

    #[tokio::test]
    async fn test_finalize_rfq_records_fill_and_reject_reliability_outcomes() {
        let repo = Arc::new(InMemoryRfqRepository::new());
        let svc = RfqService::new(repo.clone(), Arc::new(InMemoryEventPublisher::new()));
        let rfq = svc
            .create_rfq(CreateRfqRequest {
                tenant_id: tenant(),
                user_id: "user_fill".to_string(),
                direction: "OFFRAMP".to_string(),
                offramp_id: Some("ofr_fill".to_string()),
                crypto_asset: "USDT".to_string(),
                crypto_amount: Decimal::new(100, 0),
                vnd_amount: None,
                ttl_minutes: 5,
            })
            .await
            .unwrap();

        let winning_bid = svc
            .submit_bid(SubmitBidRequest {
                tenant_id: tenant(),
                rfq_id: rfq.id.clone(),
                lp_id: "lp_winner".to_string(),
                lp_name: None,
                exchange_rate: Decimal::new(26_000, 0),
                vnd_amount: Decimal::new(2_600_000, 0),
                valid_minutes: 5,
            })
            .await
            .unwrap();
        let losing_bid = svc
            .submit_bid(SubmitBidRequest {
                tenant_id: tenant(),
                rfq_id: rfq.id.clone(),
                lp_id: "lp_loser".to_string(),
                lp_name: None,
                exchange_rate: Decimal::new(25_500, 0),
                vnd_amount: Decimal::new(2_550_000, 0),
                valid_minutes: 5,
            })
            .await
            .unwrap();

        svc.finalize_rfq(&tenant(), &rfq.id).await.unwrap();

        let winner_snapshot = repo
            .get_latest_reliability_snapshot(
                &tenant(),
                &winning_bid.lp_id,
                "OFFRAMP",
                "ROLLING_30D",
            )
            .await
            .unwrap()
            .expect("winner snapshot should exist");
        let loser_snapshot = repo
            .get_latest_reliability_snapshot(&tenant(), &losing_bid.lp_id, "OFFRAMP", "ROLLING_30D")
            .await
            .unwrap()
            .expect("loser snapshot should exist");

        assert_eq!(winner_snapshot.quote_count, 1);
        assert_eq!(winner_snapshot.fill_count, 1);
        assert_eq!(winner_snapshot.fill_rate, dec!(1));
        assert_eq!(loser_snapshot.quote_count, 1);
        assert_eq!(loser_snapshot.reject_count, 1);
        assert_eq!(loser_snapshot.reject_rate, dec!(1));
    }

    #[tokio::test]
    async fn test_get_best_bid_uses_policy_when_reliability_data_exists() {
        let repo = Arc::new(InMemoryRfqRepository::new());
        let svc = RfqService::new(repo.clone(), Arc::new(InMemoryEventPublisher::new()));

        let rfq = svc
            .create_rfq(CreateRfqRequest {
                tenant_id: tenant(),
                user_id: "u_policy".to_string(),
                direction: "OFFRAMP".to_string(),
                offramp_id: None,
                crypto_asset: "USDT".to_string(),
                crypto_amount: Decimal::new(100, 0),
                vnd_amount: None,
                ttl_minutes: 5,
            })
            .await
            .unwrap();

        let stronger = svc
            .submit_bid(SubmitBidRequest {
                tenant_id: tenant(),
                rfq_id: rfq.id.clone(),
                lp_id: "lp_strong".to_string(),
                lp_name: None,
                exchange_rate: Decimal::new(25_900, 0),
                vnd_amount: Decimal::new(2_590_000, 0),
                valid_minutes: 5,
            })
            .await
            .unwrap();

        let weaker = svc
            .submit_bid(SubmitBidRequest {
                tenant_id: tenant(),
                rfq_id: rfq.id.clone(),
                lp_id: "lp_weak".to_string(),
                lp_name: None,
                exchange_rate: Decimal::new(26_000, 0),
                vnd_amount: Decimal::new(2_600_000, 0),
                valid_minutes: 5,
            })
            .await
            .unwrap();

        let mut stronger_snapshot = repo
            .get_latest_reliability_snapshot(&tenant(), &stronger.lp_id, "OFFRAMP", "ROLLING_30D")
            .await
            .unwrap()
            .unwrap();
        stronger_snapshot.quote_count = 8;
        stronger_snapshot.reliability_score = Some(dec!(0.95));
        stronger_snapshot.fill_rate = dec!(0.98);
        stronger_snapshot.reject_rate = dec!(0.01);
        stronger_snapshot.dispute_rate = dec!(0.01);
        stronger_snapshot.avg_slippage_bps = dec!(4);
        stronger_snapshot.p95_settlement_latency_seconds = 180;
        repo.upsert_reliability_snapshot(&stronger_snapshot)
            .await
            .unwrap();

        let mut weaker_snapshot = repo
            .get_latest_reliability_snapshot(&tenant(), &weaker.lp_id, "OFFRAMP", "ROLLING_30D")
            .await
            .unwrap()
            .unwrap();
        weaker_snapshot.quote_count = 8;
        weaker_snapshot.reliability_score = Some(dec!(0.40));
        weaker_snapshot.fill_rate = dec!(0.55);
        weaker_snapshot.reject_rate = dec!(0.25);
        weaker_snapshot.dispute_rate = dec!(0.15);
        weaker_snapshot.avg_slippage_bps = dec!(25);
        weaker_snapshot.p95_settlement_latency_seconds = 1800;
        repo.upsert_reliability_snapshot(&weaker_snapshot)
            .await
            .unwrap();

        let best = svc.get_best_bid(&tenant(), &rfq.id).await.unwrap().unwrap();
        assert_eq!(best.id, stronger.id);
    }
}
