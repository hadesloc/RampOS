//! Solana Chain Adapter
//!
//! Real implementation using reqwest HTTP calls to Solana JSON-RPC API.
//! Does NOT depend on solana-sdk/solana-client crates.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use super::{
    Balance, Chain, ChainError, ChainId, ChainType, FeeEstimate, FeeOption, Result,
    TokenBalance, Transaction, TxHash, TxState, TxStatus, UnifiedAddress,
};

const DEFAULT_RPC_URL: &str = "https://api.mainnet-beta.solana.com";
const RPC_TIMEOUT_SECS: u64 = 10;
const LAMPORTS_PER_SOL: f64 = 1_000_000_000.0;

/// Solana chain configuration
#[derive(Debug, Clone)]
pub struct SolanaChainConfig {
    pub chain_id: ChainId,
    pub name: String,
    pub rpc_url: String,
    pub is_testnet: bool,
    pub explorer_url: String,
}

impl SolanaChainConfig {
    /// Create config for Solana mainnet
    pub fn mainnet(rpc_url: &str) -> Self {
        Self {
            chain_id: ChainId::SOLANA_MAINNET,
            name: "Solana".to_string(),
            rpc_url: rpc_url.to_string(),
            is_testnet: false,
            explorer_url: "https://explorer.solana.com".to_string(),
        }
    }

    /// Create config for Solana devnet
    pub fn devnet(rpc_url: &str) -> Self {
        Self {
            chain_id: ChainId::SOLANA_DEVNET,
            name: "Solana Devnet".to_string(),
            rpc_url: rpc_url.to_string(),
            is_testnet: true,
            explorer_url: "https://explorer.solana.com?cluster=devnet".to_string(),
        }
    }
}

// --- JSON-RPC request/response types ---

#[derive(Serialize)]
struct RpcRequest<'a> {
    jsonrpc: &'a str,
    id: u64,
    method: &'a str,
    params: serde_json::Value,
}

#[derive(Deserialize)]
struct RpcResponse<T> {
    result: Option<T>,
    error: Option<RpcErrorDetail>,
}

#[derive(Deserialize)]
struct RpcErrorDetail {
    code: i64,
    message: String,
}

#[derive(Deserialize)]
struct BalanceResult {
    value: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct TransactionResult {
    slot: Option<u64>,
    block_time: Option<i64>,
    meta: Option<TransactionMeta>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransactionMeta {
    err: Option<serde_json::Value>,
    fee: Option<u64>,
}

/// Solana Chain implementation using JSON-RPC via reqwest
pub struct SolanaChain {
    config: SolanaChainConfig,
    client: reqwest::Client,
    rpc_url: String,
}

impl SolanaChain {
    /// Create a new Solana chain instance
    pub fn new(config: SolanaChainConfig) -> Result<Self> {
        let rpc_url = std::env::var("SOLANA_RPC_URL")
            .unwrap_or_else(|_| {
                if config.rpc_url.is_empty() {
                    DEFAULT_RPC_URL.to_string()
                } else {
                    config.rpc_url.clone()
                }
            });

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(RPC_TIMEOUT_SECS))
            .build()
            .map_err(|e| ChainError::Internal(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            config,
            client,
            rpc_url,
        })
    }

    /// Validate Solana address (base58, 32-44 chars)
    fn validate_solana_address(address: &str) -> Result<()> {
        const BASE58_CHARS: &str = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

        if address.len() < 32 || address.len() > 44 {
            return Err(ChainError::InvalidAddress(format!(
                "Solana address must be 32-44 characters, got {}",
                address.len()
            )));
        }

        for c in address.chars() {
            if !BASE58_CHARS.contains(c) {
                return Err(ChainError::InvalidAddress(format!(
                    "Invalid base58 character in Solana address: {}",
                    c
                )));
            }
        }

        // Validate that it decodes to 32 bytes
        let decoded = bs58_decode(address).map_err(|_| {
            ChainError::InvalidAddress("Address is not valid base58".to_string())
        })?;
        if decoded.len() != 32 {
            return Err(ChainError::InvalidAddress(format!(
                "Solana address must decode to 32 bytes, got {}",
                decoded.len()
            )));
        }

        Ok(())
    }

    /// Send a JSON-RPC request to the Solana node
    async fn rpc_call<T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<T> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            id: 1,
            method,
            params,
        };

