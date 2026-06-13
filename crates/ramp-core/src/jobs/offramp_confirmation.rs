use crate::chain::{ChainAbstractionLayer, ChainId as RuntimeChainId, TxState, TxStatus};
use crate::repository::{
    OfframpIntentRepository, OnchainObservationRepository, OnchainObservationRow, TenantRepository,
};
use crate::service::{ConfirmOfframpObservationRequest, OfframpObservationService};
use chrono::Utc;
use ramp_common::{
    types::{ChainId as ObservationChainId, TenantId},
    Result,
};
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

pub struct OfframpConfirmationJob {
    tenant_repo: Arc<dyn TenantRepository>,
    offramp_repo: Arc<dyn OfframpIntentRepository>,
    observation_repo: Arc<dyn OnchainObservationRepository>,
    chain_layer: Arc<ChainAbstractionLayer>,
    poll_interval: Duration,
    batch_size: usize,
}

impl OfframpConfirmationJob {
    pub fn new(
        tenant_repo: Arc<dyn TenantRepository>,
        offramp_repo: Arc<dyn OfframpIntentRepository>,
        observation_repo: Arc<dyn OnchainObservationRepository>,
        chain_layer: Arc<ChainAbstractionLayer>,
    ) -> Self {
        Self {
            tenant_repo,
            offramp_repo,
            observation_repo,
            chain_layer,
            poll_interval: Duration::from_secs(30),
            batch_size: 100,
        }
    }

    pub fn with_poll_interval(mut self, poll_interval: Duration) -> Self {
        self.poll_interval = poll_interval;
        self
    }

    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
        self
    }

    pub async fn run(&self) {
        info!(
            poll_interval_secs = self.poll_interval.as_secs(),
            batch_size = self.batch_size,
            "Starting off-ramp confirmation job"
        );

        let mut interval = tokio::time::interval(self.poll_interval);
        loop {
            interval.tick().await;
            match self.process_pending().await {
                Ok(count) if count > 0 => {
                    info!(
                        processed = count,
                        "Off-ramp confirmation job processed observations"
                    );
                }
                Ok(_) => {}
                Err(error) => {
                    error!(error = %error, "Off-ramp confirmation job failed");
                }
            }
        }
    }

    pub async fn process_pending(&self) -> Result<usize> {
        let tenant_ids = self.tenant_repo.list_ids().await?;
        let observation_service = OfframpObservationService::new(
            self.offramp_repo.clone(),
            self.observation_repo.clone(),
        );
        let mut processed = 0usize;

        for tenant_id in tenant_ids {
            let intents = self
                .offramp_repo
                .list_by_status(&tenant_id, "CRYPTO_RECEIVED", self.batch_size as i64)
                .await?;

            for intent in intents {
                let Some(tx_hash) = intent.tx_hash.as_deref() else {
                    continue;
                };
                let observations = self
                    .observation_repo
                    .list_by_offramp_intent(&tenant_id, &intent.id)
                    .await?;

                for observation in observations {
                    if !should_poll_observation(&observation, tx_hash) {
                        continue;
                    }

                    let Some((service_chain_id, runtime_chain_id)) =
                        map_observation_chain_id(observation.chain_id)
                    else {
                        warn!(
                            offramp_intent_id = %intent.id,
                            chain_id = observation.chain_id,
                            "Skipping off-ramp confirmation poll for unsupported chain id"
                        );
                        continue;
                    };

                    let tx_status = match self
                        .chain_layer
                        .get_transaction_status(runtime_chain_id, &observation.tx_hash)
                        .await
                    {
                        Ok(status) => status,
                        Err(error) => {
                            warn!(
                                offramp_intent_id = %intent.id,
                                tx_hash = %observation.tx_hash,
                                chain_id = runtime_chain_id.0,
                                error = %error,
                                "Skipping off-ramp confirmation poll after chain lookup error"
                            );
                            continue;
                        }
                    };

                    if let Some(confirm_request) = build_confirm_request(
                        &tenant_id,
                        &intent.id,
                        service_chain_id,
                        &observation,
                        &tx_status,
                    ) {
                        match observation_service.confirm_detected(confirm_request).await {
                            Ok(_) => {
                                processed += 1;
                            }
                            Err(error) => {
                                warn!(
                                    offramp_intent_id = %intent.id,
                                    tx_hash = %observation.tx_hash,
                                    chain_id = observation.chain_id,
                                    error = %error,
                                    "Skipping off-ramp confirmation for invalid observation row"
                                );
                            }
                        }
                    }
                }
            }
        }

        Ok(processed)
    }
}

