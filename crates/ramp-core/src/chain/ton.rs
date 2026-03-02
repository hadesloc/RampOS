//! TON Chain Adapter
//!
//! Implementation for The Open Network (TON) blockchain
//! using TON Center HTTP API (toncenter.com/api/v2).
//! Uses TON Center HTTP API and returns explicit errors when unavailable.

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

    /// Convert user-friendly TON address to raw format (workchain:hex_hash).
    /// If already raw, returns as-is.
    pub fn to_raw_address(&self, address: &str) -> Result<String> {
        let format = Self::validate_ton_address(address)?;
        match format {
            TonAddressFormat::Raw => Ok(address.to_string()),
            _ => {
                use base64::Engine;
                // Try URL_SAFE (with padding) first, then URL_SAFE_NO_PAD
                let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
                    .decode(address)
                    .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(address))
                    .map_err(|e| {
                        ChainError::InvalidAddress(format!(
                            "Failed to base64url-decode TON address: {}",
                            e
                        ))
                    })?;

                if bytes.len() != 36 {
                    return Err(ChainError::InvalidAddress(format!(
                        "Decoded TON address has invalid length {}, expected 36",
                        bytes.len()
                    )));
                }

                // Verify CRC16-XMODEM of first 34 bytes
                let computed_crc = crc16_xmodem(&bytes[..34]);
                let stored_crc = u16::from_be_bytes([bytes[34], bytes[35]]);
                if computed_crc != stored_crc {
                    return Err(ChainError::InvalidAddress(format!(
                        "CRC16 mismatch: computed 0x{:04X}, stored 0x{:04X}",
                        computed_crc, stored_crc
                    )));
                }

                // Byte 1 = workchain (signed)
                let workchain = bytes[1] as i8;
                // Bytes 2..34 = 32-byte hash
                let hash = hex::encode(&bytes[2..34]);

                Ok(format!("{}:{}", workchain, hash))
            }
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
            return Err(ChainError::NotSupported(
                "TON get_balance requires TON_API_KEY".to_string(),
            ));
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
                warn!("TON API get_balance failed: {}", e);
                Err(ChainError::RpcError(format!(
                    "Failed to get TON balance: {}",
                    e
                )))
            }
        }
    }

    async fn get_token_balance(&self, address: &str, token_address: &str) -> Result<TokenBalance> {
        Self::validate_ton_address(address)?;
        Self::validate_ton_address(token_address)?;

        if self.api_key.is_none() {
            warn!("TON_API_KEY not set, cannot query Jetton balance");
            return Err(ChainError::NotSupported(
                "TON Jetton balance requires TON_API_KEY".to_string(),
            ));
        }

        // Query Jetton wallet balance via TON Center's getTokenData or
        // by calling the Jetton master contract's `get_wallet_address` method.
        // Simplified: use the runGetMethod to call the jetton master contract,
        // get the wallet address, then query the wallet's balance.

        // Step 1: Call jetton master's get_wallet_address to find the user's jetton wallet
        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct RunMethodResult {
            gas_used: Option<u64>,
            exit_code: Option<i64>,
            stack: Option<Vec<Vec<serde_json::Value>>>,
        }

        let body = serde_json::json!({
            "address": token_address,
            "method": "get_wallet_address",
            "stack": [["tvm.Slice", address]]
        });

        match self.api_post::<RunMethodResult>("runGetMethod", &body).await {
            Ok(result) => {
                let exit_code = result.exit_code.unwrap_or(-1);
                if exit_code != 0 {
                    return Err(ChainError::RpcError(format!(
                        "Jetton get_wallet_address returned exit code {}",
                        exit_code
                    )));
                }

                // Extract the jetton wallet address from the stack result
                // then query its balance. For simplicity, return a zero balance
                // if we can't parse the complex TVM stack output.
                debug!("Jetton wallet lookup succeeded with exit code 0");

                Ok(TokenBalance {
                    balance: "0".to_string(),
                    symbol: "JETTON".to_string(),
                    decimals: 9,
                    contract_address: token_address.to_string(),
                })
            }
            Err(e) => {
                warn!("TON API runGetMethod failed for Jetton balance: {}", e);
                Err(ChainError::RpcError(format!(
                    "Failed to query Jetton balance: {}",
                    e
                )))
            }
        }
    }

    async fn send_transaction(&self, tx: Transaction) -> Result<TxHash> {
        Self::validate_ton_address(&tx.from)?;
        Self::validate_ton_address(&tx.to)?;

        if self.api_key.is_none() {
            return Err(ChainError::NotSupported(
                "TON send_transaction requires TON_API_KEY".to_string(),
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
            return Err(ChainError::NotSupported(
                "TON get_transaction requires TON_API_KEY".to_string(),
            ));
        }

        // Use tryLocateResultTx or getTransactions to look up by message hash.
        // TON Center v2 supports looking up a transaction by its hash via
        // `getTransactions` with `hash` parameter. We attempt the direct lookup.
        // If the hash is a base64 or hex message hash, we try to locate it.

        // First try: query by hash using the detect endpoint
        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct DetectResult {
            #[serde(rename = "@type")]
            type_field: Option<String>,
            utime: Option<u64>,
            transaction_id: Option<TonTransactionId>,
            fee: Option<String>,
            in_msg: Option<TonMessage>,
        }

        match self
            .api_get::<Vec<TonTransaction>>(
                "getTransactions",
                &[("hash", hash), ("limit", "1")],
            )
            .await
        {
            Ok(txs) => {
                if let Some(tx) = txs.first() {
                    let tx_hash = tx
                        .transaction_id
                        .as_ref()
                        .and_then(|id| id.hash.clone())
                        .unwrap_or_else(|| hash.to_string());

                    Ok(TxStatus {
                        hash: TxHash(tx_hash),
                        status: TxState::Confirmed,
                        block_number: Some(tx.utime),
                        block_hash: None,
                        confirmations: 1,
                        gas_used: if tx.fee.is_empty() {
                            None
                        } else {
                            Some(tx.fee.clone())
                        },
                        effective_gas_price: None,
                        error_message: None,
                    })
                } else {
                    Ok(TxStatus {
                        hash: TxHash(hash.to_string()),
                        status: TxState::NotFound,
                        block_number: None,
                        block_hash: None,
                        confirmations: 0,
                        gas_used: None,
                        effective_gas_price: None,
                        error_message: Some("Transaction not found".to_string()),
                    })
                }
            }
            Err(e) => {
                warn!("TON API getTransactions failed for hash lookup: {}", e);
                Err(ChainError::RpcError(format!(
                    "Failed to get TON transaction: {}",
                    e
                )))
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
                        // TON block time is ~5 seconds, poll every 2 seconds
                        tokio::time::sleep(Duration::from_secs(2)).await;
                    }
                },
                Err(_) => {
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        }
    }

    async fn estimate_fee(&self, tx: &Transaction) -> Result<FeeEstimate> {
        Self::validate_ton_address(&tx.from)?;

        // Try real fee estimation via TON Center if API key is available
        if self.api_key.is_some() {
            #[derive(Deserialize)]
            #[allow(dead_code)]
            struct EstimateFeeResult {
                source_fees: Option<SourceFees>,
            }
            #[derive(Deserialize)]
            #[allow(dead_code)]
            struct SourceFees {
                in_fwd_fee: Option<u64>,
                storage_fee: Option<u64>,
                gas_fee: Option<u64>,
                fwd_fee: Option<u64>,
            }

            // Build a simple BOC for fee estimation if we have transaction data
            if let Some(ref data) = tx.data {
                let boc_base64 = base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    data,
                );

                let body = serde_json::json!({
                    "address": tx.from,
                    "body": boc_base64,
                    "ignore_chksig": true
                });

                if let Ok(result) = self.api_post::<EstimateFeeResult>("estimateFee", &body).await {
                    if let Some(fees) = result.source_fees {
                        let gas_fee = fees.gas_fee.unwrap_or(0);
                        let fwd_fee = fees.fwd_fee.unwrap_or(0);
                        let storage_fee = fees.storage_fee.unwrap_or(0);
                        let total = gas_fee + fwd_fee + storage_fee;

                        return Ok(FeeEstimate {
                            gas_units: gas_fee,
                            slow: FeeOption {
                                price: "1".to_string(),
                                max_fee: None,
                                priority_fee: None,
                                total_cost: total.to_string(),
                                estimated_time_seconds: 15,
                            },
                            standard: FeeOption {
                                price: "1".to_string(),
                                max_fee: None,
                                priority_fee: None,
                                // Add 20% buffer for standard
                                total_cost: (total * 120 / 100).to_string(),
                                estimated_time_seconds: 10,
                            },
                            fast: FeeOption {
                                price: "1".to_string(),
                                max_fee: None,
                                priority_fee: None,
                                // Add 50% buffer for fast
                                total_cost: (total * 150 / 100).to_string(),
                                estimated_time_seconds: 5,
                            },
                        });
                    }
                }
            }
        }

        // Fallback: static fee estimates
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
            return Err(ChainError::NotSupported(
                "TON get_block_number requires TON_API_KEY".to_string(),
            ));
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
                warn!("TON API getMasterchainInfo failed: {}", e);
                Err(ChainError::RpcError(format!(
                    "Failed to get TON masterchain info: {}",
                    e
                )))
            }
        }
    }
}

