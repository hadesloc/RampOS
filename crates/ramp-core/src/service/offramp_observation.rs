use chrono::Utc;
use ramp_common::{
    types::{ChainId, CryptoSymbol, TenantId, TxHash, WalletAddress},
    Error, Result,
};
use rust_decimal::Decimal;
use serde_json::json;
use std::sync::Arc;

use crate::repository::{
    OfframpIntentRepository, OfframpIntentRow, OnchainObservationRepository,
    OnchainObservationStatus, UpsertOnchainObservationRequest,
};

#[derive(Debug, Clone)]
pub struct RecordOfframpObservationRequest {
    pub tenant_id: TenantId,
    pub offramp_intent_id: String,
    pub chain_id: ChainId,
    pub tx_hash: TxHash,
    pub from_address: WalletAddress,
    pub to_address: WalletAddress,
    pub amount: Decimal,
    pub symbol: CryptoSymbol,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct ConfirmOfframpObservationRequest {
    pub tenant_id: TenantId,
    pub offramp_intent_id: String,
    pub chain_id: ChainId,
    pub tx_hash: TxHash,
    pub confirmations: u32,
    pub block_number: u64,
    pub metadata: serde_json::Value,
}

pub struct OfframpObservationService {
    offramp_repo: Arc<dyn OfframpIntentRepository>,
    onchain_observation_repo: Arc<dyn OnchainObservationRepository>,
    required_confirmations: u32,
}

impl OfframpObservationService {
    pub fn new(
        offramp_repo: Arc<dyn OfframpIntentRepository>,
        onchain_observation_repo: Arc<dyn OnchainObservationRepository>,
    ) -> Self {
        Self {
            offramp_repo,
            onchain_observation_repo,
            required_confirmations: 12,
        }
    }

    pub fn with_confirmations(mut self, confirmations: u32) -> Self {
        self.required_confirmations = confirmations;
        self
    }

    fn required_confirmations_for_chain(&self, chain_id: &ChainId) -> u32 {
        match chain_id {
            ChainId::Avalanche => 3,
            _ => self.required_confirmations,
        }
    }

    pub async fn record_detected(
        &self,
        req: RecordOfframpObservationRequest,
    ) -> Result<OfframpIntentRow> {
        let mut intent = self
            .offramp_repo
            .get_intent(&req.tenant_id, &req.offramp_intent_id)
            .await?
            .ok_or_else(|| {
                Error::NotFound(format!(
                    "Off-ramp intent not found: {}",
                    req.offramp_intent_id
                ))
            })?;

        validate_observation_request(&intent, &req)?;

        match intent.state.as_str() {
            "CRYPTO_PENDING" => {
                if intent.chain_id.is_none() {
                    intent.chain_id = Some(chain_id_to_observation_id(&req.chain_id));
                } else if intent.chain_id != Some(chain_id_to_observation_id(&req.chain_id)) {
                    return Err(Error::Validation(
                        "Observed chain does not match the persisted off-ramp chain assignment"
                            .to_string(),
                    ));
                }
                intent.tx_hash = Some(req.tx_hash.0.clone());
                intent.state_history = append_state_transition(
                    intent.state_history.clone(),
                    "CRYPTO_PENDING",
                    "CRYPTO_RECEIVED",
                    Some("Off-ramp monitor detected on-chain receipt"),
                );
                intent.state = "CRYPTO_RECEIVED".to_string();
                intent.updated_at = Utc::now();
                self.offramp_repo.update_intent(&intent).await?;
            }
            "CRYPTO_RECEIVED" => {
                if intent.chain_id.is_none() {
                    intent.chain_id = Some(chain_id_to_observation_id(&req.chain_id));
                } else if intent.chain_id != Some(chain_id_to_observation_id(&req.chain_id)) {
                    return Err(Error::Validation(
                        "Observed chain does not match the persisted off-ramp chain assignment"
                            .to_string(),
                    ));
                }
                if intent.tx_hash.is_none() {
                    intent.tx_hash = Some(req.tx_hash.0.clone());
                    intent.updated_at = Utc::now();
                    self.offramp_repo.update_intent(&intent).await?;
                }
            }
            other => {
                return Err(Error::InvalidStateTransition {
                    from: other.to_string(),
                    to: "CRYPTO_RECEIVED".to_string(),
                });
            }
        }

        let required_confirmations = self.required_confirmations_for_chain(&req.chain_id);

        self.onchain_observation_repo
            .upsert_observation(&UpsertOnchainObservationRequest {
                tenant_id: req.tenant_id.0.clone(),
                intent_id: None,
                offramp_intent_id: Some(intent.id.clone()),
                tx_hash: req.tx_hash.0.clone(),
                chain_id: chain_id_to_observation_id(&req.chain_id),
                asset_code: req.symbol.to_string(),
                amount: req.amount,
                from_address: req.from_address.0.clone(),
                to_address: req.to_address.0.clone(),
                status: OnchainObservationStatus::Observed,
                confirmations: 0,
                required_confirmations: required_confirmations as i32,
                block_number: None,
                observation_source: "offramp_monitor_detect".to_string(),
                raw_payload: None,
                metadata: json!({
                    "offrampIntentId": intent.id,
                    "inputMetadata": req.metadata,
                }),
                observed_at: Utc::now(),
                confirmed_at: None,
            })
            .await?;

        Ok(intent)
    }