fn should_poll_observation(observation: &OnchainObservationRow, expected_tx_hash: &str) -> bool {
    observation.offramp_intent_id.is_some()
        && observation.tx_hash == expected_tx_hash
        && matches!(observation.status.as_str(), "OBSERVED" | "PENDING")
}

fn map_observation_chain_id(chain_id: i64) -> Option<(ObservationChainId, RuntimeChainId)> {
    match chain_id {
        1 => Some((ObservationChainId::Ethereum, RuntimeChainId::ETHEREUM)),
        137 => Some((ObservationChainId::Polygon, RuntimeChainId::POLYGON)),
        56 => Some((ObservationChainId::BnbChain, RuntimeChainId::BSC)),
        42161 => Some((ObservationChainId::Arbitrum, RuntimeChainId::ARBITRUM)),
        10 => Some((ObservationChainId::Optimism, RuntimeChainId::OPTIMISM)),
        8453 => Some((ObservationChainId::Base, RuntimeChainId::BASE)),
        43114 => Some((ObservationChainId::Avalanche, RuntimeChainId::AVALANCHE)),
        101 => Some((ObservationChainId::Solana, RuntimeChainId::SOLANA_MAINNET)),
        _ => None,
    }
}

fn build_confirm_request(
    tenant_id: &TenantId,
    offramp_intent_id: &str,
    chain_id: ObservationChainId,
    observation: &OnchainObservationRow,
    tx_status: &TxStatus,
) -> Option<ConfirmOfframpObservationRequest> {
    if tx_status.status != TxState::Confirmed {
        return None;
    }

    let block_number = tx_status.block_number?;
    Some(ConfirmOfframpObservationRequest {
        tenant_id: tenant_id.clone(),
        offramp_intent_id: offramp_intent_id.to_string(),
        chain_id,
        tx_hash: ramp_common::types::TxHash::new(observation.tx_hash.clone()),
        confirmations: tx_status.confirmations as u32,
        block_number,
        metadata: serde_json::json!({
            "source_kind": "background_confirmation_job",
            "polledAt": Utc::now().to_rfc3339(),
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::{
        Balance, Chain, ChainError, ChainType, FeeEstimate, TokenBalance, Transaction, TxHash,
        UnifiedAddress,
    };
    use crate::repository::{
        OfframpIntentRow, OnchainObservationStatus, UpsertOnchainObservationRequest,
    };
    use async_trait::async_trait;
    use chrono::Utc;
    use rust_decimal::Decimal;
    use std::sync::Mutex;

    #[derive(Default)]
    struct SpyTenantRepository {
        tenant_ids: Vec<TenantId>,
    }

    #[async_trait]
    impl TenantRepository for SpyTenantRepository {
        async fn get_by_id(
            &self,
            _id: &TenantId,
        ) -> Result<Option<crate::repository::tenant::TenantRow>> {
            Ok(None)
        }
        async fn get_by_api_key_hash(
            &self,
            _hash: &str,
        ) -> Result<Option<crate::repository::tenant::TenantRow>> {
            Ok(None)
        }
        async fn create(&self, _tenant: &crate::repository::tenant::TenantRow) -> Result<()> {
            Ok(())
        }
        async fn update_status(&self, _id: &TenantId, _status: &str) -> Result<()> {
            Ok(())
        }
        async fn update_webhook_url(&self, _id: &TenantId, _url: &str) -> Result<()> {
            Ok(())
        }
        async fn update_api_key_hash(&self, _id: &TenantId, _hash: &str) -> Result<()> {
            Ok(())
        }
        async fn update_api_credentials(
            &self,
            _id: &TenantId,
            _api_key_hash: &str,
            _api_secret_encrypted: &[u8],
        ) -> Result<()> {
            Ok(())
        }
        async fn update_webhook_secret(
            &self,
            _id: &TenantId,
            _hash: &str,
            _encrypted: &[u8],
        ) -> Result<()> {
            Ok(())
        }
        async fn update_limits(
            &self,
            _id: &TenantId,
            _daily_payin: Option<rust_decimal::Decimal>,
            _daily_payout: Option<rust_decimal::Decimal>,
        ) -> Result<()> {
            Ok(())
        }
        async fn update_config(&self, _id: &TenantId, _config: &serde_json::Value) -> Result<()> {
            Ok(())
        }
        async fn update_api_version(&self, _id: &TenantId, _version: Option<String>) -> Result<()> {
            Ok(())
        }
        async fn list_ids(&self) -> Result<Vec<TenantId>> {
            Ok(self.tenant_ids.clone())
        }
    }

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
        observations: Arc<Mutex<Vec<OnchainObservationRow>>>,
        writes: Arc<Mutex<Vec<UpsertOnchainObservationRequest>>>,
    }

    #[async_trait]
    impl OnchainObservationRepository for SpyOnchainObservationRepository {
        async fn upsert_observation(
            &self,
            request: &UpsertOnchainObservationRequest,
        ) -> Result<OnchainObservationRow> {
            self.writes.lock().unwrap().push(request.clone());
            let row = OnchainObservationRow {
                id: "OCO_JOB_TEST".to_string(),
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
            };
            let mut observations = self.observations.lock().unwrap();
            if let Some(existing) = observations.iter_mut().find(|value| {
                value.tenant_id == row.tenant_id
                    && value.chain_id == row.chain_id
                    && value.tx_hash == row.tx_hash
            }) {
                *existing = row.clone();
            } else {
                observations.push(row.clone());
            }
            Ok(row)
        }
        async fn get_by_tx_hash(
            &self,
            tenant_id: &TenantId,
            chain_id: i64,
            tx_hash: &str,
        ) -> Result<Option<OnchainObservationRow>> {
            Ok(self
                .observations
                .lock()
                .unwrap()
                .iter()
                .find(|row| {
                    row.tenant_id == tenant_id.0
                        && row.chain_id == chain_id
                        && row.tx_hash == tx_hash
                })
                .cloned())
        }
        async fn list_by_offramp_intent(
            &self,
            tenant_id: &TenantId,
            offramp_intent_id: &str,
        ) -> Result<Vec<OnchainObservationRow>> {
            Ok(self
                .observations
                .lock()
                .unwrap()
                .iter()
                .filter(|row| {
                    row.tenant_id == tenant_id.0
                        && row.offramp_intent_id.as_deref() == Some(offramp_intent_id)
                })
                .cloned()
                .collect())
        }
        async fn list_by_intent(
            &self,
            _tenant_id: &TenantId,
            _intent_id: &str,
        ) -> Result<Vec<OnchainObservationRow>> {
            Ok(Vec::new())
        }
    }

    struct MockConfirmedChain;

    #[async_trait]
    impl Chain for MockConfirmedChain {
        fn chain_id(&self) -> RuntimeChainId {
            RuntimeChainId::POLYGON
        }
        fn name(&self) -> &str {
            "Mock Polygon"
        }
        fn chain_type(&self) -> crate::chain::ChainType {
            crate::chain::ChainType::Evm
        }
        fn is_testnet(&self) -> bool {
            false
        }
        fn native_symbol(&self) -> &str {
            "MATIC"
        }
        fn explorer_url(&self) -> &str {
            "https://example.test"
        }
        fn validate_address(&self, address: &str) -> crate::chain::Result<UnifiedAddress> {
            Ok(UnifiedAddress {
                chain_type: ChainType::Evm,
                address: address.to_string(),
                normalized: address.to_string(),
            })
        }
        async fn get_balance(&self, _address: &str) -> crate::chain::Result<Balance> {
            Err(ChainError::NotSupported("unused".to_string()))
        }
        async fn get_token_balance(
            &self,
            _address: &str,
            _token_address: &str,
        ) -> crate::chain::Result<TokenBalance> {
            Err(ChainError::NotSupported("unused".to_string()))
        }
        async fn send_transaction(&self, _tx: Transaction) -> crate::chain::Result<TxHash> {
            Err(ChainError::NotSupported("unused".to_string()))
        }
        async fn get_transaction(&self, hash: &str) -> crate::chain::Result<TxStatus> {
            Ok(TxStatus {
                hash: TxHash(hash.to_string()),
                status: TxState::Confirmed,
                block_number: Some(12345),
                block_hash: None,
                confirmations: 12,
                gas_used: None,
                effective_gas_price: None,
                error_message: None,
            })
        }
        async fn wait_for_confirmation(
            &self,
            _hash: &TxHash,
            _confirmations: u64,
            _timeout_secs: u64,
        ) -> crate::chain::Result<TxStatus> {
            Err(ChainError::NotSupported("unused".to_string()))
        }
        async fn estimate_fee(&self, _tx: &Transaction) -> crate::chain::Result<FeeEstimate> {
            Err(ChainError::NotSupported("unused".to_string()))
        }
        async fn get_block_number(&self) -> crate::chain::Result<u64> {
            Ok(12357)
        }
    }

    struct MockConfirmedBscChain;

    #[async_trait]
    impl Chain for MockConfirmedBscChain {
        fn chain_id(&self) -> RuntimeChainId {
            RuntimeChainId::BSC
        }
        fn name(&self) -> &str {
            "Mock BSC"
        }
        fn chain_type(&self) -> crate::chain::ChainType {
            crate::chain::ChainType::Evm
        }
        fn is_testnet(&self) -> bool {
            false
        }
        fn native_symbol(&self) -> &str {
            "BNB"
        }
        fn explorer_url(&self) -> &str {
            "https://example.test"
        }
        fn validate_address(&self, address: &str) -> crate::chain::Result<UnifiedAddress> {
            Ok(UnifiedAddress {
                chain_type: ChainType::Evm,
                address: address.to_string(),
                normalized: address.to_string(),
            })
        }
        async fn get_balance(&self, _address: &str) -> crate::chain::Result<Balance> {
            Err(ChainError::NotSupported("unused".to_string()))
        }
        async fn get_token_balance(
            &self,
            _address: &str,
            _token_address: &str,
        ) -> crate::chain::Result<TokenBalance> {
            Err(ChainError::NotSupported("unused".to_string()))
        }
        async fn send_transaction(&self, _tx: Transaction) -> crate::chain::Result<TxHash> {
            Err(ChainError::NotSupported("unused".to_string()))
        }
        async fn get_transaction(&self, hash: &str) -> crate::chain::Result<TxStatus> {
            Ok(TxStatus {
                hash: TxHash(hash.to_string()),
                status: TxState::Confirmed,
                block_number: Some(9001),
                block_hash: None,
                confirmations: 12,
                gas_used: None,
                effective_gas_price: None,
                error_message: None,
            })
        }
        async fn wait_for_confirmation(
            &self,
            _hash: &TxHash,
            _confirmations: u64,
            _timeout_secs: u64,
        ) -> crate::chain::Result<TxStatus> {
            Err(ChainError::NotSupported("unused".to_string()))
        }
        async fn estimate_fee(&self, _tx: &Transaction) -> crate::chain::Result<FeeEstimate> {
            Err(ChainError::NotSupported("unused".to_string()))
        }
        async fn get_block_number(&self) -> crate::chain::Result<u64> {
            Ok(9012)
        }
    }

    fn test_intent() -> OfframpIntentRow {
        let now = Utc::now();
        OfframpIntentRow {
            id: "ofr_job_001".to_string(),
            tenant_id: "tenant_job_1".to_string(),
            user_id: "user1".to_string(),
            chain_id: Some(137),
            crypto_asset: "USDT".to_string(),
            crypto_amount: Decimal::from(100),
            exchange_rate: Decimal::from(25000),
            locked_rate_id: Some("lock_1".to_string()),
            fees: serde_json::json!({}),
            net_vnd_amount: Decimal::from(2500000),
            gross_vnd_amount: Decimal::from(2500000),
            bank_account: serde_json::json!({}),
            deposit_address: Some("0x2222222222222222222222222222222222222222".to_string()),
            tx_hash: Some("0xjobhash".to_string()),
            bank_reference: None,
            linked_rfq_id: None,
            winning_lp_id: None,
            matched_rate: None,
            settlement_id: None,
            state: "CRYPTO_RECEIVED".to_string(),
            state_history: serde_json::json!([]),
            created_at: now,
            updated_at: now,
            quote_expires_at: now,
        }
    }

    #[test]
    fn map_observation_chain_id_supports_avalanche() {
        assert_eq!(
            map_observation_chain_id(43114),
            Some((ObservationChainId::Avalanche, RuntimeChainId::AVALANCHE))
        );
    }

    #[tokio::test]
    async fn process_pending_confirms_observed_offramp_transaction() {
        let tenant_repo = Arc::new(SpyTenantRepository {
            tenant_ids: vec![TenantId::new("tenant_job_1")],
        });
        let offramp_repo = Arc::new(SpyOfframpIntentRepository::default());
        let observation_repo = Arc::new(SpyOnchainObservationRepository::default());
        offramp_repo.create_intent(&test_intent()).await.unwrap();
        observation_repo
            .observations
            .lock()
            .unwrap()
            .push(OnchainObservationRow {
                id: "obs_job_1".to_string(),
                tenant_id: "tenant_job_1".to_string(),
                intent_id: None,
                offramp_intent_id: Some("ofr_job_001".to_string()),
                tx_hash: "0xjobhash".to_string(),
                chain_id: 137,
                asset_code: "USDT".to_string(),
                amount: Decimal::from(100),
                from_address: "0x1111111111111111111111111111111111111111".to_string(),
                to_address: "0x2222222222222222222222222222222222222222".to_string(),
                status: "OBSERVED".to_string(),
                confirmations: 0,
                required_confirmations: 12,
                block_number: None,
                observation_source: "portal_offramp_crypto_received".to_string(),
                raw_payload: None,
                metadata: serde_json::json!({}),
                observed_at: Utc::now(),
                confirmed_at: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            });

        let mut chain_layer = ChainAbstractionLayer::new();
        chain_layer.register(Arc::new(MockConfirmedChain));
        let job = OfframpConfirmationJob::new(
            tenant_repo,
            offramp_repo,
            observation_repo.clone(),
            Arc::new(chain_layer),
        );

        let processed = job.process_pending().await.unwrap();
        assert_eq!(processed, 1);

        let writes = observation_repo.writes.lock().unwrap();
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].observation_source, "offramp_monitor_confirm");
        assert_eq!(writes[0].status, OnchainObservationStatus::Confirmed);
        assert_eq!(writes[0].block_number, Some(12345));
    }

    #[tokio::test]
    async fn process_pending_skips_invalid_chain_mismatch_observation_and_continues() {
        let tenant_repo = Arc::new(SpyTenantRepository {
            tenant_ids: vec![TenantId::new("tenant_job_1")],
        });
        let offramp_repo = Arc::new(SpyOfframpIntentRepository::default());
        let observation_repo = Arc::new(SpyOnchainObservationRepository::default());
        offramp_repo.create_intent(&test_intent()).await.unwrap();
        observation_repo.observations.lock().unwrap().extend([
            OnchainObservationRow {
                id: "obs_bad_chain".to_string(),
                tenant_id: "tenant_job_1".to_string(),
                intent_id: None,
                offramp_intent_id: Some("ofr_job_001".to_string()),
                tx_hash: "0xjobhash".to_string(),
                chain_id: 56,
                asset_code: "USDT".to_string(),
                amount: Decimal::from(100),
                from_address: "0x1111111111111111111111111111111111111111".to_string(),
                to_address: "0x2222222222222222222222222222222222222222".to_string(),
                status: "OBSERVED".to_string(),
                confirmations: 0,
                required_confirmations: 12,
                block_number: None,
                observation_source: "portal_offramp_crypto_received".to_string(),
                raw_payload: None,
                metadata: serde_json::json!({}),
                observed_at: Utc::now(),
                confirmed_at: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            OnchainObservationRow {
                id: "obs_good_chain".to_string(),
                tenant_id: "tenant_job_1".to_string(),
                intent_id: None,
                offramp_intent_id: Some("ofr_job_001".to_string()),
                tx_hash: "0xjobhash".to_string(),
                chain_id: 137,
                asset_code: "USDT".to_string(),
                amount: Decimal::from(100),
                from_address: "0x1111111111111111111111111111111111111111".to_string(),
                to_address: "0x2222222222222222222222222222222222222222".to_string(),
                status: "OBSERVED".to_string(),
                confirmations: 0,
                required_confirmations: 12,
                block_number: None,
                observation_source: "portal_offramp_crypto_received".to_string(),
                raw_payload: None,
                metadata: serde_json::json!({}),
                observed_at: Utc::now(),
                confirmed_at: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        ]);

        let mut chain_layer = ChainAbstractionLayer::new();
        chain_layer.register(Arc::new(MockConfirmedBscChain));
        chain_layer.register(Arc::new(MockConfirmedChain));
        let job = OfframpConfirmationJob::new(
            tenant_repo,
            offramp_repo,
            observation_repo.clone(),
            Arc::new(chain_layer),
        );

        let processed = job.process_pending().await.unwrap();
        assert_eq!(processed, 1);

        let writes = observation_repo.writes.lock().unwrap();
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].chain_id, 137);
        assert_eq!(writes[0].status, OnchainObservationStatus::Confirmed);
    }
}
