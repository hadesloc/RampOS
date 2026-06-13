//! Coordinates linked OFFRAMP RFQ matching with settlement execution.

use std::sync::Arc;

use chrono::Utc;
use ramp_common::{types::TenantId, Error, Result};
use serde_json::json;

use crate::repository::{OfframpIntentRepository, RfqRepository, SettlementRepository};
use crate::service::rfq::{FinalizeResult, RfqService};
use crate::service::settlement::{
    ApplySettlementOutcomeRequest, Settlement, SettlementService, TriggerSettlementRequest,
};

pub struct LinkedOfframpExecutionService {
    rfq_service: RfqService,
    rfq_repo: Arc<dyn RfqRepository>,
    offramp_repo: Arc<dyn OfframpIntentRepository>,
    settlement_service: SettlementService,
}

impl LinkedOfframpExecutionService {
    pub fn new(
        rfq_service: RfqService,
        rfq_repo: Arc<dyn RfqRepository>,
        offramp_repo: Arc<dyn OfframpIntentRepository>,
        settlement_repo: Arc<dyn SettlementRepository>,
    ) -> Self {
        Self {
            rfq_service,
            rfq_repo,
            offramp_repo,
            settlement_service: SettlementService::with_repository(settlement_repo),
        }
    }

    pub async fn finalize_rfq(&self, tenant_id: &TenantId, rfq_id: &str) -> Result<FinalizeResult> {
        match self.rfq_service.finalize_rfq(tenant_id, rfq_id).await {
            Ok(result) => {
                self.ensure_linked_settlement(tenant_id, &result).await?;
                Ok(result)
            }
            Err(Error::Conflict(_)) => {
                let rfq = self
                    .rfq_repo
                    .get_request(tenant_id, rfq_id)
                    .await?
                    .ok_or_else(|| Error::NotFound(format!("RFQ {} not found", rfq_id)))?;
                if rfq.state != "MATCHED" {
                    return Err(Error::Conflict(format!(
                        "RFQ {} cannot be finalized from state {}",
                        rfq_id, rfq.state
                    )));
                }
                let winning_bid_id = rfq.winning_bid_id.clone().ok_or_else(|| {
                    Error::Internal(format!("Matched RFQ {} is missing winning bid", rfq_id))
                })?;
                let winning_bid = self
                    .rfq_repo
                    .list_bids_for_request(tenant_id, rfq_id)
                    .await?
                    .into_iter()
                    .find(|bid| bid.id == winning_bid_id)
                    .ok_or_else(|| {
                        Error::Internal(format!(
                            "Matched RFQ {} winning bid {} not found",
                            rfq_id, winning_bid_id
                        ))
                    })?;
                let result = FinalizeResult { rfq, winning_bid };
                self.ensure_linked_settlement(tenant_id, &result).await?;
                Ok(result)
            }
            Err(error) => Err(error),
        }
    }

    pub async fn apply_settlement_outcome(
        &self,
        request: ApplySettlementOutcomeRequest,
    ) -> Result<Settlement> {
        let settlement = self.settlement_service.apply_outcome_async(request).await?;
        let Some(offramp) =
            self.offramp_repo
                .get_intent(
                    &TenantId(settlement.tenant_id.clone().ok_or_else(|| {
                        Error::Internal("Settlement tenant_id is required".into())
                    })?),
                    &settlement.offramp_intent_id,
                )
                .await?
        else {
            return Ok(settlement);
        };

        if is_terminal_offramp_state(&offramp.state) {
            return Ok(settlement);
        }

        let new_state = match settlement.status {
            crate::service::settlement::SettlementStatus::Completed => "COMPLETED",
            crate::service::settlement::SettlementStatus::Failed => "FAILED",
            _ => return Ok(settlement),
        };
        let tenant_id = TenantId(offramp.tenant_id.clone());
        let mut updated = offramp.clone();
        updated.state_history = append_state_transition(
            offramp.state_history,
            &offramp.state,
            new_state,
            Some("Linked settlement outcome applied"),
        );
        updated.state = new_state.to_string();
        updated.settlement_id = Some(settlement.id.clone());
        updated.updated_at = Utc::now();
        self.offramp_repo.update_intent(&updated).await?;

        if let (Some(lp_id), Some(direction)) = (settlement.lp_id.as_deref(), Some("OFFRAMP")) {
            self.settlement_service
                .ingest_reliability_outcome(
                    self.rfq_repo.clone(),
                    &tenant_id,
                    lp_id,
                    direction,
                    &settlement,
                )
                .await?;
        }

        Ok(settlement)
    }

    async fn ensure_linked_settlement(
        &self,
        tenant_id: &TenantId,
        result: &FinalizeResult,
    ) -> Result<Option<Settlement>> {
        if result.rfq.direction != "OFFRAMP" {
            return Ok(None);
        }
        let Some(offramp_id) = result.rfq.offramp_id.as_deref() else {
            return Ok(None);
        };

        let settlement = self
            .settlement_service
            .trigger_settlement_with_request_async(TriggerSettlementRequest {
                tenant_id: Some(tenant_id.clone()),
                offramp_intent_id: offramp_id.to_string(),
                rfq_id: Some(result.rfq.id.clone()),
                lp_id: Some(result.winning_bid.lp_id.clone()),
                final_rate: Some(result.winning_bid.exchange_rate),
            })
            .await?;

        self.offramp_repo
            .update_execution_linkage(
                tenant_id,
                offramp_id,
                Some(&result.rfq.id),
                Some(&result.winning_bid.lp_id),
                Some(result.winning_bid.exchange_rate),
                Some(&settlement.id),
            )
            .await?;

        Ok(Some(settlement))
    }
}

fn is_terminal_offramp_state(state: &str) -> bool {
    matches!(state, "COMPLETED" | "FAILED" | "EXPIRED")
}

fn append_state_transition(
    mut history: serde_json::Value,
    from: &str,
    to: &str,
    reason: Option<&str>,
) -> serde_json::Value {
    let transition = json!({
        "from": from,
        "to": to,
        "timestamp": Utc::now().to_rfc3339(),
        "reason": reason,
    });

    if let Some(arr) = history.as_array_mut() {
        arr.push(transition);
        history
    } else {
        json!([transition])
    }
}
