use std::sync::Arc;

use chrono::Utc;
use ramp_common::{types::TenantId, Error, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::repository::{
    BeneficiaryProfileRecord, UpsertVenueTransferRequest, VenueAccountRecord,
    VenueConnectionRecord, VenueTrustRepository, WalletAttestationRecord,
};
use crate::service::rfq::{CreateRfqRequest, RfqService};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareHyperliquidCashoutRequest {
    pub tenant_id: String,
    pub user_id: String,
    pub venue_connection_id: String,
    pub venue_account_id: String,
    pub beneficiary_profile_id: String,
    pub wallet_attestation_id: Uuid,
    pub asset_symbol: String,
    pub network: String,
    pub amount: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedVenueCashout {
    pub transfer_id: String,
    pub venue_key: String,
    pub transfer_direction: String,
    pub status: String,
    pub rfq_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmVenueCashoutReceiptRequest {
    pub tenant_id: String,
    pub user_id: String,
    pub transfer_id: String,
    pub wallet_tx_hash: String,
    pub ttl_minutes: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmedVenueCashout {
    pub transfer_id: String,
    pub status: String,
    pub rfq_id: String,
    pub wallet_tx_hash: String,
    pub offramp_reference: String,
}

pub struct VenueCashoutService {
    venue_repository: Arc<dyn VenueTrustRepository>,
    rfq_service: RfqService,
}

impl VenueCashoutService {
    pub fn new(venue_repository: Arc<dyn VenueTrustRepository>, rfq_service: RfqService) -> Self {
        Self {
            venue_repository,
            rfq_service,
        }
    }

    pub async fn prepare_hyperliquid_cashout(
        &self,
        request: &PrepareHyperliquidCashoutRequest,
    ) -> Result<PreparedVenueCashout> {
        if request.amount <= Decimal::ZERO {
            return Err(Error::Validation("amount must be positive".to_string()));
        }

        let connection = self
            .venue_repository
            .get_connection(&request.tenant_id, &request.venue_connection_id)
            .await?
            .ok_or_else(|| Error::NotFound("venue connection not found".to_string()))?;
        self.ensure_hyperliquid_connection(&connection, &request.user_id)?;

        let account = self
            .venue_repository
            .get_account(&request.tenant_id, &request.venue_account_id)
            .await?
            .ok_or_else(|| Error::NotFound("venue account not found".to_string()))?;
        self.ensure_active_account(&account, &connection)?;

        let beneficiary = self
            .venue_repository
            .get_beneficiary_profile(&request.tenant_id, &request.beneficiary_profile_id)
            .await?
            .ok_or_else(|| Error::NotFound("beneficiary profile not found".to_string()))?;
        self.ensure_verified_beneficiary(&beneficiary, &request.user_id)?;

        let attestation = self
            .venue_repository
            .get_wallet_attestation(&request.tenant_id, request.wallet_attestation_id)
            .await?
            .ok_or_else(|| Error::NotFound("wallet attestation not found".to_string()))?;
        self.ensure_verified_attestation(&attestation, &request.user_id, &request.network)?;

        let transfer_id = format!("vcash_{}", Uuid::now_v7());
        self.venue_repository
            .upsert_transfer(&UpsertVenueTransferRequest {
                transfer_id: transfer_id.clone(),
                tenant_id: request.tenant_id.clone(),
                user_id: request.user_id.clone(),
                beneficiary_profile_id: Some(request.beneficiary_profile_id.clone()),
                wallet_attestation_id: request.wallet_attestation_id,
                venue_connection_id: request.venue_connection_id.clone(),
                venue_account_id: request.venue_account_id.clone(),
                transfer_direction: "venue_to_wallet".to_string(),
                asset_symbol: request.asset_symbol.clone(),
                network: request.network.clone(),
                amount: request.amount,
                origin_intent_id: None,
                rfq_id: None,
                status: "draft".to_string(),
                wallet_tx_hash: None,
                venue_credit_ref: None,
                failure_code: None,
                metadata: json!({
                    "connector": "hyperliquid",
                    "integrationMode": "read_only",
                }),
                submitted_at: None,
                completed_at: None,
            })
            .await?;

        Ok(PreparedVenueCashout {
            transfer_id,
            venue_key: connection.venue_key,
            transfer_direction: "venue_to_wallet".to_string(),
            status: "draft".to_string(),
            rfq_id: None,
        })
    }

    pub async fn confirm_wallet_receipt(
        &self,
        request: &ConfirmVenueCashoutReceiptRequest,
    ) -> Result<ConfirmedVenueCashout> {
        let mut transfer = self
            .venue_repository
            .get_transfer(&request.tenant_id, &request.transfer_id)
            .await?
            .ok_or_else(|| Error::NotFound("venue transfer not found".to_string()))?;

        if transfer.user_id != request.user_id {
            return Err(Error::NotFound("venue transfer not found".to_string()));
        }
        if transfer.transfer_direction != "venue_to_wallet" {
            return Err(Error::Validation(
                "venue transfer must be venue_to_wallet".to_string(),
            ));
        }
        if transfer.status != "draft" {
            return Err(Error::Conflict(format!(
                "venue transfer is not receivable from state {}",
                transfer.status
            )));
        }

        let offramp_reference = format!("venue_cashout_{}", transfer.transfer_id);
        let rfq = self
            .rfq_service
            .create_rfq(CreateRfqRequest {
                tenant_id: TenantId(request.tenant_id.clone()),
                user_id: request.user_id.clone(),
                direction: "OFFRAMP".to_string(),
                offramp_id: Some(offramp_reference.clone()),
                crypto_asset: transfer.asset_symbol.clone(),
                crypto_amount: transfer.amount,
                vnd_amount: None,
                ttl_minutes: request.ttl_minutes.unwrap_or(5).clamp(1, 60),
            })
            .await?;

        transfer.rfq_id = Some(rfq.id.clone());
        transfer.status = "submitted".to_string();
        transfer.wallet_tx_hash = Some(request.wallet_tx_hash.clone());
        transfer.submitted_at = Some(Utc::now());
        transfer.metadata = merge_metadata(
            transfer.metadata,
            json!({ "offrampReference": offramp_reference.clone() }),
        );

        self.venue_repository
            .upsert_transfer(&UpsertVenueTransferRequest {
                transfer_id: transfer.transfer_id.clone(),
                tenant_id: transfer.tenant_id.clone(),
                user_id: transfer.user_id.clone(),
                beneficiary_profile_id: transfer.beneficiary_profile_id.clone(),
                wallet_attestation_id: transfer.wallet_attestation_id,
                venue_connection_id: transfer.venue_connection_id.clone(),
                venue_account_id: transfer.venue_account_id.clone(),
                transfer_direction: transfer.transfer_direction.clone(),
                asset_symbol: transfer.asset_symbol.clone(),
                network: transfer.network.clone(),
                amount: transfer.amount,
                origin_intent_id: transfer.origin_intent_id.clone(),
                rfq_id: transfer.rfq_id.clone(),
                status: transfer.status.clone(),
                wallet_tx_hash: transfer.wallet_tx_hash.clone(),
                venue_credit_ref: transfer.venue_credit_ref.clone(),
                failure_code: transfer.failure_code.clone(),
                metadata: transfer.metadata.clone(),
                submitted_at: transfer.submitted_at,
                completed_at: transfer.completed_at,
            })
            .await?;

        Ok(ConfirmedVenueCashout {
            transfer_id: transfer.transfer_id,
            status: transfer.status,
            rfq_id: rfq.id,
            wallet_tx_hash: request.wallet_tx_hash.clone(),
            offramp_reference,
        })
    }

    fn ensure_hyperliquid_connection(
        &self,
        connection: &VenueConnectionRecord,
        user_id: &str,
    ) -> Result<()> {
        if connection.user_id.as_deref() != Some(user_id) {
            return Err(Error::NotFound("venue connection not found".to_string()));
        }
        if connection.venue_key != "hyperliquid" {
            return Err(Error::Validation(
                "venue cashout currently supports only hyperliquid".to_string(),
            ));
        }
        if connection.status != "active" {
            return Err(Error::Conflict(
                "venue connection is not active".to_string(),
            ));
        }
        Ok(())
    }

    fn ensure_active_account(
        &self,
        account: &VenueAccountRecord,
        connection: &VenueConnectionRecord,
    ) -> Result<()> {
        if account.venue_connection_id != connection.connection_id {
            return Err(Error::Validation(
                "venue account does not belong to the supplied connection".to_string(),
            ));
        }
        if account.status != "active" {
            return Err(Error::Conflict("venue account is not active".to_string()));
        }
        Ok(())
    }

    fn ensure_verified_beneficiary(
        &self,
        beneficiary: &BeneficiaryProfileRecord,
        user_id: &str,
    ) -> Result<()> {
        if beneficiary.user_id.as_deref() != Some(user_id) {
            return Err(Error::NotFound("beneficiary profile not found".to_string()));
        }
        if beneficiary.verification_status != "verified" {
            return Err(Error::Conflict(
                "beneficiary profile is not verified".to_string(),
            ));
        }
        Ok(())
    }

    fn ensure_verified_attestation(
        &self,
        attestation: &WalletAttestationRecord,
        user_id: &str,
        network: &str,
    ) -> Result<()> {
        if attestation.user_id != user_id {
            return Err(Error::NotFound("wallet attestation not found".to_string()));
        }
        if attestation.attestation_status != "verified" {
            return Err(Error::Conflict(
                "wallet attestation is not verified".to_string(),
            ));
        }
        if attestation.chain_id != network {
            return Err(Error::Validation(
                "wallet attestation chain does not match requested network".to_string(),
            ));
        }
        Ok(())
    }
}

fn merge_metadata(current: serde_json::Value, patch: serde_json::Value) -> serde_json::Value {
    match (current, patch) {
        (serde_json::Value::Object(mut current_map), serde_json::Value::Object(patch_map)) => {
            current_map.extend(patch_map);
            serde_json::Value::Object(current_map)
        }
        (_, patch) => patch,
    }
}