    pub async fn confirm_detected(
        &self,
        req: ConfirmOfframpObservationRequest,
    ) -> Result<OnchainObservationStatus> {
        let intent = self
            .offramp_repo
            .get_intent(&req.tenant_id, &req.offramp_intent_id)
            .await?
            .ok_or_else(|| {
                Error::NotFound(format!(
                    "Off-ramp intent not found: {}",
                    req.offramp_intent_id
                ))
            })?;

        let expected_tx_hash = intent.tx_hash.as_deref().ok_or_else(|| {
            Error::Validation("Off-ramp intent has no recorded transaction hash".to_string())
        })?;
        if expected_tx_hash != req.tx_hash.0 {
            return Err(Error::Validation(
                "Off-ramp observation confirmation tx hash does not match the persisted intent"
                    .to_string(),
            ));
        }
        if let Some(existing_chain_id) = intent.chain_id {
            if existing_chain_id != chain_id_to_observation_id(&req.chain_id) {
                return Err(Error::Validation(
                    "Off-ramp observation confirmation chain does not match the persisted intent"
                        .to_string(),
                ));
            }
        }

        if intent.state != "CRYPTO_RECEIVED" {
            return Err(Error::InvalidStateTransition {
                from: intent.state,
                to: "CRYPTO_RECEIVED".to_string(),
            });
        }
        let existing_observation = self
            .onchain_observation_repo
            .get_by_tx_hash(
                &req.tenant_id,
                chain_id_to_observation_id(&req.chain_id),
                &req.tx_hash.0,
            )
            .await?;

        let required_confirmations = self.required_confirmations_for_chain(&req.chain_id);

        let status = if req.confirmations >= required_confirmations {
            OnchainObservationStatus::Confirmed
        } else {
            OnchainObservationStatus::Pending
        };
        let now = Utc::now();

        self.onchain_observation_repo
            .upsert_observation(&UpsertOnchainObservationRequest {
                tenant_id: req.tenant_id.0.clone(),
                intent_id: None,
                offramp_intent_id: Some(intent.id.clone()),
                tx_hash: req.tx_hash.0.clone(),
                chain_id: chain_id_to_observation_id(&req.chain_id),
                asset_code: intent.crypto_asset.clone(),
                amount: intent.crypto_amount,
                from_address: existing_observation
                    .as_ref()
                    .map(|row| row.from_address.clone())
                    .unwrap_or_else(|| "unknown".to_string()),
                to_address: existing_observation
                    .as_ref()
                    .map(|row| row.to_address.clone())
                    .or(intent.deposit_address.clone())
                    .unwrap_or_else(|| "unknown".to_string()),
                status,
                confirmations: req.confirmations as i32,
                required_confirmations: required_confirmations as i32,
                block_number: Some(req.block_number as i64),
                observation_source: "offramp_monitor_confirm".to_string(),
                raw_payload: None,
                metadata: json!({
                    "offrampIntentId": intent.id,
                    "inputMetadata": req.metadata,
                }),
                observed_at: now,
                confirmed_at: (req.confirmations >= required_confirmations).then_some(now),
            })
            .await?;

        Ok(status)
    }
}

fn validate_observation_request(
    intent: &OfframpIntentRow,
    req: &RecordOfframpObservationRequest,
) -> Result<()> {
    if intent.crypto_asset != req.symbol.to_string() {
        return Err(Error::Validation(
            "Observed asset does not match the off-ramp intent".to_string(),
        ));
    }
    if intent.crypto_amount != req.amount {
        return Err(Error::Validation(
            "Observed amount does not match the off-ramp intent".to_string(),
        ));
    }
    if let Some(expected_to_address) = intent.deposit_address.as_deref() {
        if !expected_to_address.eq_ignore_ascii_case(&req.to_address.0) {
            return Err(Error::Validation(
                "Observed recipient address does not match the assigned deposit address"
                    .to_string(),
            ));
        }
    }
    if let Some(existing_tx_hash) = intent.tx_hash.as_deref() {
        if existing_tx_hash != req.tx_hash.0 {
            return Err(Error::Validation(
                "Observed transaction hash does not match the persisted off-ramp intent"
                    .to_string(),
            ));
        }
    }
    Ok(())
}

fn append_state_transition(
    mut history: serde_json::Value,
    from: &str,
    to: &str,
    reason: Option<&str>,
) -> serde_json::Value {
    let Some(arr) = history.as_array_mut() else {
        return json!([{
            "from": from,
            "to": to,
            "timestamp": Utc::now().to_rfc3339(),
            "reason": reason,
        }]);
    };

    arr.push(json!({
        "from": from,
        "to": to,
        "timestamp": Utc::now().to_rfc3339(),
        "reason": reason,
    }));
    history
}

fn chain_id_to_observation_id(chain_id: &ChainId) -> i64 {
    match chain_id {
        ChainId::Ethereum => 1,
        ChainId::Polygon => 137,
        ChainId::BnbChain => 56,
        ChainId::Arbitrum => 42161,
        ChainId::Optimism => 10,
        ChainId::Base => 8453,
        ChainId::Avalanche => 43114,
        ChainId::Solana => 101,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::{OnchainObservationRow, UpsertOnchainObservationRequest};
    use async_trait::async_trait;
    use chrono::Utc;
    use serde_json::json;
    use std::sync::Mutex;

    #[derive(Default)]
    struct SpyOfframpIntentRepository {
        intents: Mutex<Vec<OfframpIntentRow>>,
    }

    #[async_trait]
    impl OfframpIntentRepository for SpyOfframpIntentRepository {
        async fn create_intent(&self, intent: &OfframpIntentRow) -> Result<()> {
            self.intents.lock().unwrap().push(intent.clone());
            Ok(())
        }

        async fn get_intent(
            &self,
            tenant_id: &TenantId,
            id: &str,
        ) -> Result<Option<OfframpIntentRow>> {
            Ok(self
                .intents
                .lock()
                .unwrap()
                .iter()
                .find(|intent| intent.tenant_id == tenant_id.0 && intent.id == id)
                .cloned())
        }

        async fn update_status(
            &self,
            _tenant_id: &TenantId,
            _id: &str,
            _new_state: &str,
            _state_history: &serde_json::Value,
        ) -> Result<()> {
            Ok(())
        }

        async fn update_intent(&self, intent: &OfframpIntentRow) -> Result<()> {
            let mut intents = self.intents.lock().unwrap();
            if let Some(existing) = intents.iter_mut().find(|row| row.id == intent.id) {
                *existing = intent.clone();
            }
            Ok(())
        }

        async fn list_by_tenant(
            &self,
            tenant_id: &TenantId,
            _limit: i64,
            _offset: i64,
        ) -> Result<Vec<OfframpIntentRow>> {
            Ok(self
                .intents
                .lock()
                .unwrap()
                .iter()
                .filter(|intent| intent.tenant_id == tenant_id.0)
                .cloned()
                .collect())
        }

        async fn list_by_status(
            &self,
            tenant_id: &TenantId,
            status: &str,
            _limit: i64,
        ) -> Result<Vec<OfframpIntentRow>> {
            Ok(self
                .intents
                .lock()
                .unwrap()
                .iter()
                .filter(|intent| intent.tenant_id == tenant_id.0 && intent.state == status)
                .cloned()
                .collect())
        }

        async fn list_by_cursor(
            &self,
            tenant_id: &TenantId,
            _cursor: Option<&str>,
            _limit: i64,
        ) -> Result<Vec<OfframpIntentRow>> {
            self.list_by_tenant(tenant_id, 0, 0).await
        }
    }

    #[derive(Clone, Default)]
    struct SpyOnchainObservationRepository {
        writes: Arc<Mutex<Vec<UpsertOnchainObservationRequest>>>,
    }

    #[async_trait]
    impl OnchainObservationRepository for SpyOnchainObservationRepository {
        async fn upsert_observation(
            &self,
            request: &UpsertOnchainObservationRequest,
        ) -> Result<OnchainObservationRow> {
            self.writes.lock().unwrap().push(request.clone());
            Ok(OnchainObservationRow {
                id: "OCO_OFFRAMP_TEST".to_string(),
                tenant_id: request.tenant_id.clone(),
                intent_id: request.intent_id.clone(),
                offramp_intent_id: request.offramp_intent_id.clone(),
                tx_hash: request.tx_hash.clone(),
                chain_id: request.chain_id,
                asset_code: request.asset_code.clone(),
                amount: request.amount,
                from_address: request.from_address.clone(),
                to_address: request.to_address.clone(),
                status: request.status.as_str().to_string(),
                confirmations: request.confirmations,
                required_confirmations: request.required_confirmations,
                block_number: request.block_number,
                observation_source: request.observation_source.clone(),
                raw_payload: request.raw_payload.clone(),
                metadata: request.metadata.clone(),
                observed_at: request.observed_at,
                confirmed_at: request.confirmed_at,
                created_at: request.observed_at,
                updated_at: request.observed_at,
            })
        }

        async fn get_by_tx_hash(
            &self,
            _tenant_id: &TenantId,
            _chain_id: i64,
            _tx_hash: &str,
        ) -> Result<Option<OnchainObservationRow>> {
            Ok(None)
        }

        async fn list_by_offramp_intent(
            &self,
            _tenant_id: &TenantId,
            _offramp_intent_id: &str,
        ) -> Result<Vec<OnchainObservationRow>> {
            Ok(Vec::new())
        }

        async fn list_by_intent(
            &self,
            _tenant_id: &TenantId,
            _intent_id: &str,
        ) -> Result<Vec<OnchainObservationRow>> {
            Ok(Vec::new())
        }
    }

    fn test_intent() -> OfframpIntentRow {
        let now = Utc::now();
        OfframpIntentRow {
            id: "ofr_obs_001".to_string(),
            tenant_id: "tenant1".to_string(),
            user_id: "user1".to_string(),
            chain_id: None,
            crypto_asset: "USDT".to_string(),
            crypto_amount: Decimal::from(100),
            exchange_rate: Decimal::from(25000),
            locked_rate_id: Some("lock_1".to_string()),
            fees: json!({}),
            net_vnd_amount: Decimal::from(2500000),
            gross_vnd_amount: Decimal::from(2500000),
            bank_account: json!({}),
            deposit_address: Some("0x2222222222222222222222222222222222222222".to_string()),
            tx_hash: None,
            bank_reference: None,
            linked_rfq_id: None,
            winning_lp_id: None,
            matched_rate: None,
            settlement_id: None,
            state: "CRYPTO_PENDING".to_string(),
            state_history: json!([]),
            created_at: now,
            updated_at: now,
            quote_expires_at: now,
        }
    }

    #[tokio::test]
    async fn test_record_detected_transitions_intent_and_writes_observation() {
        let offramp_repo = Arc::new(SpyOfframpIntentRepository::default());
        let observation_repo = Arc::new(SpyOnchainObservationRepository::default());
        offramp_repo.create_intent(&test_intent()).await.unwrap();

        let service =
            OfframpObservationService::new(offramp_repo.clone(), observation_repo.clone());
        let updated = service
            .record_detected(RecordOfframpObservationRequest {
                tenant_id: TenantId::new("tenant1"),
                offramp_intent_id: "ofr_obs_001".to_string(),
                chain_id: ChainId::Polygon,
                tx_hash: TxHash::new("0xofframpdetected"),
                from_address: WalletAddress::new("0x1111111111111111111111111111111111111111"),
                to_address: WalletAddress::new("0x2222222222222222222222222222222222222222"),
                amount: Decimal::from(100),
                symbol: CryptoSymbol::USDT,
                metadata: json!({"source":"monitor"}),
            })
            .await
            .unwrap();

        assert_eq!(updated.state, "CRYPTO_RECEIVED");
        assert_eq!(updated.chain_id, Some(137));
        assert_eq!(updated.tx_hash.as_deref(), Some("0xofframpdetected"));

        let writes = observation_repo.writes.lock().unwrap();
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].observation_source, "offramp_monitor_detect");
        assert_eq!(writes[0].status, OnchainObservationStatus::Observed);
        assert_eq!(writes[0].chain_id, 137);
    }

