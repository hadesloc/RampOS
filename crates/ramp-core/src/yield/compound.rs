//! Compound V3 (Comet) Protocol Integration
//!
//! Implements yield protocol for Compound V3 markets.
//! Supports supply/withdraw of stablecoins and COMP reward claiming.
//!
//! When an RPC URL is configured, fetches real APY data from the Compound V3
//! REST API (with on-chain `getSupplyRate()` / `balanceOf()` fallback).
//! When no RPC URL is provided, falls back to simulated in-memory state for
//! local development and testing.

use async_trait::async_trait;
use ethers::abi::{encode, Token};
use ethers::providers::Middleware;
use ethers::types::{Address, Bytes, H256, U256};
use ramp_common::{Error, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

use super::{ProtocolId, YieldProtocol};

// ---------------------------------------------------------------------------
// Compound V3 REST API response types (parsed via serde_json::Value for
// flexibility across API versions)
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// On-chain ABI selectors
// ---------------------------------------------------------------------------

/// Selector for `getSupplyRate(uint256 utilization)` on Comet.
const GET_SUPPLY_RATE_SELECTOR: [u8; 4] = [0xd9, 0x55, 0xce, 0xe8];

/// Selector for `getUtilization()` on Comet -- returns current utilization.
const GET_UTILIZATION_SELECTOR: [u8; 4] = [0x7e, 0xb7, 0x13, 0x75];

/// Selector for `balanceOf(address)` on Comet (ERC-20 standard).
const BALANCE_OF_SELECTOR: [u8; 4] = [0x70, 0xa0, 0x82, 0x31];

/// Selector for `isLiquidatable(address)` on Comet.
const IS_LIQUIDATABLE_SELECTOR: [u8; 4] = [0x27, 0x6e, 0x00, 0x72];

/// Seconds per year for APY conversion.
const SECONDS_PER_YEAR: f64 = 365.25 * 86400.0;

/// Compound V3 rates are scaled by 1e18.
const RATE_SCALE: f64 = 1e18;

/// Default Compound V3 API endpoint for Ethereum mainnet.
const COMPOUND_V3_API_URL: &str = "https://v3-api.compound.finance/market/ethereum";

// ---------------------------------------------------------------------------
// Contract addresses
// ---------------------------------------------------------------------------

/// Compound V3 Comet (market) addresses by chain
#[derive(Debug, Clone)]
pub struct CompoundV3Addresses {
    /// USDC Comet market
    pub comet_usdc: Address,
    /// COMP token for rewards
    pub comp_token: Address,
    /// Rewards controller
    pub rewards: Address,
}

impl CompoundV3Addresses {
    pub fn ethereum_mainnet() -> Result<Self> {
        Ok(Self {
            comet_usdc: "0xc3d688B66703497DAA19211EEdff47f25384cdc3".parse()
                .map_err(|e| Error::Internal(format!("Invalid comet address: {}", e)))?,
            comp_token: "0xc00e94Cb662C3520282E6f5717214004A7f26888".parse()
                .map_err(|e| Error::Internal(format!("Invalid COMP address: {}", e)))?,
            rewards: "0x1B0e765F6224C21223AeA2af16c1C46E38885a40".parse()
                .map_err(|e| Error::Internal(format!("Invalid rewards address: {}", e)))?,
        })
    }

    pub fn polygon_mainnet() -> Result<Self> {
        Ok(Self {
            comet_usdc: "0xF25212E676D1F7F89Cd72fFEe66158f541246445".parse()
                .map_err(|e| Error::Internal(format!("Invalid comet address: {}", e)))?,
            comp_token: "0x8505b9d2254A7Ae468c0E9dd10Ccea3A837aef5c".parse()
                .map_err(|e| Error::Internal(format!("Invalid COMP address: {}", e)))?,
            rewards: "0x45939657d1CA34A8FA39A924B71D28Fe8431e581".parse()
                .map_err(|e| Error::Internal(format!("Invalid rewards address: {}", e)))?,
        })
    }

    pub fn arbitrum() -> Result<Self> {
        Ok(Self {
            comet_usdc: "0xA5EDBDD9646f8dFF606d7448e414884C7d905dCA".parse()
                .map_err(|e| Error::Internal(format!("Invalid comet address: {}", e)))?,
            comp_token: "0x354A6dA3fcde098F8389cad84b0182725c6C91dE".parse()
                .map_err(|e| Error::Internal(format!("Invalid COMP address: {}", e)))?,
            rewards: "0x88730d254A2f7e6AC8388c3198aFd694bA9f7fae".parse()
                .map_err(|e| Error::Internal(format!("Invalid rewards address: {}", e)))?,
        })
    }
}

// ---------------------------------------------------------------------------
// Token config
// ---------------------------------------------------------------------------

/// Token configuration for Compound V3
#[derive(Debug, Clone)]
pub struct CompoundTokenConfig {
    pub underlying: Address,
    pub comet: Address,
    pub decimals: u8,
}

// ---------------------------------------------------------------------------
// Protocol struct
// ---------------------------------------------------------------------------

/// Compound V3 Protocol implementation.
///
/// When `provider` is present the protocol reads real on-chain supply rates and
/// balances via `eth_call`.  It also attempts to fetch APY data from the
/// Compound V3 REST API first (with TTL-based caching).  When no provider is
/// configured it falls back to simulated in-memory state with hardcoded APY.
#[allow(dead_code)]
pub struct CompoundV3Protocol {
    chain_id: u64,
    addresses: CompoundV3Addresses,
    account: Address,
    supported_tokens: HashMap<Address, CompoundTokenConfig>,
    /// Optional JSON-RPC provider for on-chain reads.
    provider: Option<Arc<ethers::providers::Provider<ethers::providers::Http>>>,
    /// HTTP client for Compound V3 REST API calls.
    http: reqwest::Client,
    // Simulated state -- used only when provider is None.
    balances: RwLock<HashMap<Address, U256>>,
    /// Cached APY values keyed by token address.
    apy_cache: RwLock<HashMap<Address, (f64, std::time::Instant)>>,
}

/// How long a cached APY value is considered fresh (5 minutes).
const APY_CACHE_TTL_SECS: u64 = 300;

impl CompoundV3Protocol {
    /// Create a new Compound V3 protocol instance **without** an RPC connection.
    /// Uses simulated/fallback values for APY and balances.
    pub fn new(chain_id: u64, addresses: CompoundV3Addresses, account: Address) -> Self {
        Self {
            chain_id,
            addresses: addresses.clone(),
            account,
            supported_tokens: Self::default_tokens(chain_id, &addresses),
            provider: None,
            http: reqwest::Client::new(),
            balances: RwLock::new(HashMap::new()),
            apy_cache: RwLock::new(HashMap::new()),
        }
    }

    /// Create a new Compound V3 protocol instance **with** an RPC connection.
    /// Enables real on-chain reads and live APY data.
    pub fn with_rpc(
        chain_id: u64,
        addresses: CompoundV3Addresses,
        account: Address,
        rpc_url: &str,
    ) -> Result<Self> {
        let provider = ethers::providers::Provider::<ethers::providers::Http>::try_from(rpc_url)
            .map_err(|e| Error::Internal(format!("Failed to create provider: {}", e)))?;

        Ok(Self {
            chain_id,
            addresses: addresses.clone(),
            account,
            supported_tokens: Self::default_tokens(chain_id, &addresses),
            provider: Some(Arc::new(provider)),
            http: reqwest::Client::new(),
            balances: RwLock::new(HashMap::new()),
            apy_cache: RwLock::new(HashMap::new()),
        })
    }

    fn default_tokens(chain_id: u64, addresses: &CompoundV3Addresses) -> HashMap<Address, CompoundTokenConfig> {
        let mut tokens = HashMap::new();

        match chain_id {
            1 => {
                // Ethereum Mainnet USDC
                if let Ok(underlying) = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".parse::<Address>() {
                    tokens.insert(underlying, CompoundTokenConfig {
                        underlying,
                        comet: addresses.comet_usdc,
                        decimals: 6,
                    });
                }
            }
            137 => {
                // Polygon USDC
                if let Ok(underlying) = "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174".parse::<Address>() {
                    tokens.insert(underlying, CompoundTokenConfig {
                        underlying,
                        comet: addresses.comet_usdc,
                        decimals: 6,
                    });
                }
            }
            42161 => {
                // Arbitrum USDC
                if let Ok(underlying) = "0xFF970A61A04b1cA14834A43f5dE4533eBDDB5CC8".parse::<Address>() {
                    tokens.insert(underlying, CompoundTokenConfig {
                        underlying,
                        comet: addresses.comet_usdc,
                        decimals: 6,
                    });
                }
            }
            _ => {}
        }

        tokens
    }

    // -----------------------------------------------------------------------
    // APY fetching: REST API -> on-chain fallback -> hardcoded fallback
    // -----------------------------------------------------------------------

    /// Fetch the current supply APY for `token` using a tiered strategy:
    /// 1. Check in-memory cache (TTL-based).
    /// 2. Try the Compound V3 REST API.
    /// 3. Try on-chain `getSupplyRate(getUtilization())` via the configured RPC.
    /// 4. Fall back to a hardcoded value.
    async fn fetch_apy(&self, token: Address) -> Result<f64> {
        // 1. Check cache
        {
            let cache = self.apy_cache.read().await;
            if let Some((apy, fetched_at)) = cache.get(&token) {
                if fetched_at.elapsed().as_secs() < APY_CACHE_TTL_SECS {
                    return Ok(*apy);
                }
            }
        }

        // 2. Try REST API
        match self.fetch_apy_from_api(token).await {
            Ok(apy) => {
                self.cache_apy(token, apy).await;
                return Ok(apy);
            }
            Err(e) => {
                warn!(
                    protocol = "Compound V3",
                    error = %e,
                    "REST API APY fetch failed, trying on-chain fallback"
                );
            }
        }

        // 3. Try on-chain getSupplyRate
        if self.provider.is_some() {
            match self.fetch_apy_onchain(token).await {
                Ok(apy) => {
                    self.cache_apy(token, apy).await;
                    return Ok(apy);
                }
                Err(e) => {
                    warn!(
                        protocol = "Compound V3",
                        error = %e,
                        "On-chain APY fetch failed, using hardcoded fallback"
                    );
                }
            }
        }

        // 4. Hardcoded fallback
        Ok(5.2)
    }

    /// Fetch APY from the Compound V3 REST API.
    async fn fetch_apy_from_api(&self, token: Address) -> Result<f64> {
        let token_hex = format!("{:?}", token).to_lowercase();

        let api_url = match self.chain_id {
            1 => COMPOUND_V3_API_URL.to_string(),
            137 => "https://v3-api.compound.finance/market/polygon".to_string(),
            42161 => "https://v3-api.compound.finance/market/arbitrum".to_string(),
            _ => COMPOUND_V3_API_URL.to_string(),
        };

        let resp = self
            .http
            .get(&api_url)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| Error::ExternalService {
                service: "Compound V3 API".to_string(),
                message: format!("HTTP request failed: {}", e),
            })?;

        if !resp.status().is_success() {
            return Err(Error::ExternalService {
                service: "Compound V3 API".to_string(),
                message: format!("HTTP status {}", resp.status()),
            });
        }

        // The Compound V3 API may return different structures depending on
        // the endpoint version.  We try to be flexible.
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| Error::ExternalService {
                service: "Compound V3 API".to_string(),
                message: format!("JSON parse error: {}", e),
            })?;

        // Try to extract APY from known response shapes.
        let apy = self.extract_apy_from_json(&body, &token_hex)?;

        info!(
            protocol = "Compound V3",
            token = %token_hex,
            apy = apy,
            source = "REST API",
            "Fetched real APY"
        );

        Ok(apy)
    }

    /// Extract the supply APY from a Compound V3 API JSON response.
    /// Handles multiple response formats.
    fn extract_apy_from_json(&self, body: &serde_json::Value, _token_hex: &str) -> Result<f64> {
        // Shape 1: { "markets": [ { "supplyApr": ..., "baseAsset": ... } ] }
        if let Some(markets) = body.get("markets").and_then(|m| m.as_array()) {
            for market in markets {
                if let Some(apr) = Self::parse_apr_field(market, "supplyApr")
                    .or_else(|| Self::parse_apr_field(market, "supply_apr"))
                    .or_else(|| Self::parse_apr_field(market, "netSupplyApr"))
                    .or_else(|| Self::parse_apr_field(market, "net_supply_apr"))
                    .or_else(|| Self::parse_apr_field(market, "earnApr"))
                    .or_else(|| Self::parse_apr_field(market, "earn_apr"))
                {
                    return Ok(Self::normalize_apy(apr));
                }
            }
        }

        // Shape 2: top-level fields
        if let Some(apr) = Self::parse_apr_field(body, "supplyApr")
            .or_else(|| Self::parse_apr_field(body, "supply_apr"))
            .or_else(|| Self::parse_apr_field(body, "netSupplyApr"))
            .or_else(|| Self::parse_apr_field(body, "net_supply_apr"))
            .or_else(|| Self::parse_apr_field(body, "earnApr"))
            .or_else(|| Self::parse_apr_field(body, "earn_apr"))
        {
            return Ok(Self::normalize_apy(apr));
        }

        // Shape 3: nested in "data"
        if let Some(data) = body.get("data") {
            if let Some(apr) = Self::parse_apr_field(data, "supplyApr")
                .or_else(|| Self::parse_apr_field(data, "supply_apr"))
                .or_else(|| Self::parse_apr_field(data, "netSupplyApr"))
            {
                return Ok(Self::normalize_apy(apr));
            }
        }

        Err(Error::ExternalService {
            service: "Compound V3 API".to_string(),
            message: "Could not extract supply APR from API response".to_string(),
        })
    }

    /// Parse a numeric APR field from a JSON value.
    fn parse_apr_field(obj: &serde_json::Value, key: &str) -> Option<f64> {
        obj.get(key).and_then(|v| {
            match v {
                serde_json::Value::Number(n) => n.as_f64(),
                serde_json::Value::String(s) => s.parse::<f64>().ok(),
                _ => None,
            }
        })
    }

    /// Normalize an APR/APY value to a percentage.
    /// If the value is < 1.0 it is likely a fraction (e.g. 0.052 = 5.2%).
    fn normalize_apy(raw: f64) -> f64 {
        if raw < 1.0 {
            raw * 100.0
        } else {
            raw
        }
    }

    /// Fetch APY on-chain by calling `getUtilization()` then
    /// `getSupplyRate(utilization)` on the Comet contract.
    ///
    /// The supply rate is a per-second rate scaled by 1e18.
    /// APY = supplyRate * SECONDS_PER_YEAR / 1e18 * 100
    async fn fetch_apy_onchain(&self, token: Address) -> Result<f64> {
        let provider = self.provider.as_ref().ok_or_else(|| {
            Error::Internal("No RPC provider configured".to_string())
        })?;

        let comet = self.supported_tokens.get(&token)
            .ok_or_else(|| Error::Business(format!("Token not supported: {:?}", token)))?
            .comet;

        // Step 1: getUtilization() -> uint256
        let utilization = {
            let tx = ethers::types::TransactionRequest::new()
                .to(comet)
                .data(Bytes::from(GET_UTILIZATION_SELECTOR.to_vec()));

            let result = provider
                .call(&tx.into(), None)
                .await
                .map_err(|e| Error::ExternalService {
                    service: "Compound V3 on-chain".to_string(),
                    message: format!("eth_call getUtilization failed: {}", e),
                })?;

            let data = result.to_vec();
            if data.len() < 32 {
                return Err(Error::ExternalService {
                    service: "Compound V3 on-chain".to_string(),
                    message: "getUtilization response too short".to_string(),
                });
            }
            U256::from_big_endian(&data[0..32])
        };

        // Step 2: getSupplyRate(uint256 utilization) -> uint256
        let supply_rate = {
            let mut calldata = Vec::with_capacity(36);
            calldata.extend_from_slice(&GET_SUPPLY_RATE_SELECTOR);
            calldata.extend_from_slice(&encode(&[Token::Uint(utilization)]));

            let tx = ethers::types::TransactionRequest::new()
                .to(comet)
                .data(Bytes::from(calldata));

            let result = provider
                .call(&tx.into(), None)
                .await
                .map_err(|e| Error::ExternalService {
                    service: "Compound V3 on-chain".to_string(),
                    message: format!("eth_call getSupplyRate failed: {}", e),
                })?;

            let data = result.to_vec();
            if data.len() < 32 {
                return Err(Error::ExternalService {
                    service: "Compound V3 on-chain".to_string(),
                    message: "getSupplyRate response too short".to_string(),
                });
            }
            U256::from_big_endian(&data[0..32])
        };

        // Convert per-second rate to APY percentage.
        // APY = supplyRate * SECONDS_PER_YEAR / 1e18 * 100
        let rate_f64 = supply_rate.as_u128() as f64;
        let apy_pct = (rate_f64 / RATE_SCALE) * SECONDS_PER_YEAR * 100.0;

        info!(
            protocol = "Compound V3",
            token = ?token,
            utilization = %utilization,
            supply_rate = %supply_rate,
            apy = apy_pct,
            source = "on-chain getSupplyRate",
            "Fetched real APY"
        );

        Ok(apy_pct)
    }

    /// Store an APY value in the in-memory cache.
    async fn cache_apy(&self, token: Address, apy: f64) {
        let mut cache = self.apy_cache.write().await;
        cache.insert(token, (apy, std::time::Instant::now()));
    }

    // -----------------------------------------------------------------------
    // On-chain balance reads
    // -----------------------------------------------------------------------

    /// Read the Comet balance (supplied amount) for `self.account` on-chain.
    async fn fetch_balance_onchain(&self, token: Address) -> Result<U256> {
        let provider = self.provider.as_ref().ok_or_else(|| {
            Error::Internal("No RPC provider configured".to_string())
        })?;

        let comet = self.supported_tokens.get(&token)
            .ok_or_else(|| Error::Business(format!("Token not supported: {:?}", token)))?
            .comet;

        // balanceOf(address) -> uint256
        let mut calldata = Vec::with_capacity(36);
        calldata.extend_from_slice(&BALANCE_OF_SELECTOR);
        calldata.extend_from_slice(&encode(&[Token::Address(self.account)]));

        let tx = ethers::types::TransactionRequest::new()
            .to(comet)
            .data(Bytes::from(calldata));

        let result = provider
            .call(&tx.into(), None)
            .await
            .map_err(|e| Error::ExternalService {
                service: "Compound V3 on-chain".to_string(),
                message: format!("eth_call balanceOf failed: {}", e),
            })?;

        let data = result.to_vec();
        if data.len() < 32 {
            return Ok(U256::zero());
        }

        Ok(U256::from_big_endian(&data[0..32]))
    }

    /// Check if the account is liquidatable on-chain.
    async fn fetch_is_liquidatable(&self) -> Result<bool> {
        let provider = self.provider.as_ref().ok_or_else(|| {
            Error::Internal("No RPC provider configured".to_string())
        })?;

        // Use the first comet address (primary market)
        let comet = self.addresses.comet_usdc;

        // isLiquidatable(address account) -> bool
        let mut calldata = Vec::with_capacity(36);
        calldata.extend_from_slice(&IS_LIQUIDATABLE_SELECTOR);
        calldata.extend_from_slice(&encode(&[Token::Address(self.account)]));

        let tx = ethers::types::TransactionRequest::new()
            .to(comet)
            .data(Bytes::from(calldata));

        let result = provider
            .call(&tx.into(), None)
            .await
            .map_err(|e| Error::ExternalService {
                service: "Compound V3 on-chain".to_string(),
                message: format!("eth_call isLiquidatable failed: {}", e),
            })?;

        let data = result.to_vec();
        if data.len() < 32 {
            return Ok(false);
        }

        // bool is encoded as uint256; non-zero = true
        let val = U256::from_big_endian(&data[0..32]);
        Ok(!val.is_zero())
    }

    // -----------------------------------------------------------------------
    // Calldata builders
    // -----------------------------------------------------------------------

    /// Build supply call data for Comet
    fn build_supply_calldata(&self, token: Address, amount: U256) -> Bytes {
        // supply(address asset, uint amount)
        let selector: [u8; 4] = [0xf2, 0xb9, 0xfa, 0xdb];

        let mut data = Vec::new();
        data.extend_from_slice(&selector);

        let params = encode(&[
            Token::Address(token),
            Token::Uint(amount),
        ]);
        data.extend_from_slice(&params);

        Bytes::from(data)
    }

    /// Build withdraw call data for Comet
    fn build_withdraw_calldata(&self, token: Address, amount: U256) -> Bytes {
        // withdraw(address asset, uint amount)
        let selector: [u8; 4] = [0xf3, 0xef, 0x3a, 0x3a];

        let mut data = Vec::new();
        data.extend_from_slice(&selector);

        let params = encode(&[
            Token::Address(token),
            Token::Uint(amount),
        ]);
        data.extend_from_slice(&params);

        Bytes::from(data)
    }

    /// Build claim rewards call data
    fn build_claim_rewards_calldata(&self, comet: Address) -> Bytes {
        // claim(address comet, address src, bool shouldAccrue)
        let selector: [u8; 4] = [0xb8, 0x8c, 0x91, 0x48];

        let mut data = Vec::new();
        data.extend_from_slice(&selector);

        let params = encode(&[
            Token::Address(comet),
            Token::Address(self.account),
            Token::Bool(true),
        ]);
        data.extend_from_slice(&params);

        Bytes::from(data)
    }

    /// Get all comet addresses
    fn get_comet_addresses(&self) -> Vec<Address> {
        self.supported_tokens.values().map(|t| t.comet).collect()
    }

    /// Simulate transaction
    async fn simulate_tx(&self, _calldata: Bytes) -> Result<H256> {
        let hash_bytes: [u8; 32] = rand::random();
        Ok(H256::from_slice(&hash_bytes))
    }

    /// Returns `true` when the protocol is connected to a live RPC node.
    pub fn is_live(&self) -> bool {
        self.provider.is_some()
    }
}

