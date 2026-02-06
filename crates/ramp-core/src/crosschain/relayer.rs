//! Cross-chain Message Relayer
//!
//! Handles cross-chain message passing and verification for intent execution.

use crate::bridge::{ChainId, TxHash};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ethers::types::{Address, Bytes, H256, U256};
use ramp_common::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Relayer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayerConfig {
    /// RPC endpoints per chain
    pub rpc_endpoints: HashMap<ChainId, String>,
    /// Relayer wallet address
    pub relayer_address: Address,
    /// Minimum confirmations before relaying
    pub min_confirmations: u32,
    /// Maximum gas price (gwei) for relay tx
    pub max_gas_price_gwei: u64,
    /// Retry delay (seconds)
    pub retry_delay_secs: u64,
    /// Maximum retries
    pub max_retries: u32,
}

impl Default for RelayerConfig {
    fn default() -> Self {
        let mut rpc_endpoints = HashMap::new();
        rpc_endpoints.insert(1, "https://eth.llamarpc.com".to_string());
        rpc_endpoints.insert(42161, "https://arb1.arbitrum.io/rpc".to_string());
        rpc_endpoints.insert(8453, "https://mainnet.base.org".to_string());
        rpc_endpoints.insert(10, "https://mainnet.optimism.io".to_string());
        rpc_endpoints.insert(137, "https://polygon-rpc.com".to_string());

        Self {
            rpc_endpoints,
            relayer_address: Address::zero(),
            min_confirmations: 12,
            max_gas_price_gwei: 100,
            retry_delay_secs: 30,
            max_retries: 5,
        }
    }
}

/// Cross-chain message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossChainMessage {
    /// Unique message ID
    pub id: String,
    /// Source chain
    pub source_chain: ChainId,
    /// Destination chain
    pub dest_chain: ChainId,
    /// Source transaction hash
    pub source_tx_hash: TxHash,
    /// Message sender on source chain
    pub sender: Address,
    /// Target contract on destination chain
    pub target: Address,
    /// Encoded message data
    pub data: Bytes,
    /// Message value (if any)
    pub value: U256,
    /// Nonce for ordering
    pub nonce: u64,
    /// Message created timestamp
    pub created_at: DateTime<Utc>,
}

impl CrossChainMessage {
    pub fn new(
        source_chain: ChainId,
        dest_chain: ChainId,
        source_tx_hash: TxHash,
        sender: Address,
        target: Address,
        data: Bytes,
    ) -> Self {
        Self {
            id: format!("msg_{}", uuid::Uuid::now_v7()),
            source_chain,
            dest_chain,
            source_tx_hash,
            sender,
            target,
            data,
            value: U256::zero(),
            nonce: 0,
            created_at: Utc::now(),
        }
    }

    pub fn with_value(mut self, value: U256) -> Self {
        self.value = value;
        self
    }

    pub fn message_hash(&self) -> H256 {
        // In production, compute proper keccak256 hash
        let mut bytes = [0u8; 32];
        bytes[..8].copy_from_slice(&self.source_chain.to_be_bytes());
        bytes[8..16].copy_from_slice(&self.dest_chain.to_be_bytes());
        bytes[16..24].copy_from_slice(&self.nonce.to_be_bytes());
        H256::from(bytes)
    }
}

/// Message relay status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageStatus {
    /// Waiting for source confirmation
    PendingConfirmation,
    /// Ready to relay
    ReadyToRelay,
    /// Relay transaction submitted
    Relaying,
    /// Relay confirmed on destination
    Delivered,
    /// Relay failed
    Failed(String),
    /// Message expired
    Expired,
}

impl MessageStatus {
    pub fn is_final(&self) -> bool {
        matches!(
            self,
            MessageStatus::Delivered | MessageStatus::Failed(_) | MessageStatus::Expired
        )
    }
}

/// Message relay record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayRecord {
    pub message: CrossChainMessage,
    pub status: MessageStatus,
    /// Source chain confirmations
    pub confirmations: u32,
    /// Destination transaction hash
    pub dest_tx_hash: Option<TxHash>,
    /// Gas used for relay
    pub gas_used: Option<U256>,
    /// Relay attempts
    pub attempts: u32,
    /// Last attempt timestamp
    pub last_attempt_at: Option<DateTime<Utc>>,
    /// Error messages from attempts
    pub errors: Vec<String>,
}