    #[tokio::test]
    async fn test_confirm_detected_updates_observation_status() {
        let offramp_repo = Arc::new(SpyOfframpIntentRepository::default());
        let observation_repo = Arc::new(SpyOnchainObservationRepository::default());
        let mut intent = test_intent();
        intent.state = "CRYPTO_RECEIVED".to_string();
        intent.chain_id = Some(137);
        intent.tx_hash = Some("0xofframpconfirmed".to_string());
        offramp_repo.create_intent(&intent).await.unwrap();

        let service =
            OfframpObservationService::new(offramp_repo.clone(), observation_repo.clone())
                .with_confirmations(12);
        let status = service
            .confirm_detected(ConfirmOfframpObservationRequest {
                tenant_id: TenantId::new("tenant1"),
                offramp_intent_id: "ofr_obs_001".to_string(),
                chain_id: ChainId::Polygon,
                tx_hash: TxHash::new("0xofframpconfirmed"),
                confirmations: 12,
                block_number: 12345,
                metadata: json!({"source":"monitor"}),
            })
            .await
            .unwrap();

        assert_eq!(status, OnchainObservationStatus::Confirmed);

        let writes = observation_repo.writes.lock().unwrap();
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].observation_source, "offramp_monitor_confirm");
        assert_eq!(writes[0].status, OnchainObservationStatus::Confirmed);
        assert_eq!(writes[0].block_number, Some(12345));
    }

    #[tokio::test]
    async fn test_record_detected_rejects_chain_mismatch_when_intent_already_received() {
        let offramp_repo = Arc::new(SpyOfframpIntentRepository::default());
        let observation_repo = Arc::new(SpyOnchainObservationRepository::default());
        let mut intent = test_intent();
        intent.state = "CRYPTO_RECEIVED".to_string();
        intent.chain_id = Some(137);
        intent.tx_hash = Some("0xofframpdetected".to_string());
        offramp_repo.create_intent(&intent).await.unwrap();

        let service =
            OfframpObservationService::new(offramp_repo.clone(), observation_repo.clone());
        let result = service
            .record_detected(RecordOfframpObservationRequest {
                tenant_id: TenantId::new("tenant1"),
                offramp_intent_id: "ofr_obs_001".to_string(),
                chain_id: ChainId::BnbChain,
                tx_hash: TxHash::new("0xofframpdetected"),
                from_address: WalletAddress::new("0x1111111111111111111111111111111111111111"),
                to_address: WalletAddress::new("0x2222222222222222222222222222222222222222"),
                amount: Decimal::from(100),
                symbol: CryptoSymbol::USDT,
                metadata: json!({"source":"monitor"}),
            })
            .await;

        match result {
            Err(Error::Validation(message)) => {
                assert!(message.contains("Observed chain does not match"));
            }
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_record_detected_uses_avalanche_required_confirmations() {
        let offramp_repo = Arc::new(SpyOfframpIntentRepository::default());
        let observation_repo = Arc::new(SpyOnchainObservationRepository::default());
        let mut intent = test_intent();
        intent.chain_id = Some(43114);
        offramp_repo.create_intent(&intent).await.unwrap();

        let service =
            OfframpObservationService::new(offramp_repo.clone(), observation_repo.clone());
        service
            .record_detected(RecordOfframpObservationRequest {
                tenant_id: TenantId::new("tenant1"),
                offramp_intent_id: "ofr_obs_001".to_string(),
                chain_id: ChainId::Avalanche,
                tx_hash: TxHash::new("0xofframpavaxdetected"),
                from_address: WalletAddress::new("0x1111111111111111111111111111111111111111"),
                to_address: WalletAddress::new("0x2222222222222222222222222222222222222222"),
                amount: Decimal::from(100),
                symbol: CryptoSymbol::USDT,
                metadata: json!({"source":"monitor"}),
            })
            .await
            .unwrap();

        let writes = observation_repo.writes.lock().unwrap();
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].required_confirmations, 3);
        assert_eq!(writes[0].chain_id, 43114);
    }

    #[tokio::test]
    async fn test_confirm_detected_uses_avalanche_required_confirmations() {
        let offramp_repo = Arc::new(SpyOfframpIntentRepository::default());
        let observation_repo = Arc::new(SpyOnchainObservationRepository::default());
        let mut intent = test_intent();
        intent.state = "CRYPTO_RECEIVED".to_string();
        intent.chain_id = Some(43114);
        intent.tx_hash = Some("0xofframpavaxconfirmed".to_string());
        offramp_repo.create_intent(&intent).await.unwrap();

        let service =
            OfframpObservationService::new(offramp_repo.clone(), observation_repo.clone())
                .with_confirmations(12);
        let status = service
            .confirm_detected(ConfirmOfframpObservationRequest {
                tenant_id: TenantId::new("tenant1"),
                offramp_intent_id: "ofr_obs_001".to_string(),
                chain_id: ChainId::Avalanche,
                tx_hash: TxHash::new("0xofframpavaxconfirmed"),
                confirmations: 3,
                block_number: 43210,
                metadata: json!({"source":"monitor"}),
            })
            .await
            .unwrap();

        assert_eq!(status, OnchainObservationStatus::Confirmed);

        let writes = observation_repo.writes.lock().unwrap();
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].required_confirmations, 3);
        assert_eq!(writes[0].status, OnchainObservationStatus::Confirmed);
        assert_eq!(writes[0].chain_id, 43114);
    }

    #[test]
    fn test_chain_id_to_observation_id_supports_avalanche() {
        assert_eq!(chain_id_to_observation_id(&ChainId::Avalanche), 43114);
    }
}
