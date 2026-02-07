//! Aave V3 Protocol Integration
//!
//! Implements yield protocol for Aave V3 lending pool.
//! Supports supply/withdraw of stablecoins and reward claiming.

use async_trait::async_trait;
use ethers::abi::{encode, Token};
use ethers::types::{Address, Bytes, H256, U256};
use ramp_common::{Error, Result};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::info;

use super::{ProtocolId, YieldProtocol};

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
            pool: "0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2".parse()
                .map_err(|e| Error::Internal(format!("Invalid pool address: {}", e)))?,
            pool_data_provider: "0x7B4EB56E7CD4b454BA8ff71E4518426369a138a3".parse()
                .map_err(|e| Error::Internal(format!("Invalid data provider address: {}", e)))?,
            incentives_controller: "0x8164Cc65827dcFe994AB23944CBC90e0aa80bFcb".parse()
                .map_err(|e| Error::Internal(format!("Invalid incentives address: {}", e)))?,
        })
    }

    pub fn polygon_mainnet() -> Result<Self> {
        Ok(Self {
            pool: "0x794a61358D6845594F94dc1DB02A252b5b4814aD".parse()
                .map_err(|e| Error::Internal(format!("Invalid pool address: {}", e)))?,
            pool_data_provider: "0x69FA688f1Dc47d4B5d8029D5a35FB7a548310654".parse()
                .map_err(|e| Error::Internal(format!("Invalid data provider address: {}", e)))?,
            incentives_controller: "0x929EC64c34a17401F460460D4B9390518E5B473e".parse()
                .map_err(|e| Error::Internal(format!("Invalid incentives address: {}", e)))?,
        })
    }

    pub fn arbitrum() -> Result<Self> {
        Ok(Self {
            pool: "0x794a61358D6845594F94dc1DB02A252b5b4814aD".parse()
                .map_err(|e| Error::Internal(format!("Invalid pool address: {}", e)))?,
            pool_data_provider: "0x69FA688f1Dc47d4B5d8029D5a35FB7a548310654".parse()
                .map_err(|e| Error::Internal(format!("Invalid data provider address: {}", e)))?,
            incentives_controller: "0x929EC64c34a17401F460460D4B9390518E5B473e".parse()
                .map_err(|e| Error::Internal(format!("Invalid incentives address: {}", e)))?,
        })
    }
}

/// Token configuration for Aave
#[derive(Debug, Clone)]
pub struct AaveTokenConfig {
    pub underlying: Address,
    pub a_token: Address,
    pub decimals: u8,
}

/// Aave V3 Protocol implementation
#[allow(dead_code)]
pub struct AaveV3Protocol {
    chain_id: u64,
    addresses: AaveV3Addresses,
    account: Address,
    supported_tokens: HashMap<Address, AaveTokenConfig>,
    // In production, this would use actual RPC calls
    // For now, using simulated state
    balances: RwLock<HashMap<Address, U256>>,
}

