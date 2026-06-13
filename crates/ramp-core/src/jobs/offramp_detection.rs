use crate::chain::{
    ChainAbstractionLayer, ChainId as RuntimeChainId, InboundTransferQuery,
    NativeInboundTransferQuery,
};
use crate::repository::{OfframpIntentRepository, OnchainObservationRepository, TenantRepository};
use crate::service::{OfframpObservationService, RecordOfframpObservationRequest};
use crate::stablecoin::StablecoinRegistry;
use ramp_common::{
    types::{ChainId as ObservationChainId, CryptoSymbol, TxHash, WalletAddress},
    Result,
};
use rust_decimal::Decimal;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

pub struct OfframpDetectionJob {
    tenant_repo: Arc<dyn TenantRepository>,
    offramp_repo: Arc<dyn OfframpIntentRepository>,
    observation_repo: Arc<dyn OnchainObservationRepository>,
    chain_layer: Arc<ChainAbstractionLayer>,
    poll_interval: Duration,
    batch_size: usize,
    lookback_blocks: u64,
}

impl OfframpDetectionJob {
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
            lookback_blocks: 256,
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

    pub fn with_lookback_blocks(mut self, lookback_blocks: u64) -> Self {
        self.lookback_blocks = lookback_blocks;
        self
    }

    pub async fn run(&self) {
        info!(
            poll_interval_secs = self.poll_interval.as_secs(),
            batch_size = self.batch_size,
            lookback_blocks = self.lookback_blocks,
            "Starting off-ramp detection job"
        );

        let mut interval = tokio::time::interval(self.poll_interval);
        loop {
            interval.tick().await;
            match self.process_pending().await {
                Ok(count) if count > 0 => {
                    info!(
                        processed = count,
                        "Off-ramp detection job processed intents"
                    );
                }
                Ok(_) => {}
                Err(error) => {
                    error!(error = %error, "Off-ramp detection job failed");
                }
            }
        }
    }