impl RelayRecord {
    pub fn new(message: CrossChainMessage) -> Self {
        Self {
            message,
            status: MessageStatus::PendingConfirmation,
            confirmations: 0,
            dest_tx_hash: None,
            gas_used: None,
            attempts: 0,
            last_attempt_at: None,
            errors: Vec::new(),
        }
    }
}

/// Cross-chain relayer trait
#[async_trait]
pub trait Relayer: Send + Sync {
    /// Submit a message for relaying
    async fn submit_message(&self, message: CrossChainMessage) -> Result<String>;

    /// Get message status
    async fn get_status(&self, message_id: &str) -> Result<Option<RelayRecord>>;

    /// Retry failed message
    async fn retry(&self, message_id: &str) -> Result<()>;

    /// Get pending messages for a chain
    async fn get_pending(&self, dest_chain: ChainId) -> Result<Vec<RelayRecord>>;
}

/// Cross-chain relayer implementation
pub struct CrossChainRelayer {
    config: RelayerConfig,
    messages: RwLock<HashMap<String, RelayRecord>>,
}

impl CrossChainRelayer {
    pub fn new(config: RelayerConfig) -> Self {
        Self {
            config,
            messages: RwLock::new(HashMap::new()),
        }
    }

    pub fn with_default_config() -> Self {
        Self::new(RelayerConfig::default())
    }

    /// Check source chain confirmations
    async fn check_confirmations(&self, record: &mut RelayRecord) -> Result<u32> {
        // In production, query the RPC for block confirmations
        // For now, simulate confirmation
        record.confirmations = self.config.min_confirmations;
        Ok(record.confirmations)
    }

    /// Execute relay transaction on destination chain
    async fn execute_relay(&self, record: &mut RelayRecord) -> Result<TxHash> {
        record.status = MessageStatus::Relaying;
        record.attempts += 1;
        record.last_attempt_at = Some(Utc::now());

        // In production:
        // 1. Build relay transaction with message proof
        // 2. Estimate gas
        // 3. Sign and submit transaction
        // 4. Wait for confirmation

        info!(
            message_id = %record.message.id,
            dest_chain = record.message.dest_chain,
            attempt = record.attempts,
            "Executing relay"
        );

        // Mock successful relay
        let tx_hash = TxHash::random();
        record.dest_tx_hash = Some(tx_hash);
        record.gas_used = Some(U256::from(100000u64));
        record.status = MessageStatus::Delivered;

        Ok(tx_hash)
    }

    /// Process pending messages
    pub async fn process_pending(&self) -> Result<u32> {
        let mut processed = 0;

        let message_ids: Vec<String> = {
            let messages = self.messages.read().await;
            messages
                .iter()
                .filter(|(_, r)| !r.status.is_final())
                .map(|(id, _)| id.clone())
                .collect()
        };

        for id in message_ids {
            let mut messages = self.messages.write().await;
            if let Some(record) = messages.get_mut(&id) {
                match &record.status {
                    MessageStatus::PendingConfirmation => {
                        let confirmations = self.check_confirmations(record).await?;
                        if confirmations >= self.config.min_confirmations {
                            record.status = MessageStatus::ReadyToRelay;
                        }
                    }
                    MessageStatus::ReadyToRelay => {
                        match self.execute_relay(record).await {
                            Ok(_) => processed += 1,
                            Err(e) => {
                                warn!(
                                    message_id = %id,
                                    error = %e,
                                    "Relay failed"
                                );
                                record.errors.push(e.to_string());
                                if record.attempts >= self.config.max_retries {
                                    record.status = MessageStatus::Failed(
                                        "Max retries exceeded".to_string()
                                    );
                                } else {
                                    record.status = MessageStatus::ReadyToRelay;
                                }
                            }
                        }
                    }
                    MessageStatus::Relaying => {
                        // Check if relay completed
                        // In production, query for tx receipt
                    }
                    _ => {}
                }
            }
        }

        Ok(processed)
    }

    /// Run relayer loop
    pub async fn run(&self) {
        info!("Starting cross-chain relayer");

        loop {
            match self.process_pending().await {
                Ok(count) if count > 0 => {
                    info!(processed = count, "Processed pending messages");
                }
                Err(e) => {
                    warn!(error = %e, "Error processing messages");
                }
                _ => {}
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(
                self.config.retry_delay_secs,
            ))
            .await;
        }
    }
}

#[async_trait]
impl Relayer for CrossChainRelayer {
    async fn submit_message(&self, message: CrossChainMessage) -> Result<String> {
        let id = message.id.clone();
        let record = RelayRecord::new(message);

        let mut messages = self.messages.write().await;
        messages.insert(id.clone(), record);

        info!(message_id = %id, "Message submitted for relay");
        Ok(id)
    }

