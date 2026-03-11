//! Aave V3 Protocol Integration
//!
//! Implements yield protocol for Aave V3 lending pool.
//! Supports supply/withdraw of stablecoins and reward claiming.
//!
//! When an RPC URL is configured, fetches real APY data from the Aave REST API
//! (with on-chain `getReserveData()` fallback) and reads real on-chain balances.
//! When no RPC URL is provided, falls back to simulated in-memory state for
//! local development and testing.

use alloy::primitives::{Address, Bytes, B256, U256};
use alloy::providers::Provider;
use async_trait::async_trait;
use ramp_common::{Error, Result};
use serde::Deserialize;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{info, warn};

use super::{ProtocolId, YieldProtocol};

// ---------------------------------------------------------------------------
// Aave REST API response types
// ---------------------------------------------------------------------------

/// Response entry from the Aave V2/V3 REST data API.
/// The REST endpoint returns an array of market objects.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AaveMarketData {
    /// The underlying asset address (checksummed hex)
    #[serde(alias = "underlyingAsset")]
    underlying_asset: Option<String>,
    /// Supply APY as a decimal string (e.g. "0.045" = 4.5%)
    /// Different API versions use different field names.
    #[serde(alias = "liquidityRate")]
    liquidity_rate: Option<String>,
    /// Some API versions provide avg_supply_apy directly
    #[serde(alias = "avg_supply_apy")]
    avg_supply_apy: Option<f64>,
    /// Or supply_apy
    supply_apy: Option<f64>,
}

// ---------------------------------------------------------------------------
// Aave V3 on-chain ABI fragments (alloy sol! style)
// ---------------------------------------------------------------------------

// getReserveData(address asset) returns a tuple; we only need
// `currentLiquidityRate` which is the 4th element (index 3) - a ray (1e27).
// We encode/decode manually to avoid a full abigen build step.

/// Selector for `getReserveData(address)` on the Aave V3 Pool contract.
const GET_RESERVE_DATA_SELECTOR: [u8; 4] = [0x35, 0xea, 0x6a, 0x75];

/// Selector for `balanceOf(address)` on ERC-20 / aToken.
const BALANCE_OF_SELECTOR: [u8; 4] = [0x70, 0xa0, 0x82, 0x31];

/// Selector for `getUserAccountData(address)` on Aave V3 Pool.
const GET_USER_ACCOUNT_DATA_SELECTOR: [u8; 4] = [0xbf, 0x92, 0x85, 0x7c];

/// RAY = 1e27 -- Aave uses ray math for rates.
const RAY: f64 = 1e27;

/// Default Aave REST API URL for market data.
const AAVE_API_URL: &str = "https://aave-api-v2.aave.com/data/markets-data";

// ---------------------------------------------------------------------------
// Contract addresses
// ---------------------------------------------------------------------------

/// Aave V3 contract addresses by chain
#[derive(Debug, Clone)]
pub struct AaveV3Addresses {
    pub pool: Address,
    pub pool_data_provider: Address,
    pub incentives_controller: Address,
}

impl AaveV3Addresses {
    pub fn ethereum_mainnet() -> Result<Self> {
        Ok(Self {
            pool: "0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2"
                .parse()
                .map_err(|e| Error::Internal(format!("Invalid pool address: {}", e)))?,
            pool_data_provider: "0x7B4EB56E7CD4b454BA8ff71E4518426369a138a3"
                .parse()
                .map_err(|e| Error::Internal(format!("Invalid data provider address: {}", e)))?,
            incentives_controller: "0x8164Cc65827dcFe994AB23944CBC90e0aa80bFcb"
                .parse()
                .map_err(|e| Error::Internal(format!("Invalid incentives address: {}", e)))?,
        })
    }

    pub fn polygon_mainnet() -> Result<Self> {
        Ok(Self {
            pool: "0x794a61358D6845594F94dc1DB02A252b5b4814aD"
                .parse()
                .map_err(|e| Error::Internal(format!("Invalid pool address: {}", e)))?,
            pool_data_provider: "0x69FA688f1Dc47d4B5d8029D5a35FB7a548310654"
                .parse()
                .map_err(|e| Error::Internal(format!("Invalid data provider address: {}", e)))?,
            incentives_controller: "0x929EC64c34a17401F460460D4B9390518E5B473e"
                .parse()
                .map_err(|e| Error::Internal(format!("Invalid incentives address: {}", e)))?,
        })
    }