    pub async fn process_pending(&self) -> Result<usize> {
        let tenant_ids = self.tenant_repo.list_ids().await?;
        let stablecoin_registry = StablecoinRegistry::new();
        let observation_service = OfframpObservationService::new(
            self.offramp_repo.clone(),
            self.observation_repo.clone(),
        );
        let mut processed = 0usize;

        for tenant_id in tenant_ids {
            let intents = self
                .offramp_repo
                .list_by_status(&tenant_id, "CRYPTO_PENDING", self.batch_size as i64)
                .await?;

            for intent in intents {
                let Some(chain_id_value) = intent.chain_id else {
                    continue;
                };
                let Some((service_chain_id, runtime_chain_id)) =
                    map_detect_chain_id(chain_id_value)
                else {
                    continue;
                };
                let Some(deposit_address) = intent.deposit_address.as_deref() else {
                    continue;
                };
                let Some(detect_asset) = parse_detect_asset(&intent.crypto_asset) else {
                    continue;
                };

                let chain = match self.chain_layer.get_chain(runtime_chain_id) {
                    Ok(chain) => chain,
                    Err(error) => {
                        warn!(
                            offramp_intent_id = %intent.id,
                            chain_id = runtime_chain_id.0,
                            error = %error,
                            "Skipping off-ramp detect because runtime chain is unavailable"
                        );
                        continue;
                    }
                };
                let current_block = match chain.get_block_number().await {
                    Ok(block_number) => block_number,
                    Err(error) => {
                        warn!(
                            offramp_intent_id = %intent.id,
                            chain_id = runtime_chain_id.0,
                            error = %error,
                            "Skipping off-ramp detect because block height lookup failed"
                        );
                        continue;
                    }
                };
                let expected_amount =
                    decimal_to_raw_amount(intent.crypto_amount, detect_asset.decimals());
                let (transfers, symbol) = match detect_asset {
                    DetectAsset::Erc20Stablecoin {
                        symbol_name,
                        symbol,
                    } => {
                        let Some(stablecoin) = stablecoin_registry.get_token(symbol_name) else {
                            warn!(
                                offramp_intent_id = %intent.id,
                                asset = %intent.crypto_asset,
                                "Skipping off-ramp detect for unsupported stablecoin"
                            );
                            continue;
                        };
                        let Some(token_address) = stablecoin.contract_address(runtime_chain_id.0)
                        else {
                            warn!(
                                offramp_intent_id = %intent.id,
                                asset = %intent.crypto_asset,
                                chain_id = runtime_chain_id.0,
                                "Skipping off-ramp detect because stablecoin contract is unknown on chain"
                            );
                            continue;
                        };
                        match self
                            .chain_layer
                            .find_inbound_transfers(
                                runtime_chain_id,
                                &InboundTransferQuery {
                                    token_address: format!("{:#x}", token_address),
                                    to_address: deposit_address.to_string(),
                                    from_block: current_block.saturating_sub(self.lookback_blocks),
                                    to_block: current_block,
                                },
                            )
                            .await
                        {
                            Ok(transfers) => (transfers, symbol),
                            Err(error) => {
                                warn!(
                                    offramp_intent_id = %intent.id,
                                    chain_id = runtime_chain_id.0,
                                    error = %error,
                                    "Skipping off-ramp ERC20 detect after inbound transfer lookup failed"
                                );
                                continue;
                            }
                        }
                    }
                    DetectAsset::NativeAsset(symbol) => {
                        if !supports_native_asset(runtime_chain_id, symbol) {
                            continue;
                        }
                        match self
                            .chain_layer
                            .find_native_inbound_transfers(
                                runtime_chain_id,
                                &NativeInboundTransferQuery {
                                    to_address: deposit_address.to_string(),
                                    from_block: current_block.saturating_sub(self.lookback_blocks),
                                    to_block: current_block,
                                },
                            )
                            .await
                        {
                            Ok(transfers) => (transfers, symbol),
                            Err(error) => {
                                warn!(
                                    offramp_intent_id = %intent.id,
                                    chain_id = runtime_chain_id.0,
                                    error = %error,
                                    "Skipping off-ramp native detect after inbound transaction lookup failed"
                                );
                                continue;
                            }
                        }
                    }
                };

                let Some(observed_transfer) = transfers.into_iter().find(|transfer| {
                    transfer.amount == expected_amount
                        && transfer.to_address.eq_ignore_ascii_case(deposit_address)
                }) else {
                    continue;
                };

                observation_service
                    .record_detected(RecordOfframpObservationRequest {
                        tenant_id: tenant_id.clone(),
                        offramp_intent_id: intent.id.clone(),
                        chain_id: service_chain_id,
                        tx_hash: TxHash::new(observed_transfer.tx_hash.0),
                        from_address: WalletAddress::new(observed_transfer.from_address),
                        to_address: WalletAddress::new(observed_transfer.to_address),
                        amount: intent.crypto_amount,
                        symbol,
                        metadata: json!({
                            "source_kind": "background_detection_job",
                            "lookbackBlocks": self.lookback_blocks,
                            "matchedAmountRaw": expected_amount,
                            "matchedBlockNumber": observed_transfer.block_number,
                        }),
                    })
                    .await?;
                processed += 1;
            }
        }

        Ok(processed)
    }
}

enum DetectAsset {
    Erc20Stablecoin {
        symbol_name: &'static str,
        symbol: CryptoSymbol,
    },
    NativeAsset(CryptoSymbol),
}

fn parse_detect_asset(asset: &str) -> Option<DetectAsset> {
    match asset.to_uppercase().as_str() {
        "USDT" => Some(DetectAsset::Erc20Stablecoin {
            symbol_name: "USDT",
            symbol: CryptoSymbol::USDT,
        }),
        "USDC" => Some(DetectAsset::Erc20Stablecoin {
            symbol_name: "USDC",
            symbol: CryptoSymbol::USDC,
        }),
        "ETH" => Some(DetectAsset::NativeAsset(CryptoSymbol::ETH)),
        "BNB" => Some(DetectAsset::NativeAsset(CryptoSymbol::BNB)),
        "MATIC" => Some(DetectAsset::NativeAsset(CryptoSymbol::MATIC)),
        "SOL" => Some(DetectAsset::NativeAsset(CryptoSymbol::SOL)),
        _ => None,
    }
}

