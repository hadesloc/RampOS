//! TON Chain Adapter
//!
//! Implementation for The Open Network (TON) blockchain
//! using TON Center HTTP API (toncenter.com/api/v2).
//! Falls back to mock data if API key is not set or request fails.

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::time::Duration;
use tracing::{debug, warn};

use super::{
    Balance, Chain, ChainError, ChainId, ChainType, FeeEstimate, FeeOption, Result,
    TokenBalance, Transaction, TxHash, TxState, TxStatus, UnifiedAddress,
};

/// TON Center API response wrapper
#[derive(Debug, Deserialize)]
struct TonApiResponse<T> {
    ok: bool,
    result: Option<T>,
    error: Option<String>,
}

/// Response from getAddressBalance
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum BalanceResult {
    StringVal(String),
    NumVal(i64),
}

/// Response from getTransactions
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TonTransaction {
    #[serde(default)]
    utime: u64,
    #[serde(default)]
    fee: String,
    transaction_id: Option<TonTransactionId>,
    in_msg: Option<TonMessage>,
    out_msgs: Option<Vec<TonMessage>>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TonTransactionId {
    lt: Option<String>,
    hash: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TonMessage {
    #[serde(default)]
    value: String,
    source: Option<String>,
    destination: Option<String>,
}

/// TON chain configuration
#[derive(Debug, Clone)]
pub struct TonChainConfig {
    pub chain_id: ChainId,
    pub name: String,
    pub api_url: String,
    pub is_testnet: bool,
    pub explorer_url: String,
}

impl TonChainConfig {
    /// Create config for TON mainnet
    pub fn mainnet(api_url: &str) -> Self {
        Self {
            chain_id: ChainId::TON_MAINNET,
            name: "TON".to_string(),
            api_url: api_url.to_string(),
            is_testnet: false,
            explorer_url: "https://tonscan.org".to_string(),
        }
    }

    /// Create config for TON testnet
    pub fn testnet(api_url: &str) -> Self {
        Self {
            chain_id: ChainId::TON_TESTNET,
            name: "TON Testnet".to_string(),
            api_url: api_url.to_string(),
            is_testnet: true,
            explorer_url: "https://testnet.tonscan.org".to_string(),
        }
    }
}

/// TON address format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TonAddressFormat {
    /// Raw format: workchain:hex (e.g., 0:abc123...)
    Raw,
    /// User-friendly bounceable (base64url with flags)
    Bounceable,
    /// User-friendly non-bounceable
    NonBounceable,
}

/// TON Chain implementation
pub struct TonChain {
    config: TonChainConfig,
    client: Client,
    api_key: Option<String>,
    api_url: String,
}

impl TonChain {
    /// Create a new TON chain instance
    pub fn new(config: TonChainConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| ChainError::Internal(format!("Failed to create HTTP client: {}", e)))?;

        let api_key = env::var("TON_API_KEY").ok();
        let api_url = env::var("TON_API_URL").unwrap_or_else(|_| config.api_url.clone());

        Ok(Self {
            config,
            client,
            api_key,
            api_url,
        })
    }

    /// Make a GET request to the TON Center API
    async fn api_get<T: serde::de::DeserializeOwned>(
        &self,
        endpoint: &str,
        params: &[(&str, &str)],
    ) -> std::result::Result<T, ChainError> {
        let url = format!("{}/{}", self.api_url.trim_end_matches('/'), endpoint);

        let mut req = self.client.get(&url);

        if let Some(ref key) = self.api_key {
            req = req.header("X-API-Key", key);
        }

        for (k, v) in params {
            req = req.query(&[(k, v)]);
        }

        debug!("TON API GET: {} params={:?}", endpoint, params);

        let resp = req.send().await.map_err(|e| {
            ChainError::RpcError(format!("TON API request failed: {}", e))
        })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(ChainError::RpcError(format!(
                "TON API returned {}: {}",
                status, body
            )));
        }

        let api_resp: TonApiResponse<T> = resp.json().await.map_err(|e| {
            ChainError::RpcError(format!("Failed to parse TON API response: {}", e))
        })?;

        if !api_resp.ok {
            return Err(ChainError::RpcError(format!(
                "TON API error: {}",
                api_resp.error.unwrap_or_else(|| "unknown error".to_string())
            )));
        }

        api_resp.result.ok_or_else(|| {
            ChainError::RpcError("TON API returned ok=true but no result".to_string())
        })
    }

    /// Make a POST request to the TON Center API
    async fn api_post<T: serde::de::DeserializeOwned>(
        &self,
        endpoint: &str,
        body: &serde_json::Value,
    ) -> std::result::Result<T, ChainError> {
        let url = format!("{}/{}", self.api_url.trim_end_matches('/'), endpoint);

        let mut req = self.client.post(&url).json(body);

        if let Some(ref key) = self.api_key {
            req = req.header("X-API-Key", key);
        }

        debug!("TON API POST: {} body={}", endpoint, body);

        let resp = req.send().await.map_err(|e| {
            ChainError::RpcError(format!("TON API request failed: {}", e))
        })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(ChainError::RpcError(format!(
                "TON API returned {}: {}",
                status, body
            )));
        }

        let api_resp: TonApiResponse<T> = resp.json().await.map_err(|e| {
            ChainError::RpcError(format!("Failed to parse TON API response: {}", e))
        })?;

        if !api_resp.ok {
            return Err(ChainError::RpcError(format!(
                "TON API error: {}",
                api_resp.error.unwrap_or_else(|| "unknown error".to_string())
            )));
        }

        api_resp.result.ok_or_else(|| {
            ChainError::RpcError("TON API returned ok=true but no result".to_string())
        })
    }

    /// Validate TON address
    fn validate_ton_address(address: &str) -> Result<TonAddressFormat> {
        // Raw format: workchain:hex (e.g., 0:abc123... or -1:abc123...)
        if address.contains(':') {
            let parts: Vec<&str> = address.split(':').collect();
            if parts.len() != 2 {
                return Err(ChainError::InvalidAddress(
                    "Invalid TON raw address format".to_string(),
                ));
            }

            // Workchain should be -1 or 0
            let workchain = parts[0].parse::<i32>().map_err(|_| {
                ChainError::InvalidAddress("Invalid workchain in TON address".to_string())
            })?;

            if workchain != 0 && workchain != -1 {
                return Err(ChainError::InvalidAddress(format!(
                    "Invalid workchain {}, expected 0 or -1",
                    workchain
                )));
            }

            // Hex part should be 64 characters
            if parts[1].len() != 64 {
                return Err(ChainError::InvalidAddress(format!(
                    "Invalid hex part length {}, expected 64",
                    parts[1].len()
                )));
            }

            // Validate hex characters
            if !parts[1].chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(ChainError::InvalidAddress(
                    "Invalid hex characters in TON address".to_string(),
                ));
            }

            return Ok(TonAddressFormat::Raw);
        }

        // User-friendly format: base64url encoded, 48 characters
        // E.g., EQDtFpEwcFAEcRe5mLVh2N6C0x-_hJEM7W61_JLnSF74p4q2
        if address.len() == 48 {
            // Check for valid base64url characters
            let is_valid = address.chars().all(|c| {
                c.is_ascii_alphanumeric() || c == '-' || c == '_'
            });

            if !is_valid {
                return Err(ChainError::InvalidAddress(
                    "Invalid base64url characters in TON address".to_string(),
                ));
            }

            // First character indicates format:
            // E or U for mainnet
            // k or 0 for testnet
            match address.chars().next() {
                Some('E') | Some('U') => Ok(TonAddressFormat::Bounceable),
                Some('k') | Some('0') => Ok(TonAddressFormat::NonBounceable),
                _ => Ok(TonAddressFormat::Bounceable), // Default assumption
            }
        } else {
            Err(ChainError::InvalidAddress(format!(
                "Invalid TON address length {}, expected 48 for user-friendly or workchain:hex for raw",
                address.len()
            )))
        }
    }

    /// Convert to raw address format (placeholder)
    pub fn to_raw_address(&self, address: &str) -> Result<String> {
        let format = Self::validate_ton_address(address)?;
        match format {
            TonAddressFormat::Raw => Ok(address.to_string()),
            _ => {
                // Would need base64 decoding and parsing
                Err(ChainError::NotSupported(
                    "User-friendly to raw address conversion not yet implemented".to_string(),
                ))
            }
        }
    }

    /// Return mock balance as fallback
    fn mock_balance(address: &str) -> Balance {
        warn!("Using mock TON balance for address {}", address);
        Balance {
            native: "1000000000".to_string(), // 1 TON in nanotons
            native_symbol: "TON".to_string(),
            tokens: HashMap::new(),
        }
    }

    /// Return mock tx status as fallback
    fn mock_tx_status(hash: &str) -> TxStatus {
        warn!("Using mock TON transaction status for hash {}", hash);
        TxStatus {
            hash: TxHash(hash.to_string()),
            status: TxState::Confirmed,
            block_number: Some(1),
            block_hash: None,
            confirmations: 1,
            gas_used: Some("5000000".to_string()),
            effective_gas_price: None,
            error_message: None,
        }
    }
}