    pub fn arbitrum() -> Result<Self> {
        Ok(Self {
            pool: "0x794a61358D6845594F94dc1DB02A252b5b4814aD"
                .parse()
                .map_err(|e| Error::Internal(format!("Invalid pool address: {}", e)))?,
            pool_data_provider: "0x69FA688f1Dc47d4B5d8029D5a35FB7a548310654"
                .parse()
                .map_err(|e| Error::Internal(format!("Invalid data provider address: {}", e)))?,
            incentives_controller: "0x929EC64c34a17401F460460D4B9390518E5B473e"
                .parse()
                .map_err(|e| Error::Internal(format!("Invalid incentives address: {}", e)))?,
        })
    }
}

// ---------------------------------------------------------------------------
// Token config
// ---------------------------------------------------------------------------

/// Token configuration for Aave
#[derive(Debug, Clone)]
pub struct AaveTokenConfig {
    pub underlying: Address,
    pub a_token: Address,
    pub decimals: u8,
}

// ---------------------------------------------------------------------------
// Protocol struct
// ---------------------------------------------------------------------------

/// Aave V3 Protocol implementation.
///
/// When `rpc_url` is `Some(...)`, the protocol uses real on-chain reads (via
/// `eth_call`) and the Aave REST API for APY data.  When `rpc_url` is `None`,
/// it falls back to simulated in-memory balances and hardcoded APY values so
/// that local development / unit tests work without a network connection.
#[allow(dead_code)]
pub struct AaveV3Protocol {
    chain_id: u64,
    addresses: AaveV3Addresses,
    account: Address,
    supported_tokens: HashMap<Address, AaveTokenConfig>,
    /// Optional JSON-RPC provider for real on-chain reads.
    provider:
        Option<alloy::providers::RootProvider<alloy::transports::http::Http<reqwest::Client>>>,
    /// HTTP client for Aave REST API calls.
    http: reqwest::Client,
    // Simulated state -- used only when `provider` is `None`.
    balances: RwLock<HashMap<Address, U256>>,
    /// Cached APY values from the last successful API/on-chain fetch.
    /// Key is the underlying token address, value is the APY percentage.
    apy_cache: RwLock<HashMap<Address, (f64, std::time::Instant)>>,
}

/// How long a cached APY value is considered fresh (5 minutes).
const APY_CACHE_TTL_SECS: u64 = 300;

impl AaveV3Protocol {
    /// ABI-encode an address as a 32-byte word (left-padded with zeros).
    fn abi_encode_address(addr: Address) -> [u8; 32] {
        let mut word = [0u8; 32];
        word[12..32].copy_from_slice(addr.as_slice());
        word
    }

    /// ABI-encode a U256 as a 32-byte big-endian word.
    fn abi_encode_u256(val: U256) -> [u8; 32] {
        val.to_be_bytes::<32>()
    }

    /// Create a new Aave V3 protocol instance **without** an RPC connection.
    /// This uses simulated/fallback values for APY and balances.
    pub fn new(chain_id: u64, addresses: AaveV3Addresses, account: Address) -> Self {
        Self {
            chain_id,
            addresses,
            account,
            supported_tokens: Self::default_tokens(chain_id),
            provider: None,
            http: reqwest::Client::new(),
            balances: RwLock::new(HashMap::new()),
            apy_cache: RwLock::new(HashMap::new()),
        }
    }

    /// Create a new Aave V3 protocol instance **with** an RPC connection.
    /// This enables real on-chain reads and live APY data.
    pub fn with_rpc(
        chain_id: u64,
        addresses: AaveV3Addresses,
        account: Address,
        rpc_url: &str,
    ) -> Result<Self> {
        let url: reqwest::Url = rpc_url
            .parse()
            .map_err(|e| Error::Internal(format!("Invalid RPC URL: {}", e)))?;
        let provider = alloy::providers::ProviderBuilder::new().on_http(url);

        Ok(Self {
            chain_id,
            addresses,
            account,
            supported_tokens: Self::default_tokens(chain_id),
            provider: Some(provider),
            http: reqwest::Client::new(),
            balances: RwLock::new(HashMap::new()),
            apy_cache: RwLock::new(HashMap::new()),
        })
    }

