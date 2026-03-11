//! USDC (Circle) Stablecoin Implementation
//!
//! USD Coin is a fully reserved stablecoin issued by Circle.
//! Uses 6 decimals on all chains.

use alloy::primitives::{Address, U256};
use async_trait::async_trait;
use ramp_common::{Error, Result};
use std::collections::HashMap;

use super::{Stablecoin, StablecoinMetadata, TxHash};

/// USDC Token implementation
pub struct UsdcToken {
    /// Chain ID -> Contract Address mapping
    contracts: HashMap<u64, Address>,
}

impl Default for UsdcToken {
    fn default() -> Self {
        Self::new()
    }
}

impl UsdcToken {
    /// Create a new USDC token with known contract addresses
    pub fn new() -> Self {
        let mut contracts = HashMap::new();

        // Ethereum Mainnet (Native USDC)
        if let Ok(addr) = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".parse() {
            contracts.insert(1, addr);
        }

        // Polygon (Native USDC)
        if let Ok(addr) = "0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359".parse() {
            contracts.insert(137, addr);
        }

        // BNB Chain
        if let Ok(addr) = "0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d".parse() {
            contracts.insert(56, addr);
        }

        // Arbitrum One (Native USDC)
        if let Ok(addr) = "0xaf88d065e77c8cC2239327C5EDb3A432268e5831".parse() {
            contracts.insert(42161, addr);
        }

        // Optimism (Native USDC)
        if let Ok(addr) = "0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85".parse() {
            contracts.insert(10, addr);
        }

        // Avalanche C-Chain (Native USDC)
        if let Ok(addr) = "0xB97EF9Ef8734C71904D8002F8b6Bc66Dd9c48a6E".parse() {
            contracts.insert(43114, addr);
        }

        // Base (Native USDC)
        if let Ok(addr) = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".parse() {
            contracts.insert(8453, addr);
        }

        Self { contracts }
    }

    /// Add a custom contract address for a chain
    pub fn with_contract(mut self, chain_id: u64, address: Address) -> Self {
        self.contracts.insert(chain_id, address);
        self
    }
}

#[async_trait]
impl Stablecoin for UsdcToken {
    fn symbol(&self) -> &str {
        "USDC"
    }

    fn name(&self) -> &str {
        "USD Coin"
    }

    fn decimals(&self) -> u8 {
        6
    }

    fn metadata(&self) -> StablecoinMetadata {
        StablecoinMetadata {
            symbol: "USDC".to_string(),
            name: "USD Coin".to_string(),
            decimals: 6,
            logo_url: Some(
                "https://assets.coingecko.com/coins/images/6319/large/USD_Coin_icon.png"
                    .to_string(),
            ),
            website: Some("https://www.circle.com/usdc".to_string()),
            description: Some("USDC is a fully reserved stablecoin issued by Circle".to_string()),
        }
    }

    fn contract_address(&self, chain_id: u64) -> Option<Address> {
        self.contracts.get(&chain_id).copied()
    }

    fn supported_chains(&self) -> Vec<u64> {
        self.contracts.keys().copied().collect()
    }

    async fn balance_of(&self, chain_id: u64, address: Address) -> Result<U256> {
        let _contract = self.contract_address(chain_id).ok_or_else(|| {
            Error::Validation(format!("USDC not supported on chain {}", chain_id))
        })?;

        tracing::debug!(
            chain_id = chain_id,
            address = %address,
            "Fetching USDC balance (mock)"
        );

        Ok(U256::ZERO)
    }

    async fn transfer(
        &self,
        chain_id: u64,
        from: Address,
        to: Address,
        amount: U256,
    ) -> Result<TxHash> {
        let contract = self.contract_address(chain_id).ok_or_else(|| {
            Error::Validation(format!("USDC not supported on chain {}", chain_id))
        })?;

        tracing::info!(
            chain_id = chain_id,
            contract = %contract,
            from = %from,
            to = %to,
            amount = %amount,
            "Initiating USDC transfer"
        );

        Err(Error::Validation(
            "USDC transfer not implemented - use SmartAccountService".to_string(),
        ))
    }

    async fn allowance(&self, chain_id: u64, owner: Address, spender: Address) -> Result<U256> {
        let _contract = self.contract_address(chain_id).ok_or_else(|| {
            Error::Validation(format!("USDC not supported on chain {}", chain_id))
        })?;

        tracing::debug!(
            chain_id = chain_id,
            owner = %owner,
            spender = %spender,
            "Checking USDC allowance (mock)"
        );

        Ok(U256::ZERO)
    }

    async fn approve(
        &self,
        chain_id: u64,
        owner: Address,
        spender: Address,
        amount: U256,
    ) -> Result<TxHash> {
        let contract = self.contract_address(chain_id).ok_or_else(|| {
            Error::Validation(format!("USDC not supported on chain {}", chain_id))
        })?;

        tracing::info!(
            chain_id = chain_id,
            contract = %contract,
            owner = %owner,
            spender = %spender,
            amount = %amount,
            "Approving USDC spending"
        );

        Err(Error::Validation(
            "USDC approve not implemented - use SmartAccountService".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usdc_metadata() {
        let usdc = UsdcToken::new();

        assert_eq!(usdc.symbol(), "USDC");
        assert_eq!(usdc.name(), "USD Coin");
        assert_eq!(usdc.decimals(), 6);
    }

    #[test]
    fn test_usdc_chain_support() {
        let usdc = UsdcToken::new();

        assert!(usdc.is_supported_on_chain(1)); // Ethereum
        assert!(usdc.is_supported_on_chain(137)); // Polygon
        assert!(usdc.is_supported_on_chain(42161)); // Arbitrum
        assert!(usdc.is_supported_on_chain(8453)); // Base
        assert!(!usdc.is_supported_on_chain(999)); // Unknown chain
    }

    #[test]
    fn test_usdc_contract_addresses() {
        let usdc = UsdcToken::new();

        // Ethereum mainnet USDC
        let eth_addr = usdc.contract_address(1).unwrap();
        assert_eq!(
            format!("{:?}", eth_addr).to_lowercase(),
            "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"
        );
    }
}