    async fn get_status(&self, message_id: &str) -> Result<Option<RelayRecord>> {
        let messages = self.messages.read().await;
        Ok(messages.get(message_id).cloned())
    }

    async fn retry(&self, message_id: &str) -> Result<()> {
        let mut messages = self.messages.write().await;
        let record = messages
            .get_mut(message_id)
            .ok_or_else(|| Error::NotFound(format!("Message {} not found", message_id)))?;

        if record.status.is_final() {
            return Err(Error::Validation("Cannot retry final message".to_string()));
        }

        record.status = MessageStatus::ReadyToRelay;
        Ok(())
    }

    async fn get_pending(&self, dest_chain: ChainId) -> Result<Vec<RelayRecord>> {
        let messages = self.messages.read().await;
        Ok(messages
            .values()
            .filter(|r| r.message.dest_chain == dest_chain && !r.status.is_final())
            .cloned()
            .collect())
    }
}

/// Proof verification for cross-chain messages
pub struct ProofVerifier {
    /// Trusted block headers per chain
    trusted_headers: RwLock<HashMap<ChainId, Vec<H256>>>,
}

impl ProofVerifier {
    pub fn new() -> Self {
        Self {
            trusted_headers: RwLock::new(HashMap::new()),
        }
    }

    /// Verify a message proof
    pub async fn verify_proof(
        &self,
        message: &CrossChainMessage,
        _proof: &[u8],
    ) -> Result<bool> {
        // In production:
        // 1. Decode Merkle proof
        // 2. Verify against trusted block header
        // 3. Validate message hash matches

        info!(
            message_id = %message.id,
            source_chain = message.source_chain,
            "Verifying message proof"
        );

        // Mock verification
        Ok(true)
    }

    /// Update trusted headers
    pub async fn update_header(&self, chain_id: ChainId, header: H256) {
        let mut headers = self.trusted_headers.write().await;
        headers.entry(chain_id).or_default().push(header);

        // Keep only recent headers
        if let Some(chain_headers) = headers.get_mut(&chain_id) {
            if chain_headers.len() > 1000 {
                chain_headers.drain(0..500);
            }
        }
    }
}

impl Default for ProofVerifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relayer_config() {
        let config = RelayerConfig::default();
        assert!(config.rpc_endpoints.contains_key(&1));
        assert!(config.rpc_endpoints.contains_key(&42161));
        assert_eq!(config.min_confirmations, 12);
    }

    #[test]
    fn test_message_creation() {
        let message = CrossChainMessage::new(
            1,
            42161,
            TxHash::random(),
            Address::zero(),
            Address::zero(),
            Bytes::new(),
        );

        assert!(message.id.starts_with("msg_"));
        assert_eq!(message.source_chain, 1);
        assert_eq!(message.dest_chain, 42161);
    }

    #[test]
    fn test_message_status() {
        assert!(!MessageStatus::Relaying.is_final());
        assert!(MessageStatus::Delivered.is_final());
        assert!(MessageStatus::Failed("error".to_string()).is_final());
    }

    #[tokio::test]
    async fn test_submit_message() {
        let relayer = CrossChainRelayer::with_default_config();

        let message = CrossChainMessage::new(
            1,
            42161,
            TxHash::random(),
            Address::zero(),
            Address::zero(),
            Bytes::new(),
        );

        let id = relayer.submit_message(message).await.unwrap();
        assert!(id.starts_with("msg_"));

        let status = relayer.get_status(&id).await.unwrap();
        assert!(status.is_some());
    }

    #[tokio::test]
    async fn test_get_pending() {
        let relayer = CrossChainRelayer::with_default_config();

        let message = CrossChainMessage::new(
            1,
            42161,
            TxHash::random(),
            Address::zero(),
            Address::zero(),
            Bytes::new(),
        );

        relayer.submit_message(message).await.unwrap();

        let pending = relayer.get_pending(42161).await.unwrap();
        assert_eq!(pending.len(), 1);
    }

    #[tokio::test]
    async fn test_proof_verifier() {
        let verifier = ProofVerifier::new();

        let message = CrossChainMessage::new(
            1,
            42161,
            TxHash::random(),
            Address::zero(),
            Address::zero(),
            Bytes::new(),
        );

        let result = verifier.verify_proof(&message, &[]).await.unwrap();
        assert!(result);
    }
}
