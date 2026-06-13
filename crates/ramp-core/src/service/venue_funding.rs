use std::sync::Arc;

use chrono::Utc;
use ramp_common::{Error, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::repository::{
    UpsertVenueTransferRequest, VenueAccountRecord, VenueConnectionRecord, VenueTrustRepository,
    WalletAttestationRecord,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareVenueFundingTransferRequest {
    pub tenant_id: String,
    pub user_id: String,
    pub venue_connection_id: String,
    pub venue_account_id: String,
    pub wallet_attestation_id: Uuid,
    pub asset_symbol: String,
    pub network: String,
    pub amount: Decimal,
    pub origin_intent_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedVenueFundingTransfer {
    pub transfer_id: String,
    pub venue_key: String,
    pub transfer_direction: String,
    pub status: String,
    pub origin_intent_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitVenueFundingTransferRequest {
    pub tenant_id: String,
    pub user_id: String,
    pub transfer_id: String,
    pub wallet_transfer_reference: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmittedVenueFundingTransfer {
    pub transfer_id: String,
    pub status: String,
    pub wallet_transfer_reference: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedVenueFundingTransfer {
    pub transfer_id: String,
    pub venue_key: String,
    pub transfer_direction: String,
    pub status: String,
    pub asset_symbol: String,
    pub network: String,
    pub amount: Decimal,
    pub origin_intent_id: Option<String>,
    pub wallet_transfer_reference: Option<String>,
}

pub struct VenueFundingService {
    venue_repository: Arc<dyn VenueTrustRepository>,
}

impl VenueFundingService {
    pub fn new(venue_repository: Arc<dyn VenueTrustRepository>) -> Self {
        Self { venue_repository }
    }

    pub async fn prepare_wallet_to_venue_transfer(
        &self,
        request: &PrepareVenueFundingTransferRequest,
    ) -> Result<PreparedVenueFundingTransfer> {
        if request.amount <= Decimal::ZERO {
            return Err(Error::Validation("amount must be positive".to_string()));
        }

        let connection = self
            .venue_repository
            .get_connection(&request.tenant_id, &request.venue_connection_id)
            .await?
            .ok_or_else(|| Error::NotFound("venue connection not found".to_string()))?;
        self.ensure_active_connection(&connection, &request.user_id)?;
        self.ensure_hyperliquid_funding_connection(&connection)?;

        let account = self
            .venue_repository
            .get_account(&request.tenant_id, &request.venue_account_id)
            .await?
            .ok_or_else(|| Error::NotFound("venue account not found".to_string()))?;
        self.ensure_active_account(&account, &connection)?;
        self.ensure_hyperliquid_funding_account(&account)?;

        let attestation = self
            .venue_repository
            .get_wallet_attestation(&request.tenant_id, request.wallet_attestation_id)
            .await?
            .ok_or_else(|| Error::NotFound("wallet attestation not found".to_string()))?;
        self.ensure_verified_attestation(&attestation, &request.user_id, &request.network)?;
        self.ensure_hyperliquid_funding_asset(&request.asset_symbol)?;

        let transfer_id = format!("vfund_{}", Uuid::now_v7());
        self.venue_repository
            .upsert_transfer(&UpsertVenueTransferRequest {
                transfer_id: transfer_id.clone(),
                tenant_id: request.tenant_id.clone(),
                user_id: request.user_id.clone(),
                beneficiary_profile_id: None,
                wallet_attestation_id: request.wallet_attestation_id,
                venue_connection_id: request.venue_connection_id.clone(),
                venue_account_id: request.venue_account_id.clone(),
                transfer_direction: "wallet_to_venue".to_string(),
                asset_symbol: request.asset_symbol.clone(),
                network: request.network.clone(),
                amount: request.amount,
                origin_intent_id: request.origin_intent_id.clone(),
                rfq_id: None,
                status: "draft".to_string(),
                wallet_tx_hash: None,
                venue_credit_ref: None,
                failure_code: None,
                metadata: json!({
                    "channel": "portal",
                    "fundingMode": "wallet_first",
                }),
                submitted_at: None,
                completed_at: None,
            })
            .await?;

        Ok(PreparedVenueFundingTransfer {
            transfer_id,
            venue_key: connection.venue_key,
            transfer_direction: "wallet_to_venue".to_string(),
            status: "draft".to_string(),
            origin_intent_id: request.origin_intent_id.clone(),
        })
    }

    pub async fn submit_wallet_to_venue_transfer(
        &self,
        request: &SubmitVenueFundingTransferRequest,
    ) -> Result<SubmittedVenueFundingTransfer> {
        if request.wallet_transfer_reference.trim().is_empty() {
            return Err(Error::Validation(
                "wallet transfer reference is required".to_string(),
            ));
        }

        let mut transfer = self
            .venue_repository
            .get_transfer(&request.tenant_id, &request.transfer_id)
            .await?
            .ok_or_else(|| Error::NotFound("venue transfer not found".to_string()))?;

        if transfer.user_id != request.user_id {
            return Err(Error::NotFound("venue transfer not found".to_string()));
        }
        if transfer.transfer_direction != "wallet_to_venue" {
            return Err(Error::Validation(
                "venue transfer must be wallet_to_venue".to_string(),
            ));
        }
        if transfer.status != "draft" {
            return Err(Error::Conflict(format!(
                "venue transfer is not submittable from state {}",
                transfer.status
            )));
        }

        transfer.status = "submitted".to_string();
        transfer.wallet_tx_hash = Some(request.wallet_transfer_reference.clone());
        transfer.submitted_at = Some(Utc::now());
        transfer.metadata = merge_metadata(
            transfer.metadata,
            json!({ "walletTransferReference": request.wallet_transfer_reference }),
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

        Ok(SubmittedVenueFundingTransfer {
            transfer_id: transfer.transfer_id,
            status: transfer.status,
            wallet_transfer_reference: request.wallet_transfer_reference.clone(),
        })
    }

    pub async fn get_wallet_to_venue_transfer(
        &self,
        tenant_id: &str,
        user_id: &str,
        transfer_id: &str,
    ) -> Result<Option<PersistedVenueFundingTransfer>> {
        let Some(transfer) = self
            .venue_repository
            .get_transfer(tenant_id, transfer_id)
            .await?
        else {
            return Ok(None);
        };

        if transfer.user_id != user_id || transfer.transfer_direction != "wallet_to_venue" {
            return Ok(None);
        }

        let venue_key = self
            .venue_repository
            .get_connection(tenant_id, &transfer.venue_connection_id)
            .await?
            .map(|record| record.venue_key)
            .unwrap_or_default();

        Ok(Some(PersistedVenueFundingTransfer {
            transfer_id: transfer.transfer_id,
            venue_key,
            transfer_direction: transfer.transfer_direction,
            status: transfer.status,
            asset_symbol: transfer.asset_symbol,
            network: transfer.network,
            amount: transfer.amount,
            origin_intent_id: transfer.origin_intent_id,
            wallet_transfer_reference: transfer.wallet_tx_hash,
        }))
    }

    fn ensure_active_connection(
        &self,
        connection: &VenueConnectionRecord,
        user_id: &str,
    ) -> Result<()> {
        if connection.user_id.as_deref() != Some(user_id) {
            return Err(Error::NotFound("venue connection not found".to_string()));
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

    fn ensure_hyperliquid_funding_connection(
        &self,
        connection: &VenueConnectionRecord,
    ) -> Result<()> {
        if connection.venue_key != "hyperliquid" {
            return Err(Error::Validation(
                "venue funding currently supports only hyperliquid".to_string(),
            ));
        }
        if connection.connection_mode != "wallet_linked" {
            return Err(Error::Validation(
                "hyperliquid funding requires wallet_linked connection mode".to_string(),
            ));
        }
        Ok(())
    }

    fn ensure_hyperliquid_funding_account(&self, account: &VenueAccountRecord) -> Result<()> {
        if account.venue_key != "hyperliquid" {
            return Err(Error::Validation(
                "venue account must belong to hyperliquid funding".to_string(),
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
                "wallet attestation chain does not match funding network".to_string(),
            ));
        }
        Ok(())
    }

    fn ensure_hyperliquid_funding_asset(&self, asset_symbol: &str) -> Result<()> {
        if asset_symbol != "USDT" {
            return Err(Error::Validation(
                "hyperliquid funding currently supports only USDT".to_string(),
            ));
        }
        Ok(())
    }
}

fn merge_metadata(existing: serde_json::Value, patch: serde_json::Value) -> serde_json::Value {
    let mut merged = existing.as_object().cloned().unwrap_or_default();
    if let Some(entries) = patch.as_object() {
        for (key, value) in entries {
            merged.insert(key.clone(), value.clone());
        }
    }
    serde_json::Value::Object(merged)
}