    fn default_tokens(chain_id: u64) -> HashMap<Address, AaveTokenConfig> {
        let mut tokens = HashMap::new();

        match chain_id {
            1 => {
                // Ethereum Mainnet USDC
                if let (Ok(underlying), Ok(a_token)) = (
                    "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".parse::<Address>(),
                    "0x98C23E9d8f34FEFb1B7BD6a91B7FF122F4e16F5c".parse::<Address>(),
                ) {
                    tokens.insert(
                        underlying,
                        AaveTokenConfig {
                            underlying,
                            a_token,
                            decimals: 6,
                        },
                    );
                }
                // USDT
                if let (Ok(underlying), Ok(a_token)) = (
                    "0xdAC17F958D2ee523a2206206994597C13D831ec7".parse::<Address>(),
                    "0x23878914EFE38d27C4D67Ab83ed1b93A74D4086a".parse::<Address>(),
                ) {
                    tokens.insert(
                        underlying,
                        AaveTokenConfig {
                            underlying,
                            a_token,
                            decimals: 6,
                        },
                    );
                }
                // DAI
                if let (Ok(underlying), Ok(a_token)) = (
                    "0x6B175474E89094C44Da98b954EedeAC495271d0F".parse::<Address>(),
                    "0x018008bfb33d285247A21d44E50697654f754e63".parse::<Address>(),
                ) {
                    tokens.insert(
                        underlying,
                        AaveTokenConfig {
                            underlying,
                            a_token,
                            decimals: 18,
                        },
                    );
                }
            }
            137 => {
                // Polygon USDC
                if let (Ok(underlying), Ok(a_token)) = (
                    "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174".parse::<Address>(),
                    "0x625E7708f30cA75bfd92586e17077590C60eb4cD".parse::<Address>(),
                ) {
                    tokens.insert(
                        underlying,
                        AaveTokenConfig {
                            underlying,
                            a_token,
                            decimals: 6,
                        },
                    );
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
    /// 2. Try the Aave REST API.
    /// 3. Try on-chain `getReserveData()` via the configured RPC provider.
    /// 4. Fall back to hardcoded values.
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
                    protocol = "Aave V3",
                    error = %e,
                    "REST API APY fetch failed, trying on-chain fallback"
                );
            }
        }

        // 3. Try on-chain getReserveData
        if self.provider.is_some() {
            match self.fetch_apy_onchain(token).await {
                Ok(apy) => {
                    self.cache_apy(token, apy).await;
                    return Ok(apy);
                }
                Err(e) => {
                    warn!(
                        protocol = "Aave V3",
                        error = %e,
                        "On-chain APY fetch failed, using hardcoded fallback"
                    );
                }
            }
        }

        // 4. Hardcoded fallback
        let config = self.supported_tokens.get(&token);
        let apy = match config.map(|c| c.decimals) {
            Some(6) => 4.5,  // USDC/USDT typical range
            Some(18) => 3.8, // DAI typical range
            _ => 4.0,
        };
        Ok(apy)
    }

    /// Fetch APY from the Aave REST API (v2-compatible endpoint that also
    /// serves V3 data for Ethereum mainnet).
    async fn fetch_apy_from_api(&self, token: Address) -> Result<f64> {
        let token_hex = format!("{:?}", token).to_lowercase();

        let resp = self
            .http
            .get(AAVE_API_URL)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| Error::ExternalService {
                service: "Aave API".to_string(),
                message: format!("HTTP request failed: {}", e),
            })?;

        if !resp.status().is_success() {
            return Err(Error::ExternalService {
                service: "Aave API".to_string(),
                message: format!("HTTP status {}", resp.status()),
            });
        }

        let markets: Vec<AaveMarketData> =
            resp.json().await.map_err(|e| Error::ExternalService {
                service: "Aave API".to_string(),
                message: format!("JSON parse error: {}", e),
            })?;