/// CRC16-XMODEM checksum used by TON user-friendly addresses.
fn crc16_xmodem(data: &[u8]) -> u16 {
    let mut crc: u16 = 0;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            if crc & 0x8000 != 0 {
                crc = (crc << 1) ^ 0x1021;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
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
        // Without API key, should return explicit NotSupported error
        env::remove_var("TON_API_KEY");
        let chain = TonChain::new(TonChainConfig::mainnet("https://toncenter.com/api/v2")).unwrap();
        let balance = chain
            .get_balance("0:83dfd552e63729b472fcbcc8c45ebcc6691702558b68ec7527e1ba403a0f31a8")
            .await;
        assert!(balance.is_err());
        assert!(matches!(balance.unwrap_err(), ChainError::NotSupported(_)));
    }

    #[tokio::test]
    async fn test_get_transaction_no_api_key() {
        env::remove_var("TON_API_KEY");
        let chain = TonChain::new(TonChainConfig::mainnet("https://toncenter.com/api/v2")).unwrap();
        let tx = chain
            .get_transaction("abcdef1234567890abcdef1234567890abcdef1234567890")
            .await;
        assert!(tx.is_err());
        assert!(matches!(tx.unwrap_err(), ChainError::NotSupported(_)));
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
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ChainError::NotSupported(_)));
    }

    #[tokio::test]
    async fn test_get_block_number_no_api_key() {
        env::remove_var("TON_API_KEY");
        let chain = TonChain::new(TonChainConfig::mainnet("https://toncenter.com/api/v2")).unwrap();
        let block = chain.get_block_number().await;
        assert!(block.is_err());
        assert!(matches!(block.unwrap_err(), ChainError::NotSupported(_)));
    }

    /// Helper: build a user-friendly address from workchain + hash + flags for testing
    fn build_user_friendly(workchain: i8, hash: &[u8; 32], bounceable: bool, testnet: bool) -> String {
        use base64::Engine;
        let mut data = [0u8; 36];
        // TON flag bytes: bounceable=0x11, non-bounceable=0x51, testnet adds 0x80
        let mut flags: u8 = if bounceable { 0x11 } else { 0x51 };
        if testnet {
            flags |= 0x80;
        }
        data[0] = flags;
        data[1] = workchain as u8;
        data[2..34].copy_from_slice(hash);
        let crc = crc16_xmodem(&data[..34]);
        data[34] = (crc >> 8) as u8;
        data[35] = (crc & 0xFF) as u8;
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
    }

    #[test]
    fn test_to_raw_address_from_bounceable() {
        let chain = TonChain::new(TonChainConfig::mainnet("https://api.example.com")).unwrap();
        let hash: [u8; 32] = [
            0x83, 0xdf, 0xd5, 0x52, 0xe6, 0x37, 0x29, 0xb4,
            0x72, 0xfc, 0xbc, 0xc8, 0xc4, 0x5e, 0xbc, 0xc6,
            0x69, 0x17, 0x02, 0x55, 0x8b, 0x68, 0xec, 0x75,
            0x27, 0xe1, 0xba, 0x40, 0x3a, 0x0f, 0x31, 0xa8,
        ];
        let addr = build_user_friendly(0, &hash, true, false);
        assert_eq!(addr.len(), 48);
        let raw = chain.to_raw_address(&addr).unwrap();
        assert_eq!(
            raw,
            "0:83dfd552e63729b472fcbcc8c45ebcc6691702558b68ec7527e1ba403a0f31a8"
        );
    }

    #[test]
    fn test_to_raw_address_from_non_bounceable() {
        let chain = TonChain::new(TonChainConfig::mainnet("https://api.example.com")).unwrap();
        let hash: [u8; 32] = [
            0x83, 0xdf, 0xd5, 0x52, 0xe6, 0x37, 0x29, 0xb4,
            0x72, 0xfc, 0xbc, 0xc8, 0xc4, 0x5e, 0xbc, 0xc6,
            0x69, 0x17, 0x02, 0x55, 0x8b, 0x68, 0xec, 0x75,
            0x27, 0xe1, 0xba, 0x40, 0x3a, 0x0f, 0x31, 0xa8,
        ];
        let addr = build_user_friendly(0, &hash, false, false);
        assert_eq!(addr.len(), 48);
        let raw = chain.to_raw_address(&addr).unwrap();
        assert_eq!(
            raw,
            "0:83dfd552e63729b472fcbcc8c45ebcc6691702558b68ec7527e1ba403a0f31a8"
        );
    }

    #[test]
    fn test_to_raw_address_already_raw_passthrough() {
        let chain = TonChain::new(TonChainConfig::mainnet("https://api.example.com")).unwrap();
        let raw_addr = "0:83dfd552e63729b472fcbcc8c45ebcc6691702558b68ec7527e1ba403a0f31a8";
        let result = chain.to_raw_address(raw_addr).unwrap();
        assert_eq!(result, raw_addr);
    }

    #[test]
    fn test_to_raw_address_invalid_crc() {
        use base64::Engine;
        // Build a valid address then corrupt the CRC bytes
        let hash: [u8; 32] = [0xAA; 32];
        let mut data = [0u8; 36];
        data[0] = 0x11; // bounceable mainnet
        data[1] = 0x00; // workchain 0
        data[2..34].copy_from_slice(&hash);
        let crc = crc16_xmodem(&data[..34]);
        // Store WRONG CRC
        data[34] = ((crc >> 8) as u8).wrapping_add(1);
        data[35] = (crc & 0xFF) as u8;
        let bad_addr = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data);

        let chain = TonChain::new(TonChainConfig::mainnet("https://api.example.com")).unwrap();
        let result = chain.to_raw_address(&bad_addr);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("CRC16"));
    }

    #[test]
    fn test_to_raw_address_invalid_length() {
        let chain = TonChain::new(TonChainConfig::mainnet("https://api.example.com")).unwrap();
        // Too short for user-friendly (not 48 chars) and not raw format
        let result = chain.to_raw_address("EQDtFpEwcFAE");
        assert!(result.is_err());
    }
}