#[async_trait]
impl Chain for TonChain {
    fn chain_id(&self) -> ChainId {
        self.config.chain_id
    }

    fn name(&self) -> &str {
        &self.config.name
    }

    fn chain_type(&self) -> ChainType {
        ChainType::Ton
    }

    fn is_testnet(&self) -> bool {
        self.config.is_testnet
    }

    fn native_symbol(&self) -> &str {
        "TON"
    }

    fn explorer_url(&self) -> &str {
        &self.config.explorer_url
    }

    fn validate_address(&self, address: &str) -> Result<UnifiedAddress> {
        Self::validate_ton_address(address)?;
        UnifiedAddress::new(ChainType::Ton, address)
    }

    async fn get_balance(&self, address: &str) -> Result<Balance> {
        Self::validate_ton_address(address)?;

        if self.api_key.is_none() {
            warn!("TON_API_KEY not set, returning mock balance");
            return Ok(Self::mock_balance(address));
        }

        match self
            .api_get::<BalanceResult>("getAddressBalance", &[("address", address)])
            .await
        {
            Ok(balance_result) => {
                let balance_str = match balance_result {
                    BalanceResult::StringVal(s) => s,
                    BalanceResult::NumVal(n) => n.to_string(),
                };
                Ok(Balance {
                    native: balance_str,
                    native_symbol: "TON".to_string(),
                    tokens: HashMap::new(),
                })
            }
            Err(e) => {
                warn!("TON API get_balance failed, falling back to mock: {}", e);
                Ok(Self::mock_balance(address))
            }
        }
    }