fn map_detect_chain_id(chain_id: i64) -> Option<(ObservationChainId, RuntimeChainId)> {
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

fn decimal_to_raw_amount(amount: Decimal, decimals: u8) -> String {
    let multiplier = Decimal::new(10i64.pow(decimals as u32), 0);
    (amount * multiplier).trunc().to_string()
}

impl DetectAsset {
    fn decimals(&self) -> u8 {
        match self {
            Self::Erc20Stablecoin { .. } => 6,
            Self::NativeAsset(CryptoSymbol::SOL) => 9,
            Self::NativeAsset(_) => 18,
        }
    }
}

fn supports_native_asset(chain_id: RuntimeChainId, symbol: CryptoSymbol) -> bool {
    match symbol {
        CryptoSymbol::ETH => matches!(
            chain_id,
            RuntimeChainId::ETHEREUM
                | RuntimeChainId::ARBITRUM
                | RuntimeChainId::OPTIMISM
                | RuntimeChainId::BASE
        ),
        CryptoSymbol::BNB => matches!(chain_id, RuntimeChainId::BSC),
        CryptoSymbol::MATIC => matches!(chain_id, RuntimeChainId::POLYGON),
        CryptoSymbol::SOL => matches!(chain_id, RuntimeChainId::SOLANA_MAINNET),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::{
        Balance, Chain, ChainError, ChainType, FeeEstimate, ObservedInboundTransfer, TokenBalance,
        Transaction, TxStatus, UnifiedAddress,
    };
    use crate::repository::{
        OfframpIntentRow, OnchainObservationRow, OnchainObservationStatus,
        UpsertOnchainObservationRequest,
    };
    use async_trait::async_trait;
    use chrono::Utc;
    use ramp_common::types::TenantId;
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
                id: "OCO_DETECT_TEST".to_string(),
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
            self.observations.lock().unwrap().push(row.clone());
            Ok(row)
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

    struct MockDetectChain;

    #[async_trait]
    impl Chain for MockDetectChain {
        fn chain_id(&self) -> RuntimeChainId {
            RuntimeChainId::POLYGON
        }
        fn name(&self) -> &str {
            "Mock Polygon"
        }
        fn chain_type(&self) -> ChainType {
            ChainType::Evm
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
        async fn send_transaction(
            &self,
            _tx: Transaction,
        ) -> crate::chain::Result<crate::chain::TxHash> {
            Err(ChainError::NotSupported("unused".to_string()))
        }
        async fn get_transaction(&self, _hash: &str) -> crate::chain::Result<TxStatus> {
            Err(ChainError::NotSupported("unused".to_string()))
        }
        async fn wait_for_confirmation(
            &self,
            _hash: &crate::chain::TxHash,
            _confirmations: u64,
            _timeout_secs: u64,
        ) -> crate::chain::Result<TxStatus> {
            Err(ChainError::NotSupported("unused".to_string()))
        }
        async fn estimate_fee(&self, _tx: &Transaction) -> crate::chain::Result<FeeEstimate> {
            Err(ChainError::NotSupported("unused".to_string()))
        }
        async fn get_block_number(&self) -> crate::chain::Result<u64> {
            Ok(1200)
        }
        async fn find_inbound_transfers(
            &self,
            query: &InboundTransferQuery,
        ) -> crate::chain::Result<Vec<ObservedInboundTransfer>> {
            assert_eq!(
                query.to_address,
                "0x2222222222222222222222222222222222222222"
            );
            Ok(vec![ObservedInboundTransfer {
                tx_hash: crate::chain::TxHash("0xdetectedhash".to_string()),
                from_address: "0x1111111111111111111111111111111111111111".to_string(),
                to_address: query.to_address.clone(),
                amount: "100000000".to_string(),
                block_number: 1199,
            }])
        }
    }

    fn test_intent() -> OfframpIntentRow {
        let now = Utc::now();
        OfframpIntentRow {
            id: "ofr_detect_001".to_string(),
            tenant_id: "tenant_detect_1".to_string(),
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
            tx_hash: None,
            bank_reference: None,
            linked_rfq_id: None,
            winning_lp_id: None,
            matched_rate: None,
            settlement_id: None,
            state: "CRYPTO_PENDING".to_string(),
            state_history: serde_json::json!([]),
            created_at: now,
            updated_at: now,
            quote_expires_at: now,
        }
    }

    fn test_bsc_intent() -> OfframpIntentRow {
        let mut intent = test_intent();
        intent.id = "ofr_detect_bsc_001".to_string();
        intent.chain_id = Some(56);
        intent
    }

    fn test_eth_intent() -> OfframpIntentRow {
        let mut intent = test_intent();
        intent.id = "ofr_detect_eth_001".to_string();
        intent.chain_id = Some(1);
        intent.crypto_asset = "ETH".to_string();
        intent.crypto_amount = Decimal::new(15, 1);
        intent
    }

    fn test_bnb_intent() -> OfframpIntentRow {
        let mut intent = test_intent();
        intent.id = "ofr_detect_bnb_001".to_string();
        intent.chain_id = Some(56);
        intent.crypto_asset = "BNB".to_string();
        intent.crypto_amount = Decimal::new(2, 0);
        intent
    }

    fn test_matic_intent() -> OfframpIntentRow {
        let mut intent = test_intent();
        intent.id = "ofr_detect_matic_001".to_string();
        intent.chain_id = Some(137);
        intent.crypto_asset = "MATIC".to_string();
        intent.crypto_amount = Decimal::new(35, 1);
        intent
    }

    fn test_sol_intent() -> OfframpIntentRow {
        let mut intent = test_intent();
        intent.id = "ofr_detect_sol_001".to_string();
        intent.chain_id = Some(101);
        intent.crypto_asset = "SOL".to_string();
        intent.crypto_amount = Decimal::new(25, 1);
        intent.deposit_address = Some("11111111111111111111111111111111".to_string());
        intent
    }

    #[test]
    fn map_detect_chain_id_supports_avalanche() {
        assert_eq!(
            map_detect_chain_id(43114),
            Some((ObservationChainId::Avalanche, RuntimeChainId::AVALANCHE))
        );
    }

    #[tokio::test]
    async fn process_pending_detects_inbound_offramp_transfer() {
        let tenant_repo = Arc::new(SpyTenantRepository {
            tenant_ids: vec![TenantId::new("tenant_detect_1")],
        });
        let offramp_repo = Arc::new(SpyOfframpIntentRepository::default());
        let observation_repo = Arc::new(SpyOnchainObservationRepository::default());
        offramp_repo.create_intent(&test_intent()).await.unwrap();

        let mut chain_layer = ChainAbstractionLayer::new();
        chain_layer.register(Arc::new(MockDetectChain));
        let job = OfframpDetectionJob::new(
            tenant_repo,
            offramp_repo.clone(),
            observation_repo.clone(),
            Arc::new(chain_layer),
        )
        .with_lookback_blocks(64);

        let processed = job.process_pending().await.unwrap();
        assert_eq!(processed, 1);

        let intent = offramp_repo
            .get_intent(&TenantId::new("tenant_detect_1"), "ofr_detect_001")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(intent.state, "CRYPTO_RECEIVED");
        assert_eq!(intent.tx_hash.as_deref(), Some("0xdetectedhash"));

        let writes = observation_repo.writes.lock().unwrap();
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].observation_source, "offramp_monitor_detect");
        assert_eq!(writes[0].status, OnchainObservationStatus::Observed);
        assert_eq!(writes[0].chain_id, 137);
    }

    #[tokio::test]
    async fn process_pending_detects_bsc_inbound_transfer_when_runtime_chain_is_registered() {
        let tenant_repo = Arc::new(SpyTenantRepository {
            tenant_ids: vec![TenantId::new("tenant_detect_1")],
        });
        let offramp_repo = Arc::new(SpyOfframpIntentRepository::default());
        let observation_repo = Arc::new(SpyOnchainObservationRepository::default());
        offramp_repo
            .create_intent(&test_bsc_intent())
            .await
            .unwrap();

        struct MockBscDetectChain;

        #[async_trait]
        impl Chain for MockBscDetectChain {
            fn chain_id(&self) -> RuntimeChainId {
                RuntimeChainId::BSC
            }
            fn name(&self) -> &str {
                "Mock BSC"
            }
            fn chain_type(&self) -> ChainType {
                ChainType::Evm
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
            async fn send_transaction(
                &self,
                _tx: Transaction,
            ) -> crate::chain::Result<crate::chain::TxHash> {
                Err(ChainError::NotSupported("unused".to_string()))
            }
            async fn get_transaction(&self, _hash: &str) -> crate::chain::Result<TxStatus> {
                Err(ChainError::NotSupported("unused".to_string()))
            }
            async fn wait_for_confirmation(
                &self,
                _hash: &crate::chain::TxHash,
                _confirmations: u64,
                _timeout_secs: u64,
            ) -> crate::chain::Result<TxStatus> {
                Err(ChainError::NotSupported("unused".to_string()))
            }
            async fn estimate_fee(&self, _tx: &Transaction) -> crate::chain::Result<FeeEstimate> {
                Err(ChainError::NotSupported("unused".to_string()))
            }
            async fn get_block_number(&self) -> crate::chain::Result<u64> {
                Ok(2200)
            }
            async fn find_inbound_transfers(
                &self,
                query: &InboundTransferQuery,
            ) -> crate::chain::Result<Vec<ObservedInboundTransfer>> {
                assert_eq!(
                    query.to_address,
                    "0x2222222222222222222222222222222222222222"
                );
                Ok(vec![ObservedInboundTransfer {
                    tx_hash: crate::chain::TxHash("0xbscdetectedhash".to_string()),
                    from_address: "0x1111111111111111111111111111111111111111".to_string(),
                    to_address: query.to_address.clone(),
                    amount: "100000000".to_string(),
                    block_number: 2199,
                }])
            }
        }

        let mut chain_layer = ChainAbstractionLayer::new();
        chain_layer.register(Arc::new(MockBscDetectChain));
        let job = OfframpDetectionJob::new(
            tenant_repo,
            offramp_repo.clone(),
            observation_repo.clone(),
            Arc::new(chain_layer),
        )
        .with_lookback_blocks(64);

        let processed = job.process_pending().await.unwrap();
        assert_eq!(processed, 1);

        let intent = offramp_repo
            .get_intent(&TenantId::new("tenant_detect_1"), "ofr_detect_bsc_001")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(intent.state, "CRYPTO_RECEIVED");
        assert_eq!(intent.tx_hash.as_deref(), Some("0xbscdetectedhash"));

        let writes = observation_repo.writes.lock().unwrap();
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].chain_id, 56);
        assert_eq!(writes[0].observation_source, "offramp_monitor_detect");
    }

    #[tokio::test]
    async fn process_pending_detects_native_eth_on_evm_runtime() {
        let tenant_repo = Arc::new(SpyTenantRepository {
            tenant_ids: vec![TenantId::new("tenant_detect_1")],
        });
        let offramp_repo = Arc::new(SpyOfframpIntentRepository::default());
        let observation_repo = Arc::new(SpyOnchainObservationRepository::default());
        offramp_repo
            .create_intent(&test_eth_intent())
            .await
            .unwrap();

        struct MockEthDetectChain;

        #[async_trait]
        impl Chain for MockEthDetectChain {
            fn chain_id(&self) -> RuntimeChainId {
                RuntimeChainId::ETHEREUM
            }
            fn name(&self) -> &str {
                "Mock Ethereum"
            }
            fn chain_type(&self) -> ChainType {
                ChainType::Evm
            }
            fn is_testnet(&self) -> bool {
                false
            }
            fn native_symbol(&self) -> &str {
                "ETH"
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
            async fn send_transaction(
                &self,
                _tx: Transaction,
            ) -> crate::chain::Result<crate::chain::TxHash> {
                Err(ChainError::NotSupported("unused".to_string()))
            }
            async fn get_transaction(&self, _hash: &str) -> crate::chain::Result<TxStatus> {
                Err(ChainError::NotSupported("unused".to_string()))
            }
            async fn wait_for_confirmation(
                &self,
                _hash: &crate::chain::TxHash,
                _confirmations: u64,
                _timeout_secs: u64,
            ) -> crate::chain::Result<TxStatus> {
                Err(ChainError::NotSupported("unused".to_string()))
            }
            async fn estimate_fee(&self, _tx: &Transaction) -> crate::chain::Result<FeeEstimate> {
                Err(ChainError::NotSupported("unused".to_string()))
            }
            async fn get_block_number(&self) -> crate::chain::Result<u64> {
                Ok(3200)
            }
            async fn find_native_inbound_transfers(
                &self,
                query: &NativeInboundTransferQuery,
            ) -> crate::chain::Result<Vec<ObservedInboundTransfer>> {
                assert_eq!(
                    query.to_address,
                    "0x2222222222222222222222222222222222222222"
                );
                Ok(vec![ObservedInboundTransfer {
                    tx_hash: crate::chain::TxHash("0xethdetectedhash".to_string()),
                    from_address: "0x1111111111111111111111111111111111111111".to_string(),
                    to_address: query.to_address.clone(),
                    amount: "1500000000000000000".to_string(),
                    block_number: 3199,
                }])
            }
        }

        let mut chain_layer = ChainAbstractionLayer::new();
        chain_layer.register(Arc::new(MockEthDetectChain));
        let job = OfframpDetectionJob::new(
            tenant_repo,
            offramp_repo.clone(),
            observation_repo.clone(),
            Arc::new(chain_layer),
        )
        .with_lookback_blocks(64);

        let processed = job.process_pending().await.unwrap();
        assert_eq!(processed, 1);

        let intent = offramp_repo
            .get_intent(&TenantId::new("tenant_detect_1"), "ofr_detect_eth_001")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(intent.state, "CRYPTO_RECEIVED");
        assert_eq!(intent.tx_hash.as_deref(), Some("0xethdetectedhash"));

        let writes = observation_repo.writes.lock().unwrap();
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].chain_id, 1);
        assert_eq!(writes[0].asset_code, "ETH");
        assert_eq!(writes[0].amount, Decimal::new(15, 1));
    }

    #[tokio::test]
    async fn process_pending_detects_native_bnb_on_bsc_runtime() {
        let tenant_repo = Arc::new(SpyTenantRepository {
            tenant_ids: vec![TenantId::new("tenant_detect_1")],
        });
        let offramp_repo = Arc::new(SpyOfframpIntentRepository::default());
        let observation_repo = Arc::new(SpyOnchainObservationRepository::default());
        offramp_repo
            .create_intent(&test_bnb_intent())
            .await
            .unwrap();

        struct MockNativeBscDetectChain;

        #[async_trait]
        impl Chain for MockNativeBscDetectChain {
            fn chain_id(&self) -> RuntimeChainId {
                RuntimeChainId::BSC
            }
            fn name(&self) -> &str {
                "Mock Native BSC"
            }
            fn chain_type(&self) -> ChainType {
                ChainType::Evm
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
            async fn send_transaction(
                &self,
                _tx: Transaction,
            ) -> crate::chain::Result<crate::chain::TxHash> {
                Err(ChainError::NotSupported("unused".to_string()))
            }
            async fn get_transaction(&self, _hash: &str) -> crate::chain::Result<TxStatus> {
                Err(ChainError::NotSupported("unused".to_string()))
            }
            async fn wait_for_confirmation(
                &self,
                _hash: &crate::chain::TxHash,
                _confirmations: u64,
                _timeout_secs: u64,
            ) -> crate::chain::Result<TxStatus> {
                Err(ChainError::NotSupported("unused".to_string()))
            }
            async fn estimate_fee(&self, _tx: &Transaction) -> crate::chain::Result<FeeEstimate> {
                Err(ChainError::NotSupported("unused".to_string()))
            }
            async fn get_block_number(&self) -> crate::chain::Result<u64> {
                Ok(4200)
            }
            async fn find_native_inbound_transfers(
                &self,
                query: &NativeInboundTransferQuery,
            ) -> crate::chain::Result<Vec<ObservedInboundTransfer>> {
                assert_eq!(
                    query.to_address,
                    "0x2222222222222222222222222222222222222222"
                );
                Ok(vec![ObservedInboundTransfer {
                    tx_hash: crate::chain::TxHash("0xbnbdetectedhash".to_string()),
                    from_address: "0x1111111111111111111111111111111111111111".to_string(),
                    to_address: query.to_address.clone(),
                    amount: "2000000000000000000".to_string(),
                    block_number: 4199,
                }])
            }
        }

        let mut chain_layer = ChainAbstractionLayer::new();
        chain_layer.register(Arc::new(MockNativeBscDetectChain));
        let job = OfframpDetectionJob::new(
            tenant_repo,
            offramp_repo.clone(),
            observation_repo.clone(),
            Arc::new(chain_layer),
        )
        .with_lookback_blocks(64);

        let processed = job.process_pending().await.unwrap();
        assert_eq!(processed, 1);

        let intent = offramp_repo
            .get_intent(&TenantId::new("tenant_detect_1"), "ofr_detect_bnb_001")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(intent.state, "CRYPTO_RECEIVED");
        assert_eq!(intent.tx_hash.as_deref(), Some("0xbnbdetectedhash"));

        let writes = observation_repo.writes.lock().unwrap();
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].chain_id, 56);
        assert_eq!(writes[0].asset_code, "BNB");
        assert_eq!(writes[0].amount, Decimal::new(2, 0));
    }

    #[tokio::test]
    async fn process_pending_detects_native_matic_on_polygon_runtime() {
        let tenant_repo = Arc::new(SpyTenantRepository {
            tenant_ids: vec![TenantId::new("tenant_detect_1")],
        });
        let offramp_repo = Arc::new(SpyOfframpIntentRepository::default());
        let observation_repo = Arc::new(SpyOnchainObservationRepository::default());
        offramp_repo
            .create_intent(&test_matic_intent())
            .await
            .unwrap();

        struct MockNativePolygonDetectChain;

        #[async_trait]
        impl Chain for MockNativePolygonDetectChain {
            fn chain_id(&self) -> RuntimeChainId {
                RuntimeChainId::POLYGON
            }
            fn name(&self) -> &str {
                "Mock Native Polygon"
            }
            fn chain_type(&self) -> ChainType {
                ChainType::Evm
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
            async fn send_transaction(
                &self,
                _tx: Transaction,
            ) -> crate::chain::Result<crate::chain::TxHash> {
                Err(ChainError::NotSupported("unused".to_string()))
            }
            async fn get_transaction(&self, _hash: &str) -> crate::chain::Result<TxStatus> {
                Err(ChainError::NotSupported("unused".to_string()))
            }
            async fn wait_for_confirmation(
                &self,
                _hash: &crate::chain::TxHash,
                _confirmations: u64,
                _timeout_secs: u64,
            ) -> crate::chain::Result<TxStatus> {
                Err(ChainError::NotSupported("unused".to_string()))
            }
            async fn estimate_fee(&self, _tx: &Transaction) -> crate::chain::Result<FeeEstimate> {
                Err(ChainError::NotSupported("unused".to_string()))
            }
            async fn get_block_number(&self) -> crate::chain::Result<u64> {
                Ok(6200)
            }
            async fn find_native_inbound_transfers(
                &self,
                query: &NativeInboundTransferQuery,
            ) -> crate::chain::Result<Vec<ObservedInboundTransfer>> {
                assert_eq!(
                    query.to_address,
                    "0x2222222222222222222222222222222222222222"
                );
                Ok(vec![ObservedInboundTransfer {
                    tx_hash: crate::chain::TxHash("0xmaticdetectedhash".to_string()),
                    from_address: "0x1111111111111111111111111111111111111111".to_string(),
                    to_address: query.to_address.clone(),
                    amount: "3500000000000000000".to_string(),
                    block_number: 6199,
                }])
            }
        }

        let mut chain_layer = ChainAbstractionLayer::new();
        chain_layer.register(Arc::new(MockNativePolygonDetectChain));
        let job = OfframpDetectionJob::new(
            tenant_repo,
            offramp_repo.clone(),
            observation_repo.clone(),
            Arc::new(chain_layer),
        )
        .with_lookback_blocks(64);

        let processed = job.process_pending().await.unwrap();
        assert_eq!(processed, 1);

        let intent = offramp_repo
            .get_intent(&TenantId::new("tenant_detect_1"), "ofr_detect_matic_001")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(intent.state, "CRYPTO_RECEIVED");
        assert_eq!(intent.tx_hash.as_deref(), Some("0xmaticdetectedhash"));

        let writes = observation_repo.writes.lock().unwrap();
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].chain_id, 137);
        assert_eq!(writes[0].asset_code, "MATIC");
        assert_eq!(writes[0].amount, Decimal::new(35, 1));
    }

    #[tokio::test]
    async fn process_pending_detects_native_sol_on_solana_runtime() {
        let tenant_repo = Arc::new(SpyTenantRepository {
            tenant_ids: vec![TenantId::new("tenant_detect_1")],
        });
        let offramp_repo = Arc::new(SpyOfframpIntentRepository::default());
        let observation_repo = Arc::new(SpyOnchainObservationRepository::default());
        offramp_repo
            .create_intent(&test_sol_intent())
            .await
            .unwrap();

        struct MockSolanaDetectChain;

        #[async_trait]
        impl Chain for MockSolanaDetectChain {
            fn chain_id(&self) -> RuntimeChainId {
                RuntimeChainId::SOLANA_MAINNET
            }
            fn name(&self) -> &str {
                "Mock Solana"
            }
            fn chain_type(&self) -> ChainType {
                ChainType::Solana
            }
            fn is_testnet(&self) -> bool {
                false
            }
            fn native_symbol(&self) -> &str {
                "SOL"
            }
            fn explorer_url(&self) -> &str {
                "https://example.test"
            }
            fn validate_address(&self, address: &str) -> crate::chain::Result<UnifiedAddress> {
                Ok(UnifiedAddress {
                    chain_type: ChainType::Solana,
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
            async fn send_transaction(
                &self,
                _tx: Transaction,
            ) -> crate::chain::Result<crate::chain::TxHash> {
                Err(ChainError::NotSupported("unused".to_string()))
            }
            async fn get_transaction(&self, _hash: &str) -> crate::chain::Result<TxStatus> {
                Err(ChainError::NotSupported("unused".to_string()))
            }
            async fn wait_for_confirmation(
                &self,
                _hash: &crate::chain::TxHash,
                _confirmations: u64,
                _timeout_secs: u64,
            ) -> crate::chain::Result<TxStatus> {
                Err(ChainError::NotSupported("unused".to_string()))
            }
            async fn estimate_fee(&self, _tx: &Transaction) -> crate::chain::Result<FeeEstimate> {
                Err(ChainError::NotSupported("unused".to_string()))
            }
            async fn get_block_number(&self) -> crate::chain::Result<u64> {
                Ok(5200)
            }
            async fn find_native_inbound_transfers(
                &self,
                query: &NativeInboundTransferQuery,
            ) -> crate::chain::Result<Vec<ObservedInboundTransfer>> {
                assert_eq!(query.to_address, "11111111111111111111111111111111");
                Ok(vec![ObservedInboundTransfer {
                    tx_hash: crate::chain::TxHash(
                        "5NnYvN2rKxwz1U2s3T4u5V6w7X8y9ZaBcDeFgHiJkLmNoPqRsTuVwXyZ".to_string(),
                    ),
                    from_address: "7cVfgArCheMR6Cs4t6vz5rfnqd56vZq4ndaBrY5xkxXy".to_string(),
                    to_address: query.to_address.clone(),
                    amount: "2500000000".to_string(),
                    block_number: 5199,
                }])
            }
        }

        let mut chain_layer = ChainAbstractionLayer::new();
        chain_layer.register(Arc::new(MockSolanaDetectChain));
        let job = OfframpDetectionJob::new(
            tenant_repo,
            offramp_repo.clone(),
            observation_repo.clone(),
            Arc::new(chain_layer),
        )
        .with_lookback_blocks(64);

        let processed = job.process_pending().await.unwrap();
        assert_eq!(processed, 1);

        let intent = offramp_repo
            .get_intent(&TenantId::new("tenant_detect_1"), "ofr_detect_sol_001")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(intent.state, "CRYPTO_RECEIVED");
        assert_eq!(
            intent.tx_hash.as_deref(),
            Some("5NnYvN2rKxwz1U2s3T4u5V6w7X8y9ZaBcDeFgHiJkLmNoPqRsTuVwXyZ")
        );

        let writes = observation_repo.writes.lock().unwrap();
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].chain_id, 101);
        assert_eq!(writes[0].asset_code, "SOL");
        assert_eq!(writes[0].amount, Decimal::new(25, 1));
    }
}