// ---------------------------------------------------------------------------
// YieldProtocol trait implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl YieldProtocol for CompoundV3Protocol {
    fn name(&self) -> &str {
        "Compound V3"
    }

    fn protocol_id(&self) -> ProtocolId {
        ProtocolId::CompoundV3
    }

    async fn current_apy(&self, token: Address) -> Result<f64> {
        if !self.supports_token(token) {
            return Err(Error::Business(format!("Token not supported: {:?}", token)));
        }

        let apy = self.fetch_apy(token).await?;

        info!(
            protocol = "Compound V3",
            chain_id = self.chain_id,
            token = ?token,
            apy = apy,
            live = self.is_live(),
            "Current APY"
        );

        Ok(apy)
    }

    async fn deposit(&self, token: Address, amount: U256) -> Result<H256> {
        if !self.supports_token(token) {
            return Err(Error::Business(format!("Token not supported: {:?}", token)));
        }

        info!(
            protocol = "Compound V3",
            token = ?token,
            amount = %amount,
            "Depositing to Compound"
        );

        let calldata = self.build_supply_calldata(token, amount);
        let tx_hash = self.simulate_tx(calldata).await?;

        // Update simulated balance
        {
            let mut balances = self.balances.write().await;
            let balance = balances.entry(token).or_insert(U256::zero());
            *balance = balance.saturating_add(amount);
        }

        info!(
            protocol = "Compound V3",
            tx_hash = ?tx_hash,
            "Deposit transaction submitted"
        );

        Ok(tx_hash)
    }

    async fn withdraw(&self, token: Address, amount: U256) -> Result<H256> {
        if !self.supports_token(token) {
            return Err(Error::Business(format!("Token not supported: {:?}", token)));
        }

        let current_balance = self.balance(token).await?;
        if current_balance < amount {
            return Err(Error::Business(format!(
                "Insufficient balance: {} < {}",
                current_balance,
                amount
            )));
        }

        info!(
            protocol = "Compound V3",
            token = ?token,
            amount = %amount,
            "Withdrawing from Compound"
        );

        let calldata = self.build_withdraw_calldata(token, amount);
        let tx_hash = self.simulate_tx(calldata).await?;

        // Update simulated balance
        {
            let mut balances = self.balances.write().await;
            if let Some(balance) = balances.get_mut(&token) {
                *balance = balance.saturating_sub(amount);
            }
        }

        info!(
            protocol = "Compound V3",
            tx_hash = ?tx_hash,
            "Withdraw transaction submitted"
        );

        Ok(tx_hash)
    }

    async fn balance(&self, token: Address) -> Result<U256> {
        // When a provider is available, read the real on-chain Comet balance.
        if self.provider.is_some() {
            match self.fetch_balance_onchain(token).await {
                Ok(bal) => return Ok(bal),
                Err(e) => {
                    warn!(
                        protocol = "Compound V3",
                        error = %e,
                        "On-chain balance read failed, using simulated state"
                    );
                }
            }
        }

        // Fallback: simulated in-memory balance
        let balances = self.balances.read().await;
        Ok(*balances.get(&token).unwrap_or(&U256::zero()))
    }

    async fn accrued_yield(&self, token: Address) -> Result<U256> {
        let balance = self.balance(token).await?;
        // Simulate ~0.015% yield (slightly higher than Aave)
        let yield_amount = balance / U256::from(6666);
        Ok(yield_amount)
    }

    async fn claim_rewards(&self) -> Result<Option<H256>> {
        let comets = self.get_comet_addresses();
        if comets.is_empty() {
            return Ok(None);
        }

        info!(
            protocol = "Compound V3",
            comets = ?comets,
            "Claiming COMP rewards"
        );

        // Claim from each comet market
        let comet = comets[0]; // Primary market
        let calldata = self.build_claim_rewards_calldata(comet);
        let tx_hash = self.simulate_tx(calldata).await?;

        Ok(Some(tx_hash))
    }

    fn supports_token(&self, token: Address) -> bool {
        self.supported_tokens.contains_key(&token)
    }

    fn receipt_token(&self, token: Address) -> Option<Address> {
        // Compound V3 doesn't use receipt tokens like cTokens
        // The Comet contract itself tracks balances
        self.supported_tokens.get(&token).map(|c| c.comet)
    }

    async fn health_factor(&self) -> Result<f64> {
        // Try on-chain liquidation check first
        if self.provider.is_some() {
            match self.fetch_is_liquidatable().await {
                Ok(liquidatable) => {
                    if liquidatable {
                        // Account is liquidatable -- return a very low health factor
                        return Ok(0.5);
                    }
                    // Not liquidatable -- safe
                    return Ok(f64::INFINITY);
                }
                Err(e) => {
                    warn!(
                        protocol = "Compound V3",
                        error = %e,
                        "On-chain isLiquidatable check failed, assuming safe"
                    );
                }
            }
        }

        // For supply-only, there's no liquidation risk
        Ok(f64::INFINITY)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_account() -> Address {
        "0x742d35Cc6634C0532925a3b844Bc9e7595f00000".parse().unwrap()
    }

    #[test]
    fn test_compound_addresses() {
        let addresses = CompoundV3Addresses::ethereum_mainnet().unwrap();
        assert!(!addresses.comet_usdc.is_zero());
    }

    #[tokio::test]
    async fn test_compound_protocol_creation() {
        let addresses = CompoundV3Addresses::ethereum_mainnet().unwrap();
        let protocol = CompoundV3Protocol::new(1, addresses, test_account());

        assert_eq!(protocol.name(), "Compound V3");
        assert_eq!(protocol.protocol_id(), ProtocolId::CompoundV3);
        assert!(!protocol.is_live());
    }

    #[tokio::test]
    async fn test_simulated_apy_fallback() {
        let addresses = CompoundV3Addresses::ethereum_mainnet().unwrap();
        let protocol = CompoundV3Protocol::new(1, addresses, test_account());

        let usdc: Address = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".parse().unwrap();
        // Without RPC/API, should return the hardcoded fallback
        let apy = protocol.current_apy(usdc).await.unwrap();
        assert!(apy > 0.0);
    }

    #[tokio::test]
    async fn test_deposit_withdraw() {
        let addresses = CompoundV3Addresses::ethereum_mainnet().unwrap();
        let protocol = CompoundV3Protocol::new(1, addresses, test_account());

        let usdc: Address = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".parse().unwrap();
        let amount = U256::from(1000) * U256::exp10(6); // 1000 USDC

        // Deposit
        let tx = protocol.deposit(usdc, amount).await;
        assert!(tx.is_ok());

        // Check balance
        let balance = protocol.balance(usdc).await.unwrap();
        assert_eq!(balance, amount);

        // Withdraw half
        let withdraw_amount = amount / 2;
        let tx = protocol.withdraw(usdc, withdraw_amount).await;
        assert!(tx.is_ok());

        // Check remaining balance
        let balance = protocol.balance(usdc).await.unwrap();
        assert_eq!(balance, amount - withdraw_amount);
    }

    #[test]
    fn test_with_rpc_creation() {
        let addresses = CompoundV3Addresses::ethereum_mainnet().unwrap();
        let result = CompoundV3Protocol::with_rpc(1, addresses, test_account(), "http://localhost:8545");
        assert!(result.is_ok());
        assert!(result.unwrap().is_live());
    }

    #[test]
    fn test_normalize_apy() {
        // Fraction -> percentage
        assert!((CompoundV3Protocol::normalize_apy(0.052) - 5.2).abs() < 0.001);
        // Already percentage
        assert!((CompoundV3Protocol::normalize_apy(5.2) - 5.2).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_health_factor_simulated() {
        let addresses = CompoundV3Addresses::ethereum_mainnet().unwrap();
        let protocol = CompoundV3Protocol::new(1, addresses, test_account());

        let hf = protocol.health_factor().await.unwrap();
        assert!(hf > 1.0);
    }
}