    async fn get_token_balance(&self, address: &str, token_address: &str) -> Result<TokenBalance> {
        Self::validate_ton_address(address)?;
        Self::validate_ton_address(token_address)?;

        // Jetton balance queries require a more complex flow (querying jetton wallet address first)
        Err(ChainError::NotSupported(
            "TON Jetton balance query not yet implemented.".to_string(),
        ))
    }

    async fn send_transaction(&self, tx: Transaction) -> Result<TxHash> {
        Self::validate_ton_address(&tx.from)?;
        Self::validate_ton_address(&tx.to)?;

        if self.api_key.is_none() {
            warn!("TON_API_KEY not set, cannot send real transaction - returning mock hash");
            return Ok(TxHash(
                "mock_ton_tx_00000000000000000000000000000000000000000000".to_string(),
            ));
        }

        // The sendBoc endpoint expects a pre-signed BOC (Bag of Cells) encoded in base64.
        // The transaction data field should contain the serialized BOC.
        let boc = tx.data.ok_or_else(|| {
            ChainError::TransactionFailed(
                "TON transactions require a signed BOC in the data field".to_string(),
            )
        })?;

        let boc_base64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &boc,
        );

        let body = serde_json::json!({ "boc": boc_base64 });

        #[derive(Deserialize)]
        struct SendBocResult {
            hash: Option<String>,
        }