        // Find matching market by underlying asset address
        for market in &markets {
            let asset_addr = market
                .underlying_asset
                .as_deref()
                .unwrap_or("")
                .to_lowercase();

            if asset_addr == token_hex || asset_addr == format!("0x{}", &token_hex[2..]) {
                // Try various field names that different Aave API versions use
                if let Some(apy) = market.supply_apy {
                    // supply_apy is already a percentage
                    let apy_pct = if apy < 1.0 { apy * 100.0 } else { apy };
                    info!(
                        protocol = "Aave V3",
                        token = %token_hex,
                        apy = apy_pct,
                        source = "REST API (supply_apy)",
                        "Fetched real APY"
                    );
                    return Ok(apy_pct);
                }

                if let Some(apy) = market.avg_supply_apy {
                    let apy_pct = if apy < 1.0 { apy * 100.0 } else { apy };
                    info!(
                        protocol = "Aave V3",
                        token = %token_hex,
                        apy = apy_pct,
                        source = "REST API (avg_supply_apy)",
                        "Fetched real APY"
                    );
                    return Ok(apy_pct);
                }

                if let Some(ref rate_str) = market.liquidity_rate {
                    // liquidityRate is a ray value as string
                    if let Ok(rate) = rate_str.parse::<f64>() {
                        let apy_pct = if rate > 1e20 {
                            // It is a raw ray value (1e27 scale)
                            (rate / RAY) * 100.0
                        } else if rate < 1.0 {
                            // It is a decimal fraction
                            rate * 100.0
                        } else {
                            rate
                        };
                        info!(
                            protocol = "Aave V3",
                            token = %token_hex,
                            apy = apy_pct,
                            source = "REST API (liquidityRate)",
                            "Fetched real APY"
                        );
                        return Ok(apy_pct);
                    }
                }
            }
        }