impl AaveV3Protocol {
    pub fn new(chain_id: u64, addresses: AaveV3Addresses, account: Address) -> Self {
        Self {
            chain_id,
            addresses,
            account,
            supported_tokens: Self::default_tokens(chain_id),
            balances: RwLock::new(HashMap::new()),
        }
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
                    tokens.insert(underlying, AaveTokenConfig {
                        underlying,
                        a_token,
                        decimals: 6,
                    });
                }
                // USDT
                if let (Ok(underlying), Ok(a_token)) = (
                    "0xdAC17F958D2ee523a2206206994597C13D831ec7".parse::<Address>(),
                    "0x23878914EFE38d27C4D67Ab83ed1b93A74D4086a".parse::<Address>(),
                ) {
                    tokens.insert(underlying, AaveTokenConfig {
                        underlying,
                        a_token,
                        decimals: 6,
                    });
                }
                // DAI
                if let (Ok(underlying), Ok(a_token)) = (
                    "0x6B175474E89094C44Da98b954EedeAC495271d0F".parse::<Address>(),
                    "0x018008bfb33d285247A21d44E50697654f754e63".parse::<Address>(),
                ) {
                    tokens.insert(underlying, AaveTokenConfig {
                        underlying,
                        a_token,
                        decimals: 18,
                    });
                }
            }
            137 => {
                // Polygon USDC
                if let (Ok(underlying), Ok(a_token)) = (
                    "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174".parse::<Address>(),
                    "0x625E7708f30cA75bfd92586e17077590C60eb4cD".parse::<Address>(),
                ) {
                    tokens.insert(underlying, AaveTokenConfig {
                        underlying,
                        a_token,
                        decimals: 6,
                    });
                }
            }
            _ => {}
        }

        tokens
    }

    /// Build supply call data for Aave Pool
    fn build_supply_calldata(&self, token: Address, amount: U256) -> Bytes {
        // supply(address asset, uint256 amount, address onBehalfOf, uint16 referralCode)
        let selector: [u8; 4] = [0x61, 0x7b, 0xa0, 0x37]; // keccak256("supply(address,uint256,address,uint16)")[:4]

        let mut data = Vec::new();
        data.extend_from_slice(&selector);

        let params = encode(&[
            Token::Address(token),
            Token::Uint(amount),
            Token::Address(self.account),
            Token::Uint(U256::zero()), // referral code
        ]);
        data.extend_from_slice(&params);

        Bytes::from(data)
    }

    /// Build withdraw call data for Aave Pool
    fn build_withdraw_calldata(&self, token: Address, amount: U256) -> Bytes {
        // withdraw(address asset, uint256 amount, address to)
        let selector: [u8; 4] = [0x69, 0x32, 0x8d, 0xec]; // keccak256("withdraw(address,uint256,address)")[:4]

        let mut data = Vec::new();
        data.extend_from_slice(&selector);

        let params = encode(&[
            Token::Address(token),
            Token::Uint(amount),
            Token::Address(self.account),
        ]);
        data.extend_from_slice(&params);

        Bytes::from(data)
    }

    /// Build claim rewards call data
    fn build_claim_rewards_calldata(&self, assets: Vec<Address>) -> Bytes {
        // claimAllRewards(address[] assets, address to)
        let selector: [u8; 4] = [0xbb, 0x49, 0x2b, 0xf5]; // keccak256("claimAllRewards(address[],address)")[:4]

        let mut data = Vec::new();
        data.extend_from_slice(&selector);

        let asset_tokens: Vec<Token> = assets.iter().map(|a| Token::Address(*a)).collect();
        let params = encode(&[
            Token::Array(asset_tokens),
            Token::Address(self.account),
        ]);
        data.extend_from_slice(&params);

        Bytes::from(data)
    }

    /// Get aToken addresses for all supported tokens
    fn get_a_token_addresses(&self) -> Vec<Address> {
        self.supported_tokens.values().map(|t| t.a_token).collect()
    }

    /// Simulate transaction (in production, would submit via bundler)
    async fn simulate_tx(&self, _calldata: Bytes) -> Result<H256> {
        // In production, this would:
        // 1. Build UserOperation with the calldata
        // 2. Estimate gas
        // 3. Submit to bundler
        // 4. Wait for confirmation

        // For now, return a simulated tx hash
        let hash_bytes: [u8; 32] = rand::random();
        Ok(H256::from_slice(&hash_bytes))
    }
}

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

        // In production, would query Aave's getReserveData
        // For now, return simulated APY based on token
        let config = self.supported_tokens.get(&token);
        let apy = match config.map(|c| c.decimals) {
            Some(6) => 4.5,  // USDC/USDT typical range
            Some(18) => 3.8, // DAI typical range
            _ => 4.0,
        };

        info!(
            protocol = "Aave V3",
            chain_id = self.chain_id,
            token = ?token,
            apy = apy,
            "Fetched current APY"
        );

        Ok(apy)
    }

    async fn deposit(&self, token: Address, amount: U256) -> Result<H256> {
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

        // Update simulated balance
        {
            let mut balances = self.balances.write().await;
            let balance = balances.entry(token).or_insert(U256::zero());
            *balance = balance.saturating_add(amount);
        }

        info!(
            protocol = "Aave V3",
            tx_hash = ?tx_hash,
            "Deposit transaction submitted"
        );

        Ok(tx_hash)
    }

    async fn withdraw(&self, token: Address, amount: U256) -> Result<H256> {
        if !self.supports_token(token) {
            return Err(Error::Business(format!("Token not supported: {:?}", token)));
        }

        // Check balance
        let current_balance = self.balance(token).await?;
        if current_balance < amount {
            return Err(Error::Business(format!(
                "Insufficient balance: {} < {}",
                current_balance,
                amount
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
        let balances = self.balances.read().await;
        Ok(*balances.get(&token).unwrap_or(&U256::zero()))
    }

    async fn accrued_yield(&self, token: Address) -> Result<U256> {
        // In production, would calculate (current aToken balance - principal)
        let balance = self.balance(token).await?;
        // Simulate ~0.01% yield accrued
        let yield_amount = balance / U256::from(10000);
        Ok(yield_amount)
    }

    async fn claim_rewards(&self) -> Result<Option<H256>> {
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
        // In production, would query getUserAccountData from Aave Pool
        // Returns health factor (> 1.0 is safe, liquidation at 1.0)
        // For supply-only positions, health factor is infinite
        Ok(f64::INFINITY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_account() -> Address {
        "0x742d35Cc6634C0532925a3b844Bc9e7595f00000".parse().unwrap()
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
    }

    #[tokio::test]
    async fn test_health_factor() {
        let addresses = AaveV3Addresses::ethereum_mainnet().unwrap();
        let protocol = AaveV3Protocol::new(1, addresses, test_account());

        let hf = protocol.health_factor().await.unwrap();
        assert!(hf > 1.0);
    }
}
