//! Cross-chain Message Relayer
//!
//! Handles cross-chain message passing and verification for intent execution.

use crate::bridge::{ChainId, TxHash};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use alloy::primitives::{Address, Bytes, B256, U256};
use ramp_common::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
            relayer_address: Address::ZERO,
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
            value: U256::ZERO,
            nonce: 0,
            created_at: Utc::now(),
        }
    }

    pub fn with_value(mut self, value: U256) -> Self {
        self.value = value;
        self
    }

    pub fn message_hash(&self) -> B256 {
        // In production, compute proper keccak256 hash
        let mut bytes = [0u8; 32];
        bytes[..8].copy_from_slice(&self.source_chain.to_be_bytes());
        bytes[8..16].copy_from_slice(&self.dest_chain.to_be_bytes());
        bytes[16..24].copy_from_slice(&self.nonce.to_be_bytes());
        B256::from(bytes)
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

#[allow(dead_code)]
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
#[allow(dead_code)]
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
        let tx_hash = B256::from(rand::random::<[u8; 32]>());
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
#[allow(dead_code)]
pub struct ProofVerifier {
    /// Trusted block headers per chain
    trusted_headers: RwLock<HashMap<ChainId, Vec<B256>>>,
}

#[allow(dead_code)]
impl ProofVerifier {
    pub fn new() -> Self {
        Self {
            trusted_headers: RwLock::new(HashMap::new()),
        }
    }

    // SECURITY WARNING: Proof verification is currently a placeholder.
    // In production, this MUST implement real Merkle proof verification
    // against trusted block headers. Without proper verification, an attacker
    // could forge cross-chain messages and steal funds.
    // TODO(security): Implement full Merkle Patricia proof verification before mainnet.
    /// Verify a message proof
    pub async fn verify_proof(
        &self,
        message: &CrossChainMessage,
        proof: &[u8],
    ) -> Result<bool> {
        // Basic sanity checks to reject obviously invalid proofs.
        // These are NOT sufficient for production security - real Merkle proof
        // verification must be implemented before deployment.
        if proof.is_empty() {
            return Err(Error::Validation(
                "Proof verification failed: proof data is empty".to_string(),
            ));
        }

        // Minimum proof length check - a valid Merkle proof requires at least
        // a 32-byte hash node
        if proof.len() < 32 {
            return Err(Error::Validation(
                "Proof verification failed: proof data too short (minimum 32 bytes required)".to_string(),
            ));
        }

        // Verify the proof is not all zeros (malformed placeholder)
        if proof.iter().all(|&b| b == 0) {
            return Err(Error::Validation(
                "Proof verification failed: proof data is all zeros".to_string(),
            ));
        }

        // Verify that the message has valid chain IDs
        if message.source_chain == 0 || message.dest_chain == 0 {
            return Err(Error::Validation(
                "Proof verification failed: invalid chain ID (0)".to_string(),
            ));
        }

        // Verify source and destination are different chains
        if message.source_chain == message.dest_chain {
            return Err(Error::Validation(
                "Proof verification failed: source and destination chains are the same".to_string(),
            ));
        }

        info!(
            message_id = %message.id,
            source_chain = message.source_chain,
            proof_len = proof.len(),
            "Verifying message proof (placeholder - real verification pending)"
        );

        // PLACEHOLDER: Accept proofs that pass basic sanity checks above.
        // In production, this would:
        // 1. Decode Merkle proof structure
        // 2. Verify proof against trusted block header for source_chain
        // 3. Validate message hash matches the proven leaf
        // 4. Check that the block header is recent and from a trusted source
        Ok(true)
    }

    /// Update trusted headers
    pub async fn update_header(&self, chain_id: ChainId, header: B256) {
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
            B256::from(rand::random::<[u8; 32]>()),
            Address::ZERO,
            Address::ZERO,
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
            B256::from(rand::random::<[u8; 32]>()),
            Address::ZERO,
            Address::ZERO,
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
            B256::from(rand::random::<[u8; 32]>()),
            Address::ZERO,
            Address::ZERO,
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
            B256::from(rand::random::<[u8; 32]>()),
            Address::ZERO,
            Address::ZERO,
            Bytes::new(),
        );

        // Empty proof should be rejected
        let result = verifier.verify_proof(&message, &[]).await;
        assert!(result.is_err(), "Empty proof should be rejected");

        // Too-short proof should be rejected
        let short_proof = vec![1u8; 16];
        let result = verifier.verify_proof(&message, &short_proof).await;
        assert!(result.is_err(), "Short proof should be rejected");

        // All-zeros proof should be rejected
        let zero_proof = vec![0u8; 64];
        let result = verifier.verify_proof(&message, &zero_proof).await;
        assert!(result.is_err(), "All-zeros proof should be rejected");

        // Valid-looking proof (non-empty, >= 32 bytes, not all zeros) should pass placeholder check
        let valid_proof = vec![1u8; 64];
        let result = verifier.verify_proof(&message, &valid_proof).await.unwrap();
        assert!(result, "Valid-looking proof should pass placeholder verification");
    }

    #[test]
    fn test_message_with_value() {
        let message = CrossChainMessage::new(
            1,
            42161,
            B256::from(rand::random::<[u8; 32]>()),
            Address::ZERO,
            Address::ZERO,
            Bytes::new(),
        )
        .with_value(U256::from(1000u64));

        assert_eq!(message.value, U256::from(1000u64));
    }

    #[test]
    fn test_message_hash_deterministic() {
        let tx_hash = B256::from(rand::random::<[u8; 32]>());
        let mut msg1 = CrossChainMessage::new(
            1, 42161, tx_hash, Address::ZERO, Address::ZERO, Bytes::new(),
        );
        let mut msg2 = CrossChainMessage::new(
            1, 42161, tx_hash, Address::ZERO, Address::ZERO, Bytes::new(),
        );
        msg1.nonce = 42;
        msg2.nonce = 42;
        assert_eq!(msg1.message_hash(), msg2.message_hash());
    }

    #[test]
    fn test_message_hash_differs_by_nonce() {
        let tx_hash = B256::from(rand::random::<[u8; 32]>());
        let mut msg1 = CrossChainMessage::new(
            1, 42161, tx_hash, Address::ZERO, Address::ZERO, Bytes::new(),
        );
        let mut msg2 = CrossChainMessage::new(
            1, 42161, tx_hash, Address::ZERO, Address::ZERO, Bytes::new(),
        );
        msg1.nonce = 1;
        msg2.nonce = 2;
        assert_ne!(msg1.message_hash(), msg2.message_hash());
    }

    #[test]
    fn test_message_hash_differs_by_chain() {
        let tx_hash = B256::from(rand::random::<[u8; 32]>());
        let msg1 = CrossChainMessage::new(
            1, 42161, tx_hash, Address::ZERO, Address::ZERO, Bytes::new(),
        );
        let msg2 = CrossChainMessage::new(
            10, 42161, tx_hash, Address::ZERO, Address::ZERO, Bytes::new(),
        );
        assert_ne!(msg1.message_hash(), msg2.message_hash());
    }

    #[test]
    fn test_message_status_pending_not_final() {
        assert!(!MessageStatus::PendingConfirmation.is_final());
        assert!(!MessageStatus::ReadyToRelay.is_final());
    }

    #[test]
    fn test_message_status_expired_is_final() {
        assert!(MessageStatus::Expired.is_final());
    }

    #[test]
    fn test_relay_record_new() {
        let message = CrossChainMessage::new(
            1, 42161, B256::from(rand::random::<[u8; 32]>()), Address::ZERO, Address::ZERO, Bytes::new(),
        );
        let record = RelayRecord::new(message);
        assert_eq!(record.status, MessageStatus::PendingConfirmation);
        assert_eq!(record.confirmations, 0);
        assert_eq!(record.attempts, 0);
        assert!(record.dest_tx_hash.is_none());
        assert!(record.gas_used.is_none());
        assert!(record.errors.is_empty());
    }

    #[tokio::test]
    async fn test_retry_message() {
        let relayer = CrossChainRelayer::with_default_config();

        let message = CrossChainMessage::new(
            1, 42161, B256::from(rand::random::<[u8; 32]>()), Address::ZERO, Address::ZERO, Bytes::new(),
        );
        let id = relayer.submit_message(message).await.unwrap();

        // Retry should succeed (message is not final)
        let result = relayer.retry(&id).await;
        assert!(result.is_ok());

        // Check status changed to ReadyToRelay
        let record = relayer.get_status(&id).await.unwrap().unwrap();
        assert_eq!(record.status, MessageStatus::ReadyToRelay);
    }

    #[tokio::test]
    async fn test_retry_nonexistent_message() {
        let relayer = CrossChainRelayer::with_default_config();
        let result = relayer.retry("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_pending_wrong_chain() {
        let relayer = CrossChainRelayer::with_default_config();

        let message = CrossChainMessage::new(
            1, 42161, B256::from(rand::random::<[u8; 32]>()), Address::ZERO, Address::ZERO, Bytes::new(),
        );
        relayer.submit_message(message).await.unwrap();

        // Query a different destination chain
        let pending = relayer.get_pending(10).await.unwrap();
        assert_eq!(pending.len(), 0);
    }

    #[tokio::test]
    async fn test_get_status_nonexistent() {
        let relayer = CrossChainRelayer::with_default_config();
        let status = relayer.get_status("nonexistent").await.unwrap();
        assert!(status.is_none());
    }

    #[tokio::test]
    async fn test_process_pending_delivers_messages() {
        let relayer = CrossChainRelayer::with_default_config();

        let message = CrossChainMessage::new(
            1, 42161, B256::from(rand::random::<[u8; 32]>()), Address::ZERO, Address::ZERO, Bytes::new(),
        );
        let id = relayer.submit_message(message).await.unwrap();

        // First process: PendingConfirmation -> ReadyToRelay
        relayer.process_pending().await.unwrap();

        // Second process: ReadyToRelay -> Delivered
        let processed = relayer.process_pending().await.unwrap();
        assert_eq!(processed, 1);

        let record = relayer.get_status(&id).await.unwrap().unwrap();
        assert_eq!(record.status, MessageStatus::Delivered);
        assert!(record.dest_tx_hash.is_some());
        assert!(record.gas_used.is_some());
    }

    #[tokio::test]
    async fn test_submit_multiple_messages() {
        let relayer = CrossChainRelayer::with_default_config();

        for _ in 0..5 {
            let msg = CrossChainMessage::new(
                1, 42161, B256::from(rand::random::<[u8; 32]>()), Address::ZERO, Address::ZERO, Bytes::new(),
            );
            relayer.submit_message(msg).await.unwrap();
        }

        let pending = relayer.get_pending(42161).await.unwrap();
        assert_eq!(pending.len(), 5);
    }

    #[tokio::test]
    async fn test_proof_verifier_same_chain_rejected() {
        let verifier = ProofVerifier::new();
        let message = CrossChainMessage::new(
            1, 1, B256::from(rand::random::<[u8; 32]>()), Address::ZERO, Address::ZERO, Bytes::new(),
        );
        let proof = vec![1u8; 64];
        let result = verifier.verify_proof(&message, &proof).await;
        assert!(result.is_err(), "Same chain message proof should be rejected");
    }

    #[tokio::test]
    async fn test_proof_verifier_zero_chain_rejected() {
        let verifier = ProofVerifier::new();
        let message = CrossChainMessage::new(
            0, 42161, B256::from(rand::random::<[u8; 32]>()), Address::ZERO, Address::ZERO, Bytes::new(),
        );
        let proof = vec![1u8; 64];
        let result = verifier.verify_proof(&message, &proof).await;
        assert!(result.is_err(), "Zero chain ID should be rejected");
    }

    #[tokio::test]
    async fn test_proof_verifier_update_header() {
        let verifier = ProofVerifier::new();
        let header = B256::from(rand::random::<[u8; 32]>());
        verifier.update_header(1, header).await;

        // Verify header was stored (indirectly, by checking no panic)
        let header2 = B256::from(rand::random::<[u8; 32]>());
        verifier.update_header(1, header2).await;
    }

    #[test]
    fn test_relayer_config_all_chains() {
        let config = RelayerConfig::default();
        assert!(config.rpc_endpoints.contains_key(&1));     // Ethereum
        assert!(config.rpc_endpoints.contains_key(&42161)); // Arbitrum
        assert!(config.rpc_endpoints.contains_key(&8453));  // Base
        assert!(config.rpc_endpoints.contains_key(&10));    // Optimism
        assert!(config.rpc_endpoints.contains_key(&137));   // Polygon
        assert_eq!(config.max_gas_price_gwei, 100);
        assert_eq!(config.retry_delay_secs, 30);
        assert_eq!(config.max_retries, 5);
    }

    #[test]
    fn test_proof_verifier_default() {
        let verifier = ProofVerifier::default();
        // Should be same as ProofVerifier::new()
        drop(verifier);
    }
}