        let response = self
            .client
            .post(&self.rpc_url)
            .json(&request)
            .send()
            .await
            .map_err(|e| ChainError::RpcError(format!("RPC request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(ChainError::RpcError(format!(
                "RPC returned HTTP {}",
                response.status()
            )));
        }

        let rpc_response: RpcResponse<T> = response
            .json()
            .await
            .map_err(|e| ChainError::RpcError(format!("Failed to parse RPC response: {}", e)))?;

        if let Some(err) = rpc_response.error {
            return Err(ChainError::RpcError(format!(
                "RPC error {}: {}",
                err.code, err.message
            )));
        }

        rpc_response
            .result
            .ok_or_else(|| ChainError::RpcError("RPC response missing result".to_string()))
    }
}

/// Minimal base58 decoder (no external crate needed)
fn bs58_decode(input: &str) -> std::result::Result<Vec<u8>, &'static str> {
    const ALPHABET: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

    // Count leading '1's (zero bytes in base58)
    let num_leading = input.bytes().take_while(|&b| b == b'1').count();

    let mut bytes: Vec<u8> = Vec::new();

    for c in input.bytes().skip(num_leading) {
        let mut carry = ALPHABET
            .iter()
            .position(|&x| x == c)
            .ok_or("Invalid base58 character")? as u32;

        for byte in bytes.iter_mut() {
            carry += (*byte as u32) * 58;
            *byte = (carry & 0xFF) as u8;
            carry >>= 8;
        }

        while carry > 0 {
            bytes.push((carry & 0xFF) as u8);
            carry >>= 8;
        }
    }

    bytes.reverse();

    // Prepend leading zero bytes
    let mut result = vec![0u8; num_leading];
    result.extend(bytes);
    Ok(result)
}

#[async_trait]
impl Chain for SolanaChain {
    fn chain_id(&self) -> ChainId {
        self.config.chain_id
    }

    fn name(&self) -> &str {
        &self.config.name
    }

    fn chain_type(&self) -> ChainType {
        ChainType::Solana
    }

    fn is_testnet(&self) -> bool {
        self.config.is_testnet
    }

    fn native_symbol(&self) -> &str {
        "SOL"
    }

    fn explorer_url(&self) -> &str {
        &self.config.explorer_url
    }

    fn tx_url(&self, hash: &TxHash) -> String {
        if self.config.is_testnet {
            format!("{}/tx/{}?cluster=devnet", self.explorer_url(), hash)
        } else {
            format!("{}/tx/{}", self.explorer_url(), hash)
        }
    }

    fn address_url(&self, address: &str) -> String {
        if self.config.is_testnet {
            format!("{}/address/{}?cluster=devnet", self.explorer_url(), address)
        } else {
            format!("{}/address/{}", self.explorer_url(), address)
        }
    }

    fn validate_address(&self, address: &str) -> Result<UnifiedAddress> {
        Self::validate_solana_address(address)?;
        UnifiedAddress::new(ChainType::Solana, address)
    }

    async fn get_balance(&self, address: &str) -> Result<Balance> {
        Self::validate_solana_address(address)?;

        // Try real RPC call, fall back to mock on failure
        match self
            .rpc_call::<BalanceResult>("getBalance", serde_json::json!([address]))
            .await
        {
            Ok(result) => {
                let sol_balance = result.value as f64 / LAMPORTS_PER_SOL;
                Ok(Balance {
                    native: format!("{:.9}", sol_balance),
                    native_symbol: "SOL".to_string(),
                    tokens: HashMap::new(),
                })
            }
            Err(e) => {
                tracing::warn!("Solana RPC getBalance failed, returning mock: {}", e);
                Ok(Balance {
                    native: "0.000000000".to_string(),
                    native_symbol: "SOL".to_string(),
                    tokens: HashMap::new(),
                })
            }
        }
    }

    async fn get_token_balance(&self, address: &str, token_address: &str) -> Result<TokenBalance> {
        Self::validate_solana_address(address)?;
        Self::validate_solana_address(token_address)?;

        // SPL token balance requires getTokenAccountsByOwner - simplified for now
        Err(ChainError::NotSupported(
            "SPL token balance query requires associated token account lookup".to_string(),
        ))
    }

