//! Compound V3 (Comet) Protocol Integration
//!
//! Implements yield protocol for Compound V3 markets.
//! Supports supply/withdraw of stablecoins and COMP reward claiming.

use async_trait::async_trait;
use ethers::abi::{encode, Token};
use ethers::types::{Address, Bytes, H256, U256};
use ramp_common::Result;
use std::collections::HashMap;
use std::sync::RwLock;
use tracing::info;

use super::{ProtocolId, YieldProtocol};

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
                .map_err(|e| anyhow::anyhow!("Invalid comet address: {}", e))?,
            comp_token: "0xc00e94Cb662C3520282E6f5717214004A7f26888".parse()
                .map_err(|e| anyhow::anyhow!("Invalid COMP address: {}", e))?,
            rewards: "0x1B0e765F6224C21223AeA2af16c1C46E38885a40".parse()
                .map_err(|e| anyhow::anyhow!("Invalid rewards address: {}", e))?,
        })
    }

    pub fn polygon_mainnet() -> Result<Self> {
        Ok(Self {
            comet_usdc: "0xF25212E676D1F7F89Cd72fFEe66158f541246445".parse()
                .map_err(|e| anyhow::anyhow!("Invalid comet address: {}", e))?,
            comp_token: "0x8505b9d2254A7Ae468c0E9dd10Ccea3A837aef5c".parse()
                .map_err(|e| anyhow::anyhow!("Invalid COMP address: {}", e))?,
            rewards: "0x45939657d1CA34A8FA39A924B71D28Fe8431e581".parse()
                .map_err(|e| anyhow::anyhow!("Invalid rewards address: {}", e))?,
        })
    }

    pub fn arbitrum() -> Result<Self> {
        Ok(Self {
            comet_usdc: "0xA5EDBDD9646f8dFF606d7448e414884C7d905dCA".parse()
                .map_err(|e| anyhow::anyhow!("Invalid comet address: {}", e))?,
            comp_token: "0x354A6dA3fcde098F8389cad84b0182725c6C91dE".parse()
                .map_err(|e| anyhow::anyhow!("Invalid COMP address: {}", e))?,
            rewards: "0x88730d254A2f7e6AC8388c3198aFd694bA9f7fae".parse()
                .map_err(|e| anyhow::anyhow!("Invalid rewards address: {}", e))?,
        })
    }
}

/// Token configuration for Compound V3
#[derive(Debug, Clone)]
pub struct CompoundTokenConfig {
    pub underlying: Address,
    pub comet: Address,
    pub decimals: u8,
}

/// Compound V3 Protocol implementation
pub struct CompoundV3Protocol {
    chain_id: u64,
    addresses: CompoundV3Addresses,
    account: Address,
    supported_tokens: HashMap<Address, CompoundTokenConfig>,
    // Simulated state
    balances: RwLock<HashMap<Address, U256>>,
}

impl CompoundV3Protocol {
    pub fn new(chain_id: u64, addresses: CompoundV3Addresses, account: Address) -> Self {
        Self {
            chain_id,
            addresses: addresses.clone(),
            account,
            supported_tokens: Self::default_tokens(chain_id, &addresses),
            balances: RwLock::new(HashMap::new()),
        }
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

    /// Build supply call data for Comet
    fn build_supply_calldata(&self, token: Address, amount: U256) -> Bytes {
        // supply(address asset, uint amount)
        let selector: [u8; 4] = [0xf2, 0xb9, 0xfa, 0xdb]; // keccak256("supply(address,uint256)")[:4]

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
        let selector: [u8; 4] = [0xf3, 0xfef, 0x3a, 0x3a]; // keccak256("withdraw(address,uint256)")[:4]

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
        let selector: [u8; 4] = [0xb8, 0x8c, 0x91, 0x48]; // keccak256("claim(address,address,bool)")[:4]

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
}

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
            return Err(anyhow::anyhow!("Token not supported: {:?}", token));
        }

        // In production, would query getSupplyRate from Comet
        // APR = supplyRate * seconds per year / 1e18
        // For now, return simulated APY
        let apy = 5.2; // Compound typically has slightly higher APY for suppliers

        info!(
            protocol = "Compound V3",
            chain_id = self.chain_id,
            token = ?token,
            apy = apy,
            "Fetched current APY"
        );

        Ok(apy)
    }

    async fn deposit(&self, token: Address, amount: U256) -> Result<H256> {
        if !self.supports_token(token) {
            return Err(anyhow::anyhow!("Token not supported: {:?}", token));
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
            let mut balances = self.balances.write().map_err(|_| anyhow::anyhow!("Lock poisoned"))?;
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
            return Err(anyhow::anyhow!("Token not supported: {:?}", token));
        }

        let current_balance = self.balance(token).await?;
        if current_balance < amount {
            return Err(anyhow::anyhow!(
                "Insufficient balance: {} < {}",
                current_balance,
                amount
            ));
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
            let mut balances = self.balances.write().map_err(|_| anyhow::anyhow!("Lock poisoned"))?;
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
        let balances = self.balances.read().map_err(|_| anyhow::anyhow!("Lock poisoned"))?;
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
        // In production, would check if account is liquidatable
        // For supply-only, there's no liquidation risk
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
}
