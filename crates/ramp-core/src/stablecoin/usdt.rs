//! USDT (Tether) Stablecoin Implementation
//!
//! Tether is the largest stablecoin by market cap, available on multiple chains.
//! Uses 6 decimals on most chains.

use async_trait::async_trait;
use ethers::types::{Address, U256};
use ramp_common::{Error, Result};
use std::collections::HashMap;

use super::{Stablecoin, StablecoinMetadata, TxHash};

/// USDT Token implementation
pub struct UsdtToken {
    /// Chain ID -> Contract Address mapping
    contracts: HashMap<u64, Address>,
}

impl Default for UsdtToken {
    fn default() -> Self {
        Self::new()
    }
}

impl UsdtToken {
    /// Create a new USDT token with known contract addresses
    pub fn new() -> Self {
        let mut contracts = HashMap::new();

        // Ethereum Mainnet
        if let Ok(addr) = "0xdAC17F958D2ee523a2206206994597C13D831ec7".parse() {
            contracts.insert(1, addr);
        }

        // Polygon
        if let Ok(addr) = "0xc2132D05D31c914a87C6611C10748AEb04B58e8F".parse() {
            contracts.insert(137, addr);
        }

        // BNB Chain
        if let Ok(addr) = "0x55d398326f99059fF775485246999027B3197955".parse() {
            contracts.insert(56, addr);
        }

        // Arbitrum One
        if let Ok(addr) = "0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9".parse() {
            contracts.insert(42161, addr);
        }

        // Optimism
        if let Ok(addr) = "0x94b008aA00579c1307B0EF2c499aD98a8ce58e58".parse() {
            contracts.insert(10, addr);
        }

        // Avalanche C-Chain
        if let Ok(addr) = "0x9702230A8Ea53601f5cD2dc00fDBc13d4dF4A8c7".parse() {
            contracts.insert(43114, addr);
        }

        // Base
        if let Ok(addr) = "0xfde4C96c8593536E31F229EA8f37b2ADa2699bb2".parse() {
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
impl Stablecoin for UsdtToken {
    fn symbol(&self) -> &str {
        "USDT"
    }

    fn name(&self) -> &str {
        "Tether USD"
    }

    fn decimals(&self) -> u8 {
        6
    }

    fn metadata(&self) -> StablecoinMetadata {
        StablecoinMetadata {
            symbol: "USDT".to_string(),
            name: "Tether USD".to_string(),
            decimals: 6,
            logo_url: Some("https://assets.coingecko.com/coins/images/325/large/Tether.png".to_string()),
            website: Some("https://tether.to".to_string()),
            description: Some("Tether (USDT) is a stablecoin pegged to the US Dollar".to_string()),
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
            Error::Validation(format!("USDT not supported on chain {}", chain_id))
        })?;

        // In production, this would call the ERC-20 balanceOf function
        // For now, return a placeholder
        tracing::debug!(
            chain_id = chain_id,
            address = %address,
            "Fetching USDT balance (mock)"
        );

        // Mock: Return 0 balance
        Ok(U256::zero())
    }

    async fn transfer(
        &self,
        chain_id: u64,
        from: Address,
        to: Address,
        amount: U256,
    ) -> Result<TxHash> {
        let contract = self.contract_address(chain_id).ok_or_else(|| {
            Error::Validation(format!("USDT not supported on chain {}", chain_id))
        })?;

        tracing::info!(
            chain_id = chain_id,
            contract = %contract,
            from = %from,
            to = %to,
            amount = %amount,
            "Initiating USDT transfer"
        );

        // In production, this would:
        // 1. Build ERC-20 transfer call data
        // 2. Create and sign transaction
        // 3. Submit to bundler or direct to chain
        // 4. Return actual tx hash

        // Mock: Return placeholder hash
        Err(Error::Validation(
            "USDT transfer not implemented - use SmartAccountService".to_string(),
        ))
    }

    async fn allowance(
        &self,
        chain_id: u64,
        owner: Address,
        spender: Address,
    ) -> Result<U256> {
        let _contract = self.contract_address(chain_id).ok_or_else(|| {
            Error::Validation(format!("USDT not supported on chain {}", chain_id))
        })?;

        tracing::debug!(
            chain_id = chain_id,
            owner = %owner,
            spender = %spender,
            "Checking USDT allowance (mock)"
        );

        Ok(U256::zero())
    }

    async fn approve(
        &self,
        chain_id: u64,
        owner: Address,
        spender: Address,
        amount: U256,
    ) -> Result<TxHash> {
        let contract = self.contract_address(chain_id).ok_or_else(|| {
            Error::Validation(format!("USDT not supported on chain {}", chain_id))
        })?;

        tracing::info!(
            chain_id = chain_id,
            contract = %contract,
            owner = %owner,
            spender = %spender,
            amount = %amount,
            "Approving USDT spending"
        );

        Err(Error::Validation(
            "USDT approve not implemented - use SmartAccountService".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usdt_metadata() {
        let usdt = UsdtToken::new();

        assert_eq!(usdt.symbol(), "USDT");
        assert_eq!(usdt.name(), "Tether USD");
        assert_eq!(usdt.decimals(), 6);
    }

    #[test]
    fn test_usdt_chain_support() {
        let usdt = UsdtToken::new();

        assert!(usdt.is_supported_on_chain(1)); // Ethereum
        assert!(usdt.is_supported_on_chain(137)); // Polygon
        assert!(usdt.is_supported_on_chain(56)); // BSC
        assert!(usdt.is_supported_on_chain(42161)); // Arbitrum
        assert!(!usdt.is_supported_on_chain(999)); // Unknown chain
    }

    #[test]
    fn test_usdt_contract_addresses() {
        let usdt = UsdtToken::new();

        // Ethereum mainnet USDT
        let eth_addr = usdt.contract_address(1).unwrap();
        assert_eq!(
            format!("{:?}", eth_addr).to_lowercase(),
            "0xdac17f958d2ee523a2206206994597c13d831ec7"
        );
    }

    #[test]
    fn test_custom_contract() {
        let custom_addr: Address = "0x0000000000000000000000000000000000000001".parse().unwrap();
        let usdt = UsdtToken::new().with_contract(999, custom_addr);

        assert!(usdt.is_supported_on_chain(999));
        assert_eq!(usdt.contract_address(999), Some(custom_addr));
    }
}