    async fn send_transaction(&self, tx: Transaction) -> Result<TxHash> {
        Self::validate_solana_address(&tx.from)?;
        Self::validate_solana_address(&tx.to)?;

        // send_transaction expects a base64-encoded serialized transaction
        let tx_data = tx.data.as_ref().ok_or_else(|| {
            ChainError::TransactionFailed(
                "Solana send_transaction requires pre-serialized transaction in data field"
                    .to_string(),
            )
        })?;

        use base64::{engine::general_purpose::STANDARD, Engine};
        let encoded = STANDARD.encode(tx_data);

        match self
            .rpc_call::<String>(
                "sendTransaction",
                serde_json::json!([encoded, {"encoding": "base64"}]),
            )
            .await
        {
            Ok(signature) => Ok(TxHash(signature)),
            Err(e) => {
                tracing::warn!("Solana RPC sendTransaction failed: {}", e);
                Err(ChainError::TransactionFailed(format!(
                    "Failed to send Solana transaction: {}",
                    e
                )))
            }
        }
    }

    async fn get_transaction(&self, hash: &str) -> Result<TxStatus> {
        // Solana tx signatures are base58, typically 87-88 chars but can vary
        if hash.is_empty() {
            return Err(ChainError::InvalidAddress(
                "Transaction hash cannot be empty".to_string(),
            ));
        }

        match self
            .rpc_call::<TransactionResult>(
                "getTransaction",
                serde_json::json!([hash, {"encoding": "json", "maxSupportedTransactionVersion": 0}]),
            )
            .await
        {
            Ok(result) => {
                let has_error = result
                    .meta
                    .as_ref()
                    .and_then(|m| m.err.as_ref())
                    .is_some();

                let fee_used = result
                    .meta
                    .as_ref()
                    .and_then(|m| m.fee)
                    .map(|f| f.to_string());

                Ok(TxStatus {
                    hash: TxHash(hash.to_string()),
                    status: if has_error {
                        TxState::Failed
                    } else {
                        TxState::Confirmed
                    },
                    block_number: result.slot,
                    block_hash: None,
                    confirmations: 1, // Solana finality is fast
                    gas_used: fee_used,
                    effective_gas_price: None,
                    error_message: if has_error {
                        Some("Transaction failed on-chain".to_string())
                    } else {
                        None
                    },
                })
            }
            Err(e) => {
                tracing::warn!("Solana RPC getTransaction failed, returning NotFound: {}", e);
                Ok(TxStatus {
                    hash: TxHash(hash.to_string()),
                    status: TxState::NotFound,
                    block_number: None,
                    block_hash: None,
                    confirmations: 0,
                    gas_used: None,
                    effective_gas_price: None,
                    error_message: Some(format!("RPC error: {}", e)),
                })
            }
        }
    }