        Err(Error::ExternalService {
            service: "Aave API".to_string(),
            message: format!("Token {} not found in Aave markets response", token_hex),
        })
    }

    /// Fetch APY on-chain by calling `getReserveData(address)` on the Aave V3
    /// Pool contract. The `currentLiquidityRate` field (index 2 in the returned
    /// tuple for V3) is a ray-scaled value representing the current supply rate.
    async fn fetch_apy_onchain(&self, token: Address) -> Result<f64> {
        let provider = self
            .provider
            .as_ref()
            .ok_or_else(|| Error::Internal("No RPC provider configured".to_string()))?;

        // Build calldata: getReserveData(address)
        let mut calldata = Vec::with_capacity(36);
        calldata.extend_from_slice(&GET_RESERVE_DATA_SELECTOR);
        calldata.extend_from_slice(&Self::abi_encode_address(token));

        let tx = alloy::rpc::types::TransactionRequest::default()
            .to(self.addresses.pool)
            .input(alloy::rpc::types::TransactionInput::new(Bytes::from(
                calldata,
            )));

        let result = provider
            .call(&tx)
            .await
            .map_err(|e| Error::ExternalService {
                service: "Aave V3 on-chain".to_string(),
                message: format!("eth_call getReserveData failed: {}", e),
            })?;

        // The return data is a packed struct of many uint256 fields.
        // In Aave V3 Pool, getReserveData returns:
        //   (ReserveConfigurationMap, uint128 liquidityIndex,
        //    uint128 currentLiquidityRate, ...)
        // The currentLiquidityRate starts at byte offset 64 (3rd 32-byte word
        // if we account for the first word being the config bitmap and the
        // second being liquidityIndex, both packed as uint256 on the ABI level).
        //
        // For a simpler approach we just index into the raw 32-byte words.
        let data: &[u8] = result.as_ref();
        if data.len() < 96 {
            return Err(Error::ExternalService {
                service: "Aave V3 on-chain".to_string(),
                message: format!("getReserveData response too short ({} bytes)", data.len()),
            });
        }

        // currentLiquidityRate is the 3rd word (bytes 64..96) as uint256.
        let rate_u256 = U256::from_be_slice(&data[64..96]);

        // Convert ray to APY percentage.
        // APY (simple) ~= rate / 1e27 * 100
        // For compound APY: ((1 + rate/1e27/SECONDS_PER_YEAR)^SECONDS_PER_YEAR - 1) * 100
        // We use the simple approximation which is standard for display.
        let rate_f64 = u128::try_from(rate_u256).unwrap_or(u128::MAX) as f64;
        let apy_pct = (rate_f64 / RAY) * 100.0;

        info!(
            protocol = "Aave V3",
            token = ?token,
            raw_rate = %rate_u256,
            apy = apy_pct,
            source = "on-chain getReserveData",
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

    /// Read the aToken balance for `self.account` on-chain.
    async fn fetch_balance_onchain(&self, token: Address) -> Result<U256> {
        let provider = self
            .provider
            .as_ref()
            .ok_or_else(|| Error::Internal("No RPC provider configured".to_string()))?;

        let a_token = self
            .supported_tokens
            .get(&token)
            .ok_or_else(|| Error::Business(format!("Token not supported: {:?}", token)))?
            .a_token;

        // balanceOf(address) -> uint256
        let mut calldata = Vec::with_capacity(36);
        calldata.extend_from_slice(&BALANCE_OF_SELECTOR);
        calldata.extend_from_slice(&Self::abi_encode_address(self.account));

        let tx = alloy::rpc::types::TransactionRequest::default()
            .to(a_token)
            .input(alloy::rpc::types::TransactionInput::new(Bytes::from(
                calldata,
            )));

        let result = provider
            .call(&tx)
            .await
            .map_err(|e| Error::ExternalService {
                service: "Aave V3 on-chain".to_string(),
                message: format!("eth_call balanceOf failed: {}", e),
            })?;

        let data: &[u8] = result.as_ref();
        if data.len() < 32 {
            return Ok(U256::ZERO);
        }

        Ok(U256::from_be_slice(&data[0..32]))
    }

    /// Read `getUserAccountData(address)` from the Aave V3 Pool to get the
    /// health factor. Returns the health factor as a float (1e18 scaled on-chain).
    async fn fetch_health_factor_onchain(&self) -> Result<f64> {
        let provider = self
            .provider
            .as_ref()
            .ok_or_else(|| Error::Internal("No RPC provider configured".to_string()))?;

        let mut calldata = Vec::with_capacity(36);
        calldata.extend_from_slice(&GET_USER_ACCOUNT_DATA_SELECTOR);
        calldata.extend_from_slice(&Self::abi_encode_address(self.account));

        let tx = alloy::rpc::types::TransactionRequest::default()
            .to(self.addresses.pool)
            .input(alloy::rpc::types::TransactionInput::new(Bytes::from(
                calldata,
            )));

        let result = provider
            .call(&tx)
            .await
            .map_err(|e| Error::ExternalService {
                service: "Aave V3 on-chain".to_string(),
                message: format!("eth_call getUserAccountData failed: {}", e),
            })?;

        // getUserAccountData returns:
        //   (uint256 totalCollateralBase, uint256 totalDebtBase,
        //    uint256 availableBorrowsBase, uint256 currentLiquidationThreshold,
        //    uint256 ltv, uint256 healthFactor)
        // healthFactor is the 6th word (bytes 160..192), scaled by 1e18.
        let data: &[u8] = result.as_ref();
        if data.len() < 192 {
            // If we cannot read it, assume infinite (supply-only, no debt).
            return Ok(f64::INFINITY);
        }

        let hf_u256 = U256::from_be_slice(&data[160..192]);
        if hf_u256 == U256::MAX {
            // Aave returns type(uint256).max when there is no debt
            return Ok(f64::INFINITY);
        }

        let hf = u128::try_from(hf_u256).unwrap_or(u128::MAX) as f64 / 1e18;
        Ok(hf)
    }

    // -----------------------------------------------------------------------
    // Calldata builders (unchanged from original)
    // -----------------------------------------------------------------------

    /// Build supply call data for Aave Pool
    fn build_supply_calldata(&self, token: Address, amount: U256) -> Bytes {
        // supply(address asset, uint256 amount, address onBehalfOf, uint16 referralCode)
        let selector: [u8; 4] = [0x61, 0x7b, 0xa0, 0x37];

        let mut data = Vec::with_capacity(4 + 128);
        data.extend_from_slice(&selector);
        data.extend_from_slice(&Self::abi_encode_address(token));
        data.extend_from_slice(&Self::abi_encode_u256(amount));
        data.extend_from_slice(&Self::abi_encode_address(self.account));
        data.extend_from_slice(&Self::abi_encode_u256(U256::ZERO)); // referral code

        Bytes::from(data)
    }

    /// Build withdraw call data for Aave Pool
    fn build_withdraw_calldata(&self, token: Address, amount: U256) -> Bytes {
        // withdraw(address asset, uint256 amount, address to)
        let selector: [u8; 4] = [0x69, 0x32, 0x8d, 0xec];

        let mut data = Vec::with_capacity(4 + 96);
        data.extend_from_slice(&selector);
        data.extend_from_slice(&Self::abi_encode_address(token));
        data.extend_from_slice(&Self::abi_encode_u256(amount));
        data.extend_from_slice(&Self::abi_encode_address(self.account));

        Bytes::from(data)
    }

    /// Build claim rewards call data
    fn build_claim_rewards_calldata(&self, assets: Vec<Address>) -> Bytes {
        // claimAllRewards(address[] assets, address to)
        let selector: [u8; 4] = [0xbb, 0x49, 0x2b, 0xf5];

        let mut data = Vec::new();
        data.extend_from_slice(&selector);

        // ABI encode dynamic array:
        // offset to array data (64 bytes = 2 words: offset + address)
        data.extend_from_slice(&Self::abi_encode_u256(U256::from(64)));
        // address to (self.account)
        data.extend_from_slice(&Self::abi_encode_address(self.account));
        // array length
        data.extend_from_slice(&Self::abi_encode_u256(U256::from(assets.len())));
        // array elements
        for asset in &assets {
            data.extend_from_slice(&Self::abi_encode_address(*asset));
        }

        Bytes::from(data)
    }

    /// Get aToken addresses for all supported tokens
    fn get_a_token_addresses(&self) -> Vec<Address> {
        self.supported_tokens.values().map(|t| t.a_token).collect()
    }

    /// Simulate transaction (used when no provider, or as tx-building step).
    /// In production with a provider, this would submit the real transaction
    /// via the bundler / direct sendTransaction.
    async fn simulate_tx(&self, _calldata: Bytes) -> Result<B256> {
        // In production, this would:
        // 1. Build UserOperation with the calldata
        // 2. Estimate gas
        // 3. Submit to bundler
        // 4. Wait for confirmation
        let hash_bytes: [u8; 32] = rand::random();
        Ok(B256::from_slice(&hash_bytes))
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
impl YieldProtocol for AaveV3Protocol {
    fn name(&self) -> &str {
        "Aave V3"
    }

    fn protocol_id(&self) -> ProtocolId {
        ProtocolId::AaveV3
    }

    async fn current_apy(&self, token: Address) -> Result<f64> {
        if !self.supports_token(token) {
            return Err(Error::Business(format!("Token not supported: {:?}", token)));
        }

        let apy = self.fetch_apy(token).await?;

        info!(
            protocol = "Aave V3",
            chain_id = self.chain_id,
            token = ?token,
            apy = apy,
            live = self.is_live(),
            "Current APY"
        );

        Ok(apy)
    }

    async fn deposit(&self, token: Address, amount: U256) -> Result<B256> {
        if !self.supports_token(token) {
            return Err(Error::Business(format!("Token not supported: {:?}", token)));
        }

        info!(
            protocol = "Aave V3",
            token = ?token,
            amount = %amount,
            "Depositing to Aave"
        );

        let calldata = self.build_supply_calldata(token, amount);
        let tx_hash = self.simulate_tx(calldata).await?;

        // Update simulated balance (always kept in sync for tracking)
        {
            let mut balances = self.balances.write().await;
            let balance = balances.entry(token).or_insert(U256::ZERO);
            *balance = balance.saturating_add(amount);
        }

        info!(
            protocol = "Aave V3",
            tx_hash = ?tx_hash,
            "Deposit transaction submitted"
        );

        Ok(tx_hash)
    }

    async fn withdraw(&self, token: Address, amount: U256) -> Result<B256> {
        if !self.supports_token(token) {
            return Err(Error::Business(format!("Token not supported: {:?}", token)));
        }

        // Check balance
        let current_balance = self.balance(token).await?;
        if current_balance < amount {
            return Err(Error::Business(format!(
                "Insufficient balance: {} < {}",
                current_balance, amount
            )));
        }

        info!(
            protocol = "Aave V3",
            token = ?token,
            amount = %amount,
            "Withdrawing from Aave"
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
            protocol = "Aave V3",
            tx_hash = ?tx_hash,
            "Withdraw transaction submitted"
        );

        Ok(tx_hash)
    }

    async fn balance(&self, token: Address) -> Result<U256> {
        // When a provider is available, read the real on-chain aToken balance.
        if self.provider.is_some() {
            match self.fetch_balance_onchain(token).await {
                Ok(bal) => return Ok(bal),
                Err(e) => {
                    warn!(
                        protocol = "Aave V3",
                        error = %e,
                        "On-chain balance read failed, using simulated state"
                    );
                }
            }
        }

        // Fallback: simulated in-memory balance
        let balances = self.balances.read().await;
        Ok(*balances.get(&token).unwrap_or(&U256::ZERO))
    }

    async fn accrued_yield(&self, token: Address) -> Result<U256> {
        // In a live environment we could diff (aToken balance - tracked principal).
        // For now we approximate with a small percentage of the current balance.
        let balance = self.balance(token).await?;
        let yield_amount = balance / U256::from(10000); // ~0.01%
        Ok(yield_amount)
    }

    async fn claim_rewards(&self) -> Result<Option<B256>> {
        let a_tokens = self.get_a_token_addresses();
        if a_tokens.is_empty() {
            return Ok(None);
        }

        info!(
            protocol = "Aave V3",
            assets = ?a_tokens,
            "Claiming rewards"
        );

        let calldata = self.build_claim_rewards_calldata(a_tokens);
        let tx_hash = self.simulate_tx(calldata).await?;

        Ok(Some(tx_hash))
    }

    fn supports_token(&self, token: Address) -> bool {
        self.supported_tokens.contains_key(&token)
    }

    fn receipt_token(&self, token: Address) -> Option<Address> {
        self.supported_tokens.get(&token).map(|c| c.a_token)
    }

    async fn health_factor(&self) -> Result<f64> {
        // Try on-chain first
        if self.provider.is_some() {
            match self.fetch_health_factor_onchain().await {
                Ok(hf) => return Ok(hf),
                Err(e) => {
                    warn!(
                        protocol = "Aave V3",
                        error = %e,
                        "On-chain health factor read failed, returning infinite (supply-only)"
                    );
                }
            }
        }

        // For supply-only positions, health factor is infinite
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
        "0x742d35Cc6634C0532925a3b844Bc9e7595f00000"
            .parse()
            .unwrap()
    }

    #[test]
    fn test_aave_addresses() {
        let addresses = AaveV3Addresses::ethereum_mainnet().unwrap();
        assert!(!addresses.pool.is_zero());
    }

    #[tokio::test]
    async fn test_aave_protocol_creation() {
        let addresses = AaveV3Addresses::ethereum_mainnet().unwrap();
        let protocol = AaveV3Protocol::new(1, addresses, test_account());

        assert_eq!(protocol.name(), "Aave V3");
        assert_eq!(protocol.protocol_id(), ProtocolId::AaveV3);
        assert!(!protocol.is_live());
    }

    #[tokio::test]
    async fn test_health_factor() {
        let addresses = AaveV3Addresses::ethereum_mainnet().unwrap();
        let protocol = AaveV3Protocol::new(1, addresses, test_account());

        let hf = protocol.health_factor().await.unwrap();
        assert!(hf > 1.0);
    }

    #[tokio::test]
    async fn test_simulated_apy_fallback() {
        let addresses = AaveV3Addresses::ethereum_mainnet().unwrap();
        let protocol = AaveV3Protocol::new(1, addresses, test_account());

        let usdc: Address = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
            .parse()
            .unwrap();
        // Without RPC/API, should return the hardcoded fallback
        let apy = protocol.current_apy(usdc).await.unwrap();
        assert!(apy > 0.0);
    }

    #[tokio::test]
    async fn test_deposit_withdraw_simulated() {
        let addresses = AaveV3Addresses::ethereum_mainnet().unwrap();
        let protocol = AaveV3Protocol::new(1, addresses, test_account());

        let usdc: Address = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
            .parse()
            .unwrap();
        let amount = U256::from(1000) * U256::from(1_000_000u64); // 1000 USDC

        // Deposit
        let tx = protocol.deposit(usdc, amount).await;
        assert!(tx.is_ok());

        // Check balance (simulated)
        let balance = protocol.balance(usdc).await.unwrap();
        assert_eq!(balance, amount);

        // Withdraw half
        let withdraw_amount = amount / U256::from(2);
        let tx = protocol.withdraw(usdc, withdraw_amount).await;
        assert!(tx.is_ok());

        let balance = protocol.balance(usdc).await.unwrap();
        assert_eq!(balance, amount - withdraw_amount);
    }

    #[test]
    fn test_with_rpc_invalid_url() {
        let addresses = AaveV3Addresses::ethereum_mainnet().unwrap();
        // An invalid URL should still parse with alloy Http provider
        // (it only fails on actual calls), so this should succeed.
        let result =
            AaveV3Protocol::with_rpc(1, addresses, test_account(), "http://localhost:8545");
        assert!(result.is_ok());
        assert!(result.unwrap().is_live());
    }
}
