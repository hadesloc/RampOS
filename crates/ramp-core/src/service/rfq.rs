//! RFQ (Request For Quote) Service
//!
//! Orchestrates the bidirectional auction mechanism for competitive pricing:
//! - Off-ramp (USDT→VND): LP competing to pay most VND, user picks best rate
//! - On-ramp (VND→USDT): LP competing to sell cheapest, user picks lowest rate

use chrono::Utc;
use ramp_common::{types::TenantId, Error, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tracing::{info, instrument, warn};

use crate::event::EventPublisher;
use crate::repository::rfq::{LpReliabilitySnapshotRow, RfqBidRow, RfqRepository, RfqRequestRow};
use crate::service::incident_timeline::IncidentTimelineEntry;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SettlementQualityStatus {
    Pending,
    Settled,
    Cancelled,
    Disputed,
}

impl SettlementQualityStatus {
    fn as_str(&self) -> &'static str {
        match self {
            SettlementQualityStatus::Pending => "pending",
            SettlementQualityStatus::Settled => "settled",
            SettlementQualityStatus::Cancelled => "cancelled",
            SettlementQualityStatus::Disputed => "disputed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiquidityGovernanceContext {
    pub partner_id: String,
    pub partner_class: String,
    pub corridor_code: Option<String>,
    pub capability_family: Option<String>,
    pub approval_reference: Option<String>,
    pub policy_controlled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedLiquiditySignal {
    pub signal_kind: String,
    pub partner_id: String,
    pub partner_class: String,
    pub lp_id: String,
    pub direction: String,
    pub asset: String,
    pub corridor_code: Option<String>,
    pub capability_family: Option<String>,
    pub approval_reference: Option<String>,
    pub policy_controlled: bool,
    pub rfq_id: Option<String>,
    pub bid_id: Option<String>,
    pub quoted_rate: Decimal,
    pub quoted_vnd_amount: Decimal,
    pub status: String,
    pub cancel_reason: Option<String>,
    pub settlement_latency_seconds: Option<i32>,
    pub has_dispute: bool,
    pub avg_slippage_bps: Option<Decimal>,
    pub quality_tier: String,
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
        if req.direction == "ONRAMP" {
            match req.vnd_amount {
                Some(vnd_amount) if vnd_amount > Decimal::ZERO => {}
                _ => {
                    return Err(Error::Validation(
                        "vnd_amount must be positive for ONRAMP".to_string(),
                    ));
                }
            }
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
        if req.vnd_amount <= Decimal::ZERO {
            return Err(Error::Validation("vnd_amount must be positive".to_string()));
        }

        validate_bid_amounts(&rfq, &req)?;

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
        if let Err(error) = self
            .ingest_quote_outcome(&req.tenant_id, &rfq, &bid)
            .await
        {
            warn!(
                bid_id = %bid.id,
                lp_id = %bid.lp_id,
                error = %error,
                "Failed to ingest RFQ quote reliability outcome"
            );
        }

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
        let mut rfq = self
            .rfq_repo
            .get_request(tenant_id, rfq_id)
            .await?
            .ok_or_else(|| Error::NotFound(format!("RFQ {} not found", rfq_id)))?;

        if rfq.state == "OPEN" && Utc::now() >= rfq.expires_at {
            expire_request(&mut rfq);
            self.rfq_repo.update_request(&rfq).await?;
            self.expire_stale_pending_bids(tenant_id, &rfq.id).await?;
            return Ok(None);
        }

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

        if Utc::now() >= rfq.expires_at {
            expire_request(&mut rfq);
            self.rfq_repo.update_request(&rfq).await?;
            self.expire_stale_pending_bids(tenant_id, rfq_id).await?;
            return Err(Error::Gone(format!("RFQ {} has expired", rfq_id)));
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
                    .ingest_cancel_outcome(tenant_id, &rfq, bid, "rfq_rejected")
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
        if let Err(error) = self.ingest_fill_outcome(tenant_id, &rfq, &best_bid).await {
            warn!(
                bid_id = %best_bid.id,
                lp_id = %best_bid.lp_id,
                error = %error,
                "Failed to ingest RFQ fill reliability outcome"
            );
        }

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
        rfq: &RfqRequestRow,
        bid: &RfqBidRow,
    ) -> Result<()> {
        let governance = default_liquidity_governance_context(rfq, bid);
        let signal = normalize_quote_signal(rfq, bid, &governance);
        self.ingest_reliability_delta(
            tenant_id,
            &bid.lp_id,
            &rfq.direction,
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
                "normalizedSignal": normalized_signal_to_json(&signal),
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
        let governance = default_liquidity_governance_context(rfq, winning_bid);
        let fill_signal =
            normalize_fill_signal(rfq, winning_bid, &governance, Some(Decimal::ZERO));
        let settlement_quality_signal = normalize_settlement_quality_signal(
            &winning_bid.lp_id,
            &rfq.direction,
            &rfq.crypto_asset,
            &governance,
            SettlementQualityStatus::Pending,
            0,
            false,
            Some(Decimal::ZERO),
        );
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
                "normalizedSignal": normalized_signal_to_json(&fill_signal),
                "settlementQualitySignal": normalized_signal_to_json(&settlement_quality_signal),
            }),
        )
        .await
    }

    async fn ingest_cancel_outcome(
        &self,
        tenant_id: &TenantId,
        rfq: &RfqRequestRow,
        bid: &RfqBidRow,
        reason: &str,
    ) -> Result<()> {
        let governance = default_liquidity_governance_context(rfq, bid);
        let signal = normalize_cancel_signal(rfq, bid, &governance, reason);
        self.ingest_reliability_delta(
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
                "lastOutcome": reason,
                "rfqId": rfq.id,
                "bidId": bid.id,
                "normalizedSignal": normalized_signal_to_json(&signal),
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
        self.expire_stale_pending_bids(tenant_id, &rfq.id).await?;
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

        Ok(pending_bids.into_iter().reduce(best_price_bid(&rfq.direction)))
    }

    async fn expire_stale_pending_bids(&self, tenant_id: &TenantId, rfq_id: &str) -> Result<()> {
        let now = Utc::now();
        let stale_bids: Vec<_> = self
            .rfq_repo
            .list_bids_for_request(tenant_id, rfq_id)
            .await?
            .into_iter()
            .filter(|bid| bid.state == "PENDING" && bid.valid_until <= now)
            .collect();

        for bid in stale_bids {
            self.rfq_repo
                .update_bid_state(tenant_id, &bid.id, "EXPIRED")
                .await?;
        }

        Ok(())
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

fn default_liquidity_governance_context(
    rfq: &RfqRequestRow,
    bid: &RfqBidRow,
) -> LiquidityGovernanceContext {
    LiquidityGovernanceContext {
        partner_id: format!("partner_{}", bid.lp_id),
        partner_class: "liquidity_provider".to_string(),
        corridor_code: default_corridor_code(&rfq.direction, &rfq.crypto_asset),
        capability_family: Some("otc_desk".to_string()),
        approval_reference: None,
        policy_controlled: true,
    }
}

fn default_corridor_code(direction: &str, asset: &str) -> Option<String> {
    let asset = asset.to_ascii_uppercase();
    match direction {
        "OFFRAMP" => Some(format!("{}_VN_OFFRAMP", asset)),
        "ONRAMP" => Some(format!("VN_{}_ONRAMP", asset)),
        _ => None,
    }
}

fn normalize_quote_signal(
    rfq: &RfqRequestRow,
    bid: &RfqBidRow,
    governance: &LiquidityGovernanceContext,
) -> NormalizedLiquiditySignal {
    NormalizedLiquiditySignal {
        signal_kind: "quote".to_string(),
        partner_id: governance.partner_id.clone(),
        partner_class: governance.partner_class.clone(),
        lp_id: bid.lp_id.clone(),
        direction: rfq.direction.clone(),
        asset: rfq.crypto_asset.clone(),
        corridor_code: governance.corridor_code.clone(),
        capability_family: governance.capability_family.clone(),
        approval_reference: governance.approval_reference.clone(),
        policy_controlled: governance.policy_controlled,
        rfq_id: Some(rfq.id.clone()),
        bid_id: Some(bid.id.clone()),
        quoted_rate: bid.exchange_rate,
        quoted_vnd_amount: bid.vnd_amount,
        status: "submitted".to_string(),
        cancel_reason: None,
        settlement_latency_seconds: None,
        has_dispute: false,
        avg_slippage_bps: None,
        quality_tier: "unscored".to_string(),
    }
}

fn normalize_fill_signal(
    rfq: &RfqRequestRow,
    bid: &RfqBidRow,
    governance: &LiquidityGovernanceContext,
    avg_slippage_bps: Option<Decimal>,
) -> NormalizedLiquiditySignal {
    NormalizedLiquiditySignal {
        signal_kind: "fill".to_string(),
        partner_id: governance.partner_id.clone(),
        partner_class: governance.partner_class.clone(),
        lp_id: bid.lp_id.clone(),
        direction: rfq.direction.clone(),
        asset: rfq.crypto_asset.clone(),
        corridor_code: governance.corridor_code.clone(),
        capability_family: governance.capability_family.clone(),
        approval_reference: governance.approval_reference.clone(),
        policy_controlled: governance.policy_controlled,
        rfq_id: Some(rfq.id.clone()),
        bid_id: Some(bid.id.clone()),
        quoted_rate: bid.exchange_rate,
        quoted_vnd_amount: bid.vnd_amount,
        status: "matched".to_string(),
        cancel_reason: None,
        settlement_latency_seconds: None,
        has_dispute: false,
        avg_slippage_bps,
        quality_tier: "pending".to_string(),
    }
}

fn normalize_cancel_signal(
    rfq: &RfqRequestRow,
    bid: &RfqBidRow,
    governance: &LiquidityGovernanceContext,
    reason: &str,
) -> NormalizedLiquiditySignal {
    NormalizedLiquiditySignal {
        signal_kind: "cancel".to_string(),
        partner_id: governance.partner_id.clone(),
        partner_class: governance.partner_class.clone(),
        lp_id: bid.lp_id.clone(),
        direction: rfq.direction.clone(),
        asset: rfq.crypto_asset.clone(),
        corridor_code: governance.corridor_code.clone(),
        capability_family: governance.capability_family.clone(),
        approval_reference: governance.approval_reference.clone(),
        policy_controlled: governance.policy_controlled,
        rfq_id: Some(rfq.id.clone()),
        bid_id: Some(bid.id.clone()),
        quoted_rate: bid.exchange_rate,
        quoted_vnd_amount: bid.vnd_amount,
        status: "cancelled".to_string(),
        cancel_reason: Some(reason.to_string()),
        settlement_latency_seconds: None,
        has_dispute: false,
        avg_slippage_bps: None,
        quality_tier: "not_applicable".to_string(),
    }
}

fn normalize_settlement_quality_signal(
    lp_id: &str,
    direction: &str,
    asset: &str,
    governance: &LiquidityGovernanceContext,
    status: SettlementQualityStatus,
    settlement_latency_seconds: i32,
    has_dispute: bool,
    avg_slippage_bps: Option<Decimal>,
) -> NormalizedLiquiditySignal {
    let quality_tier = settlement_quality_tier(
        &status,
        settlement_latency_seconds,
        has_dispute,
        avg_slippage_bps,
    );
    NormalizedLiquiditySignal {
        signal_kind: "settlement_quality".to_string(),
        partner_id: governance.partner_id.clone(),
        partner_class: governance.partner_class.clone(),
        lp_id: lp_id.to_string(),
        direction: direction.to_string(),
        asset: asset.to_string(),
        corridor_code: governance.corridor_code.clone(),
        capability_family: governance.capability_family.clone(),
        approval_reference: governance.approval_reference.clone(),
        policy_controlled: governance.policy_controlled,
        rfq_id: None,
        bid_id: None,
        quoted_rate: Decimal::ZERO,
        quoted_vnd_amount: Decimal::ZERO,
        status: status.as_str().to_string(),
        cancel_reason: None,
        settlement_latency_seconds: Some(settlement_latency_seconds),
        has_dispute,
        avg_slippage_bps,
        quality_tier: quality_tier.to_string(),
    }
}

fn settlement_quality_tier(
    status: &SettlementQualityStatus,
    settlement_latency_seconds: i32,
    has_dispute: bool,
    avg_slippage_bps: Option<Decimal>,
) -> &'static str {
    if *status == SettlementQualityStatus::Disputed {
        "critical"
    } else if has_dispute
        || settlement_latency_seconds >= 300
        || avg_slippage_bps.unwrap_or(Decimal::ZERO) >= Decimal::from(10)
    {
        "watch"
    } else if *status == SettlementQualityStatus::Pending {
        "monitor"
    } else {
        "healthy"
    }
}

fn normalized_signal_to_json(signal: &NormalizedLiquiditySignal) -> serde_json::Value {
    json!({
        "signalKind": signal.signal_kind,
        "partnerId": signal.partner_id,
        "partnerClass": signal.partner_class,
        "lpId": signal.lp_id,
        "direction": signal.direction,
        "asset": signal.asset,
        "corridorCode": signal.corridor_code,
        "capabilityFamily": signal.capability_family,
        "approvalReference": signal.approval_reference,
        "policyControlled": signal.policy_controlled,
        "rfqId": signal.rfq_id,
        "bidId": signal.bid_id,
        "quotedRate": signal.quoted_rate.to_string(),
        "quotedVndAmount": signal.quoted_vnd_amount.to_string(),
        "status": signal.status,
        "cancelReason": signal.cancel_reason,
        "settlementLatencySeconds": signal.settlement_latency_seconds,
        "hasDispute": signal.has_dispute,
        "avgSlippageBps": signal.avg_slippage_bps.map(|value| value.to_string()),
        "qualityTier": signal.quality_tier,
    })
}

fn validate_bid_amounts(rfq: &RfqRequestRow, req: &SubmitBidRequest) -> Result<()> {
    let expected_vnd_amount = rfq.crypto_amount * req.exchange_rate;
    if req.vnd_amount != expected_vnd_amount {
        return Err(Error::Validation(format!(
            "vnd_amount must equal crypto_amount * exchange_rate (expected {})",
            expected_vnd_amount
        )));
    }

    if let Some(budget) = rfq.vnd_amount {
        if req.vnd_amount > budget {
            return Err(Error::Validation(format!(
                "vnd_amount exceeds RFQ budget of {}",
                budget
            )));
        }
    }

    Ok(())
}

fn expire_request(rfq: &mut RfqRequestRow) {
    rfq.state = "EXPIRED".to_string();
    rfq.updated_at = Utc::now();
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

    fn make_service_with_repo() -> (Arc<InMemoryRfqRepository>, RfqService) {
        let repo = Arc::new(InMemoryRfqRepository::new());
        let svc = RfqService::new(repo.clone(), Arc::new(InMemoryEventPublisher::new()));
        (repo, svc)
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
    async fn test_get_best_bid_prefers_best_price_even_with_reliability_data() {
        let (repo, svc) = make_service_with_repo();

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
        assert_eq!(best.id, weaker.id);
    }

    #[tokio::test]
    async fn test_submit_bid_rejects_inconsistent_vnd_amount() {
        let svc = make_service();
        let rfq = svc
            .create_rfq(CreateRfqRequest {
                tenant_id: tenant(),
                user_id: "user_amount_check".to_string(),
                direction: "OFFRAMP".to_string(),
                offramp_id: None,
                crypto_asset: "USDT".to_string(),
                crypto_amount: Decimal::new(100, 0),
                vnd_amount: None,
                ttl_minutes: 5,
            })
            .await
            .unwrap();

        let error = svc
            .submit_bid(SubmitBidRequest {
                tenant_id: tenant(),
                rfq_id: rfq.id,
                lp_id: "lp_amount_check".to_string(),
                lp_name: None,
                exchange_rate: Decimal::new(26_000, 0),
                vnd_amount: Decimal::new(2_500_000, 0),
                valid_minutes: 5,
            })
            .await
            .expect_err("bid should be rejected when total VND mismatches rate * crypto amount");

        assert!(matches!(error, Error::Validation(_)));
    }

    #[tokio::test]
    async fn test_submit_bid_rejects_onramp_budget_overrun() {
        let svc = make_service();
        let rfq = svc
            .create_rfq(CreateRfqRequest {
                tenant_id: tenant(),
                user_id: "user_budget_check".to_string(),
                direction: "ONRAMP".to_string(),
                offramp_id: None,
                crypto_asset: "USDT".to_string(),
                crypto_amount: Decimal::new(100, 0),
                vnd_amount: Some(Decimal::new(2_580_000, 0)),
                ttl_minutes: 5,
            })
            .await
            .unwrap();

        let error = svc
            .submit_bid(SubmitBidRequest {
                tenant_id: tenant(),
                rfq_id: rfq.id,
                lp_id: "lp_budget_check".to_string(),
                lp_name: None,
                exchange_rate: Decimal::new(26_000, 0),
                vnd_amount: Decimal::new(2_600_000, 0),
                valid_minutes: 5,
            })
            .await
            .expect_err("ONRAMP bid should not exceed the request budget");

        assert!(matches!(error, Error::Validation(_)));
    }

    #[tokio::test]
    async fn test_finalize_rfq_marks_request_expired_once_ttl_has_elapsed() {
        let (repo, svc) = make_service_with_repo();
        let now = Utc::now();
        let rfq = crate::repository::rfq::RfqRequestRow {
            id: "rfq_expired_finalize".to_string(),
            tenant_id: tenant().0.clone(),
            user_id: "user_expired".to_string(),
            direction: "OFFRAMP".to_string(),
            offramp_id: None,
            crypto_asset: "USDT".to_string(),
            crypto_amount: Decimal::new(100, 0),
            vnd_amount: None,
            state: "OPEN".to_string(),
            winning_bid_id: None,
            winning_lp_id: None,
            final_rate: None,
            expires_at: now - chrono::Duration::seconds(1),
            created_at: now - chrono::Duration::minutes(5),
            updated_at: now - chrono::Duration::minutes(5),
        };
        repo.create_request(&rfq).await.unwrap();
        repo.create_bid(&crate::repository::rfq::RfqBidRow {
            id: "bid_expired_finalize".to_string(),
            rfq_id: rfq.id.clone(),
            tenant_id: tenant().0.clone(),
            lp_id: "lp_expired".to_string(),
            lp_name: None,
            exchange_rate: Decimal::new(26_000, 0),
            vnd_amount: Decimal::new(2_600_000, 0),
            valid_until: now + chrono::Duration::minutes(1),
            state: "PENDING".to_string(),
            created_at: now - chrono::Duration::minutes(1),
        })
        .await
        .unwrap();

        let error = match svc.finalize_rfq(&tenant(), &rfq.id).await {
            Ok(_) => panic!("expired RFQ should not finalize"),
            Err(error) => error,
        };

        assert!(matches!(error, Error::Gone(_) | Error::Conflict(_)));

        let stored = repo
            .get_request(&tenant(), &rfq.id)
            .await
            .unwrap()
            .expect("rfq should exist");
        assert_eq!(stored.state, "EXPIRED");
    }

    #[tokio::test]
    async fn test_get_best_bid_expires_stale_pending_bids() {
        let (repo, svc) = make_service_with_repo();
        let now = Utc::now();
        let rfq = crate::repository::rfq::RfqRequestRow {
            id: "rfq_stale_bid".to_string(),
            tenant_id: tenant().0.clone(),
            user_id: "user_stale".to_string(),
            direction: "OFFRAMP".to_string(),
            offramp_id: None,
            crypto_asset: "USDT".to_string(),
            crypto_amount: Decimal::new(100, 0),
            vnd_amount: None,
            state: "OPEN".to_string(),
            winning_bid_id: None,
            winning_lp_id: None,
            final_rate: None,
            expires_at: now + chrono::Duration::minutes(5),
            created_at: now - chrono::Duration::minutes(5),
            updated_at: now - chrono::Duration::minutes(5),
        };
        repo.create_request(&rfq).await.unwrap();
        repo.create_bid(&crate::repository::rfq::RfqBidRow {
            id: "bid_stale".to_string(),
            rfq_id: rfq.id.clone(),
            tenant_id: tenant().0.clone(),
            lp_id: "lp_stale".to_string(),
            lp_name: None,
            exchange_rate: Decimal::new(25_000, 0),
            vnd_amount: Decimal::new(2_500_000, 0),
            valid_until: now - chrono::Duration::seconds(1),
            state: "PENDING".to_string(),
            created_at: now - chrono::Duration::minutes(1),
        })
        .await
        .unwrap();

        let best = svc.get_best_bid(&tenant(), &rfq.id).await.unwrap();
        assert!(best.is_none());

        let bids = repo.list_bids_for_request(&tenant(), &rfq.id).await.unwrap();
        assert_eq!(bids.len(), 1);
        assert_eq!(bids[0].state, "EXPIRED");
    }

    #[test]
    fn test_normalize_quote_signal_captures_governance_and_amounts() {
        let now = Utc::now();
        let rfq = crate::repository::rfq::RfqRequestRow {
            id: "rfq_norm_quote".to_string(),
            tenant_id: tenant().0.clone(),
            user_id: "user_quote".to_string(),
            direction: "OFFRAMP".to_string(),
            offramp_id: Some("ofr_quote".to_string()),
            crypto_asset: "USDT".to_string(),
            crypto_amount: Decimal::new(125, 0),
            vnd_amount: None,
            state: "OPEN".to_string(),
            winning_bid_id: None,
            winning_lp_id: None,
            final_rate: None,
            expires_at: now + chrono::Duration::minutes(5),
            created_at: now,
            updated_at: now,
        };
        let bid = crate::repository::rfq::RfqBidRow {
            id: "bid_norm_quote".to_string(),
            rfq_id: rfq.id.clone(),
            tenant_id: tenant().0.clone(),
            lp_id: "lp_norm".to_string(),
            lp_name: Some("LP Normalized".to_string()),
            exchange_rate: Decimal::new(26_100, 0),
            vnd_amount: Decimal::new(3_262_500, 0),
            valid_until: now + chrono::Duration::minutes(5),
            state: "PENDING".to_string(),
            created_at: now,
        };
        let governance = LiquidityGovernanceContext {
            partner_id: "partner_lp_norm".to_string(),
            partner_class: "liquidity_provider".to_string(),
            corridor_code: Some("USDT_VN_OFFRAMP".to_string()),
            capability_family: Some("otc_desk".to_string()),
            approval_reference: Some("apr_lp_norm_001".to_string()),
            policy_controlled: true,
        };

        let signal = normalize_quote_signal(&rfq, &bid, &governance);

        assert_eq!(signal.signal_kind, "quote");
        assert_eq!(signal.partner_id, "partner_lp_norm");
        assert_eq!(signal.partner_class, "liquidity_provider");
        assert_eq!(signal.corridor_code.as_deref(), Some("USDT_VN_OFFRAMP"));
        assert_eq!(signal.capability_family.as_deref(), Some("otc_desk"));
        assert_eq!(signal.approval_reference.as_deref(), Some("apr_lp_norm_001"));
        assert_eq!(signal.asset, "USDT");
        assert_eq!(signal.quoted_vnd_amount, Decimal::new(3_262_500, 0));
        assert_eq!(signal.quoted_rate, Decimal::new(26_100, 0));
    }

    #[test]
    fn test_normalize_settlement_quality_signal_includes_status_latency_and_dispute_flags() {
        let governance = LiquidityGovernanceContext {
            partner_id: "partner_lp_norm".to_string(),
            partner_class: "liquidity_provider".to_string(),
            corridor_code: Some("USDT_VN_OFFRAMP".to_string()),
            capability_family: Some("otc_desk".to_string()),
            approval_reference: None,
            policy_controlled: true,
        };

        let signal = normalize_settlement_quality_signal(
            "lp_norm",
            "OFFRAMP",
            "USDT",
            &governance,
            SettlementQualityStatus::Settled,
            480,
            true,
            Some(dec!(18.5)),
        );

        assert_eq!(signal.signal_kind, "settlement_quality");
        assert_eq!(signal.status, "settled");
        assert_eq!(signal.settlement_latency_seconds, Some(480));
        assert_eq!(signal.quality_tier, "watch");
        assert!(signal.has_dispute);
        assert_eq!(signal.avg_slippage_bps, Some(dec!(18.5)));
    }

    #[tokio::test]
    async fn test_finalize_rfq_records_normalized_fill_and_cancel_metadata() {
        let (repo, svc) = make_service_with_repo();
        let rfq = svc
            .create_rfq(CreateRfqRequest {
                tenant_id: tenant(),
                user_id: "user_norm_flow".to_string(),
                direction: "OFFRAMP".to_string(),
                offramp_id: None,
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
                lp_id: "lp_win".to_string(),
                lp_name: Some("Winning LP".to_string()),
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
                lp_id: "lp_lose".to_string(),
                lp_name: Some("Losing LP".to_string()),
                exchange_rate: Decimal::new(25_900, 0),
                vnd_amount: Decimal::new(2_590_000, 0),
                valid_minutes: 5,
            })
            .await
            .unwrap();

        let result = svc.finalize_rfq(&tenant(), &rfq.id).await.unwrap();
        assert_eq!(result.winning_bid.id, winning_bid.id);

        let winner_snapshot = repo
            .get_latest_reliability_snapshot(&tenant(), &winning_bid.lp_id, "OFFRAMP", "ROLLING_30D")
            .await
            .unwrap()
            .unwrap();
        let loser_snapshot = repo
            .get_latest_reliability_snapshot(&tenant(), &losing_bid.lp_id, "OFFRAMP", "ROLLING_30D")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(winner_snapshot.metadata["normalizedSignal"]["signalKind"], "fill");
        assert_eq!(
            winner_snapshot.metadata["normalizedSignal"]["partnerClass"],
            "liquidity_provider"
        );
        assert_eq!(winner_snapshot.metadata["normalizedSignal"]["quotedRate"], "26000");
        assert_eq!(loser_snapshot.metadata["normalizedSignal"]["signalKind"], "cancel");
        assert_eq!(
            loser_snapshot.metadata["normalizedSignal"]["cancelReason"],
            "rfq_rejected"
        );
        assert_eq!(
            loser_snapshot.metadata["normalizedSignal"]["partnerClass"],
            "liquidity_provider"
        );
    }
}