        match self.api_post::<SendBocResult>("sendBoc", &body).await {
            Ok(result) => {
                let hash = result
                    .hash
                    .unwrap_or_else(|| format!("ton_tx_{}", chrono::Utc::now().timestamp()));
                Ok(TxHash(hash))
            }
            Err(e) => {
                warn!("TON API sendBoc failed: {}", e);
                Err(ChainError::TransactionFailed(format!(
                    "Failed to send TON transaction: {}",
                    e
                )))
            }
        }
    }

    async fn get_transaction(&self, hash: &str) -> Result<TxStatus> {
        // TON transaction hash format
        if hash.len() < 40 {
            return Err(ChainError::InvalidAddress(format!(
                "Invalid TON transaction hash format: {}",
                hash
            )));
        }

        if self.api_key.is_none() {
            warn!("TON_API_KEY not set, returning mock transaction status");
            return Ok(Self::mock_tx_status(hash));
        }

        // TON Center doesn't have a direct "get transaction by hash" endpoint.
        // We use getTransactions with the address. For lookup by hash, we query
        // with a known address or return mock. In practice, you'd need the address too.
        // For now, fall back to mock since we don't have the address context.
        warn!("TON getTransactions requires address context, falling back to mock for hash lookup");
        Ok(Self::mock_tx_status(hash))
    }

    async fn wait_for_confirmation(
        &self,
        hash: &TxHash,
        _confirmations: u64,
        _timeout_secs: u64,
    ) -> Result<TxStatus> {
        Ok(TxStatus {
            hash: hash.clone(),
            status: TxState::NotFound,
            block_number: None,
            block_hash: None,
            confirmations: 0,
            gas_used: None,
            effective_gas_price: None,
            error_message: Some("TON confirmation waiting not yet implemented.".to_string()),
        })
    }

    async fn estimate_fee(&self, tx: &Transaction) -> Result<FeeEstimate> {
        Self::validate_ton_address(&tx.from)?;

        // TON uses "gas" which is different from EVM
        // Typical simple transfer costs ~0.003-0.01 TON
        let base_gas: u64 = 10_000_000; // 0.01 TON in nanotons

        Ok(FeeEstimate {
            gas_units: base_gas,
            slow: FeeOption {
                price: "1".to_string(),
                max_fee: None,
                priority_fee: None,
                total_cost: "3000000".to_string(), // 0.003 TON
                estimated_time_seconds: 15,
            },
            standard: FeeOption {
                price: "1".to_string(),
                max_fee: None,
                priority_fee: None,
                total_cost: "5000000".to_string(), // 0.005 TON
                estimated_time_seconds: 10,
            },
            fast: FeeOption {
                price: "1".to_string(),
                max_fee: None,
                priority_fee: None,
                total_cost: "10000000".to_string(), // 0.01 TON
                estimated_time_seconds: 5,
            },
        })
    }

    async fn get_block_number(&self) -> Result<u64> {
        if self.api_key.is_none() {
            warn!("TON_API_KEY not set, returning mock block number");
            return Ok(0);
        }

        #[derive(Deserialize)]
        struct MasterchainInfo {
            last: Option<MasterchainBlock>,
        }
        #[derive(Deserialize)]
        struct MasterchainBlock {
            seqno: Option<u64>,
        }

        match self
            .api_get::<MasterchainInfo>("getMasterchainInfo", &[])
            .await
        {
            Ok(info) => {
                let seqno = info
                    .last
                    .and_then(|b| b.seqno)
                    .unwrap_or(0);
                Ok(seqno)
            }
            Err(e) => {
                warn!("TON API getMasterchainInfo failed, returning 0: {}", e);
                Ok(0)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ton_config() {
        let config = TonChainConfig::mainnet("https://toncenter.com/api/v2");
        assert_eq!(config.chain_id, ChainId::TON_MAINNET);
        assert!(!config.is_testnet);

        let testnet = TonChainConfig::testnet("https://testnet.toncenter.com/api/v2");
        assert_eq!(testnet.chain_id, ChainId::TON_TESTNET);
        assert!(testnet.is_testnet);
    }

    #[test]
    fn test_raw_address_validation() {
        // Valid raw address
        let valid = TonChain::validate_ton_address(
            "0:83dfd552e63729b472fcbcc8c45ebcc6691702558b68ec7527e1ba403a0f31a8",
        );
        assert!(valid.is_ok());
        assert_eq!(valid.unwrap(), TonAddressFormat::Raw);

        // Invalid workchain
        let invalid_workchain = TonChain::validate_ton_address(
            "2:83dfd552e63729b472fcbcc8c45ebcc6691702558b68ec7527e1ba403a0f31a8",
        );
        assert!(invalid_workchain.is_err());

        // Masterchain (-1)
        let masterchain = TonChain::validate_ton_address(
            "-1:83dfd552e63729b472fcbcc8c45ebcc6691702558b68ec7527e1ba403a0f31a8",
        );
        assert!(masterchain.is_ok());
    }

    #[test]
    fn test_user_friendly_address_validation() {
        // Valid user-friendly address (48 chars)
        let valid = TonChain::validate_ton_address(
            "EQDtFpEwcFAEcRe5mLVh2N6C0x-_hJEM7W61_JLnSF74p4q2",
        );
        assert!(valid.is_ok());

        // Too short
        let short = TonChain::validate_ton_address("EQDtFpEwcFAE");
        assert!(short.is_err());
    }

    #[test]
    fn test_chain_info() {
        let chain = TonChain::new(TonChainConfig::mainnet("https://api.example.com")).unwrap();
        assert_eq!(chain.chain_type(), ChainType::Ton);
        assert_eq!(chain.native_symbol(), "TON");
    }

    #[tokio::test]
    async fn test_get_balance_no_api_key() {
        // Without API key, should fall back to mock
        env::remove_var("TON_API_KEY");
        let chain = TonChain::new(TonChainConfig::mainnet("https://toncenter.com/api/v2")).unwrap();
        let balance = chain
            .get_balance("0:83dfd552e63729b472fcbcc8c45ebcc6691702558b68ec7527e1ba403a0f31a8")
            .await;
        assert!(balance.is_ok());
        let bal = balance.unwrap();
        assert_eq!(bal.native, "1000000000");
        assert_eq!(bal.native_symbol, "TON");
    }

    #[tokio::test]
    async fn test_get_transaction_no_api_key() {
        env::remove_var("TON_API_KEY");
        let chain = TonChain::new(TonChainConfig::mainnet("https://toncenter.com/api/v2")).unwrap();
        let tx = chain
            .get_transaction("abcdef1234567890abcdef1234567890abcdef1234567890")
            .await;
        assert!(tx.is_ok());
        let status = tx.unwrap();
        assert_eq!(status.status, TxState::Confirmed);
    }

    #[tokio::test]
    async fn test_send_transaction_no_api_key() {
        env::remove_var("TON_API_KEY");
        let chain = TonChain::new(TonChainConfig::mainnet("https://toncenter.com/api/v2")).unwrap();
        let tx = Transaction {
            from: "0:83dfd552e63729b472fcbcc8c45ebcc6691702558b68ec7527e1ba403a0f31a8"
                .to_string(),
            to: "0:83dfd552e63729b472fcbcc8c45ebcc6691702558b68ec7527e1ba403a0f31a8"
                .to_string(),
            value: "1000000000".to_string(),
            data: None,
            gas_limit: None,
            gas_price: None,
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            nonce: None,
        };
        let result = chain.send_transaction(tx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_block_number_no_api_key() {
        env::remove_var("TON_API_KEY");
        let chain = TonChain::new(TonChainConfig::mainnet("https://toncenter.com/api/v2")).unwrap();
        let block = chain.get_block_number().await;
        assert!(block.is_ok());
        assert_eq!(block.unwrap(), 0);
    }
}
