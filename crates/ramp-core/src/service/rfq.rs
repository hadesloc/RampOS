//! RFQ (Request For Quote) Service
//!
//! Orchestrates the bidirectional auction mechanism for competitive pricing:
//! - Off-ramp (USDT→VND): LP competing to pay most VND, user picks best rate
//! - On-ramp (VND→USDT): LP competing to sell cheapest, user picks lowest rate

use chrono::Utc;
use ramp_common::{types::TenantId, Error, Result};
use rust_decimal::Decimal;
use std::sync::Arc;
use tracing::{info, instrument, warn};

use crate::event::EventPublisher;
use crate::repository::rfq::{RfqBidRow, RfqRepository, RfqRequestRow};

// ============================================================================
// Input / Output structs
// ============================================================================

pub struct CreateRfqRequest {
    pub tenant_id: TenantId,
    pub user_id: String,
    pub direction: String,          // "OFFRAMP" | "ONRAMP"
    pub offramp_id: Option<String>, // for OFFRAMP, link to existing offramp_intent
    pub crypto_asset: String,
    pub crypto_amount: Decimal,     // for OFFRAMP: amount to sell
    pub vnd_amount: Option<Decimal>,// for ONRAMP: budget in VND
    pub ttl_minutes: i64,           // how long LPs have to submit bids
}

pub struct SubmitBidRequest {
    pub tenant_id: TenantId,
    pub rfq_id: String,
    pub lp_id: String,
    pub lp_name: Option<String>,
    pub exchange_rate: Decimal,     // VND per 1 unit of crypto
    pub vnd_amount: Decimal,        // total VND in the deal
    pub valid_minutes: i64,         // how long this bid stays valid
}

pub struct FinalizeResult {
    pub rfq: RfqRequestRow,
    pub winning_bid: RfqBidRow,
}

// ============================================================================
// Service
// ============================================================================

pub struct RfqService {
    rfq_repo: Arc<dyn RfqRepository>,
    event_publisher: Arc<dyn EventPublisher>,
}

impl RfqService {
    pub fn new(
        rfq_repo: Arc<dyn RfqRepository>,
        event_publisher: Arc<dyn EventPublisher>,
    ) -> Self {
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
            expires_at: now
                + chrono::Duration::minutes(req.ttl_minutes),
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

        self.rfq_repo
            .get_best_bid(tenant_id, rfq_id, &rfq.direction)
            .await
    }

    /// Finalize an RFQ by selecting the best bid and marking it MATCHED.
    /// Also marks all other PENDING bids as REJECTED.
    /// Can be triggered by user accept or admin manually.
    #[instrument(skip(self), fields(tenant_id = %tenant_id.0, rfq_id = %rfq_id))]
    pub async fn finalize_rfq(
        &self,
        tenant_id: &TenantId,
        rfq_id: &str,
    ) -> Result<FinalizeResult> {
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
            .rfq_repo
            .get_best_bid(tenant_id, rfq_id, &rfq.direction)
            .await?
            .ok_or_else(|| Error::Conflict(format!("No valid bids found for RFQ {}", rfq_id)))?;

        // Mark winning bid as ACCEPTED
        self.rfq_repo
            .update_bid_state(tenant_id, &best_bid.id, "ACCEPTED")
            .await?;

        // Reject all other PENDING bids
        let all_bids = self.rfq_repo.list_bids_for_request(tenant_id, rfq_id).await?;
        for bid in &all_bids {
            if bid.id != best_bid.id && bid.state == "PENDING" {
                if let Err(e) = self
                    .rfq_repo
                    .update_bid_state(tenant_id, &bid.id, "REJECTED")
                    .await
                {
                    warn!(bid_id = %bid.id, error = %e, "Failed to reject losing bid");
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

        // Publish matched event
        if let Err(e) = self
            .event_publisher
            .publish_rfq_matched(
                rfq_id,
                tenant_id,
                &best_bid.exchange_rate.to_string(),
            )
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
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::rfq::InMemoryRfqRepository;
    use crate::event::InMemoryEventPublisher;

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
}