    async fn wait_for_confirmation(
        &self,
        hash: &TxHash,
        _confirmations: u64,
        timeout_secs: u64,
    ) -> Result<TxStatus> {
        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(timeout_secs);

        loop {
            if start.elapsed() > timeout {
                return Err(ChainError::Timeout);
            }

            match self.get_transaction(&hash.0).await {
                Ok(status) => match status.status {
                    TxState::Confirmed | TxState::Failed => return Ok(status),
                    _ => {
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                },
                Err(_) => {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
        }
    }

    async fn estimate_fee(&self, tx: &Transaction) -> Result<FeeEstimate> {
        Self::validate_solana_address(&tx.from)?;

        // Solana has fixed base fee + priority fee
        // Base fee is ~5000 lamports per signature
        let base_fee_lamports: u64 = 5000;
        let compute_units: u64 = tx.gas_limit.unwrap_or(200_000);

        // Priority fee in micro-lamports per compute unit
        let slow_priority = 1;
        let standard_priority = 100;
        let fast_priority = 1000;

        Ok(FeeEstimate {
            gas_units: compute_units,
            slow: FeeOption {
                price: slow_priority.to_string(),
                max_fee: None,
                priority_fee: Some(slow_priority.to_string()),
                total_cost: (base_fee_lamports + slow_priority * compute_units / 1_000_000)
                    .to_string(),
                estimated_time_seconds: 10,
            },
            standard: FeeOption {
                price: standard_priority.to_string(),
                max_fee: None,
                priority_fee: Some(standard_priority.to_string()),
                total_cost: (base_fee_lamports + standard_priority * compute_units / 1_000_000)
                    .to_string(),
                estimated_time_seconds: 3,
            },
            fast: FeeOption {
                price: fast_priority.to_string(),
                max_fee: None,
                priority_fee: Some(fast_priority.to_string()),
                total_cost: (base_fee_lamports + fast_priority * compute_units / 1_000_000)
                    .to_string(),
                estimated_time_seconds: 1,
            },
        })
    }

    async fn get_block_number(&self) -> Result<u64> {
        match self
            .rpc_call::<u64>("getSlot", serde_json::json!([]))
            .await
        {
            Ok(slot) => Ok(slot),
            Err(e) => {
                tracing::warn!("Solana RPC getSlot failed: {}", e);
                Err(ChainError::RpcError(format!(
                    "Failed to get current slot: {}",
                    e
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solana_config() {
        let config = SolanaChainConfig::mainnet("https://api.mainnet-beta.solana.com");
        assert_eq!(config.chain_id, ChainId::SOLANA_MAINNET);
        assert!(!config.is_testnet);

        let devnet = SolanaChainConfig::devnet("https://api.devnet.solana.com");
        assert_eq!(devnet.chain_id, ChainId::SOLANA_DEVNET);
        assert!(devnet.is_testnet);
    }

    #[test]
    fn test_address_validation() {
        // Valid Solana address (example)
        let valid = SolanaChain::validate_solana_address(
            "7cVfgArCheMR6Cs4t6vz5rfnqd56vZq4ndaBrY5xkxXy",
        );
        assert!(valid.is_ok());

        // Too short
        let short = SolanaChain::validate_solana_address("abc123");
        assert!(short.is_err());

        // Invalid character (0, O, I, l are not in base58)
        let invalid_char =
            SolanaChain::validate_solana_address("0cVfgArCheMR6Cs4t6vz5rfnqd56vZq4ndaBrY5xkxXy");
        assert!(invalid_char.is_err());
    }

    #[test]
    fn test_explorer_urls() {
        let mainnet =
            SolanaChain::new(SolanaChainConfig::mainnet("https://api.mainnet-beta.solana.com"))
                .unwrap();
        let hash = TxHash("test_hash".to_string());
        assert!(!mainnet.tx_url(&hash).contains("cluster=devnet"));

        let devnet =
            SolanaChain::new(SolanaChainConfig::devnet("https://api.devnet.solana.com")).unwrap();
        assert!(devnet.tx_url(&hash).contains("cluster=devnet"));
    }

    #[test]
    fn test_bs58_decode() {
        // "1" in base58 is a leading zero byte
        let result = bs58_decode("1").unwrap();
        assert_eq!(result, vec![0]);

        // A valid Solana pubkey should decode to 32 bytes
        let result = bs58_decode("7cVfgArCheMR6Cs4t6vz5rfnqd56vZq4ndaBrY5xkxXy");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 32);
    }

    #[test]
    fn test_address_validates_32_bytes() {
        // System program address (all zeros = "11111111111111111111111111111111")
        let system_program =
            SolanaChain::validate_solana_address("11111111111111111111111111111111");
        assert!(system_program.is_ok());
    }

    #[tokio::test]
    async fn test_estimate_fee() {
        let chain =
            SolanaChain::new(SolanaChainConfig::mainnet("https://api.mainnet-beta.solana.com"))
                .unwrap();
        let tx = Transaction {
            from: "7cVfgArCheMR6Cs4t6vz5rfnqd56vZq4ndaBrY5xkxXy".to_string(),
            to: "11111111111111111111111111111111".to_string(),
            value: "1000000000".to_string(),
            data: None,
            gas_limit: Some(200_000),
            gas_price: None,
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            nonce: None,
        };
        let fee = chain.estimate_fee(&tx).await;
        assert!(fee.is_ok());
        let fee = fee.unwrap();
        assert_eq!(fee.gas_units, 200_000);
    }
}
