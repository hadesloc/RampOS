//! VNST (Vietnam Stablecoin) Implementation
//!
//! VNST is a stablecoin pegged to the Vietnamese Dong (VND).
//! Primarily operates on BNB Chain and Ethereum.
//! Uses 18 decimals.

use alloy::primitives::{Address, U256};
use async_trait::async_trait;
use ramp_common::{Error, Result};
use std::collections::HashMap;

use super::{Stablecoin, StablecoinMetadata, TxHash};

/// VNST Token implementation
pub struct VnstToken {
    /// Chain ID -> Contract Address mapping
    contracts: HashMap<u64, Address>,
}

impl Default for VnstToken {
    fn default() -> Self {
        Self::new()
    }
}

impl VnstToken {
    /// Create a new VNST token with known contract addresses
    pub fn new() -> Self {
        let mut contracts = HashMap::new();

        // BNB Chain (Primary network for VNST)
        // TESTNET: BSC Testnet VNST contract - replace with mainnet address before production
        if let Ok(addr) = "0x2C1e9e7e41A5ba5A67D4e4E07CE0A7bC9b49b1f0".parse::<Address>() {
            contracts.insert(56, addr);
        }

        // Ethereum Mainnet
        // TESTNET: Goerli VNST contract - replace with mainnet address before production
        if let Ok(addr) = "0x3A4cB5E08E76e6A1D4e7F2Cc6dB5B29cF3E9a8D1".parse::<Address>() {
            contracts.insert(1, addr);
        }

        // Polygon
        // TESTNET: Mumbai VNST contract - replace with mainnet address before production
        if let Ok(addr) = "0x5D8f4A2bC9e7F1d3E6a0B8c4D2f9E7A1b3C5d7F9".parse::<Address>() {
            contracts.insert(137, addr);
        }

        Self { contracts }
    }

    /// Create VNST with specific contract addresses
    /// Use this for production deployments with real addresses
    pub fn with_contracts(contracts: HashMap<u64, Address>) -> Self {
        Self { contracts }
    }

    /// Add a custom contract address for a chain
    pub fn with_contract(mut self, chain_id: u64, address: Address) -> Self {
        self.contracts.insert(chain_id, address);
        self
    }

    /// Convert VND amount to VNST base units (18 decimals)
    /// 1 VND = 1 VNST, but VNST uses 18 decimals
    pub fn vnd_to_base_units(vnd_amount: u64) -> U256 {
        U256::from(vnd_amount) * U256::from(10u64).pow(U256::from(18u64))
    }

    /// Convert VNST base units to VND amount
    pub fn base_units_to_vnd(base_units: U256) -> u64 {
        let divisor = U256::from(10u64).pow(U256::from(18u64));
        let val: u64 = (base_units / divisor).try_into().unwrap_or(0u64);
        val
    }
}

#[async_trait]
impl Stablecoin for VnstToken {
    fn symbol(&self) -> &str {
        "VNST"
    }

    fn name(&self) -> &str {
        "Vietnam Stablecoin"
    }

    fn decimals(&self) -> u8 {
        18
    }

    fn metadata(&self) -> StablecoinMetadata {
        StablecoinMetadata {
            symbol: "VNST".to_string(),
            name: "Vietnam Stablecoin".to_string(),
            decimals: 18,
            logo_url: Some("https://vnst.io/logo.png".to_string()),
            website: Some("https://vnst.io".to_string()),
            description: Some(
                "VNST is a stablecoin pegged 1:1 to the Vietnamese Dong (VND)".to_string(),
            ),
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
            Error::Validation(format!("VNST not supported on chain {}", chain_id))
        })?;

        tracing::debug!(
            chain_id = chain_id,
            address = %address,
            "Fetching VNST balance (mock)"
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
            Error::Validation(format!("VNST not supported on chain {}", chain_id))
        })?;

        tracing::info!(
            chain_id = chain_id,
            contract = %contract,
            from = %from,
            to = %to,
            amount = %amount,
            "Initiating VNST transfer"
        );

        Err(Error::Validation(
            "VNST transfer not implemented - use SmartAccountService".to_string(),
        ))
    }

    async fn allowance(&self, chain_id: u64, owner: Address, spender: Address) -> Result<U256> {
        let _contract = self.contract_address(chain_id).ok_or_else(|| {
            Error::Validation(format!("VNST not supported on chain {}", chain_id))
        })?;

        tracing::debug!(
            chain_id = chain_id,
            owner = %owner,
            spender = %spender,
            "Checking VNST allowance (mock)"
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
            Error::Validation(format!("VNST not supported on chain {}", chain_id))
        })?;

        tracing::info!(
            chain_id = chain_id,
            contract = %contract,
            owner = %owner,
            spender = %spender,
            amount = %amount,
            "Approving VNST spending"
        );

        Err(Error::Validation(
            "VNST approve not implemented - use SmartAccountService".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vnst_metadata() {
        let vnst = VnstToken::new();

        assert_eq!(vnst.symbol(), "VNST");
        assert_eq!(vnst.name(), "Vietnam Stablecoin");
        assert_eq!(vnst.decimals(), 18);
    }

    #[test]
    fn test_vnst_chain_support() {
        let vnst = VnstToken::new();

        // VNST primarily on BSC
        assert!(vnst.is_supported_on_chain(56)); // BSC
        assert!(!vnst.is_supported_on_chain(999)); // Unknown chain
    }

    #[test]
    fn test_vnd_conversion() {
        // 1,000,000 VND
        let vnd_amount = 1_000_000u64;
        let base_units = VnstToken::vnd_to_base_units(vnd_amount);

        // Should be 1,000,000 * 10^18
        let expected = U256::from(1_000_000u64) * U256::from(10u64).pow(U256::from(18u64));
        assert_eq!(base_units, expected);

        // Convert back
        let back_to_vnd = VnstToken::base_units_to_vnd(base_units);
        assert_eq!(back_to_vnd, vnd_amount);
    }

    #[test]
    fn test_custom_contracts() {
        let custom_addr: Address = "0x1234567890123456789012345678901234567890"
            .parse()
            .unwrap();

        let mut contracts = HashMap::new();
        contracts.insert(1337, custom_addr);

        let vnst = VnstToken::with_contracts(contracts);
        assert!(vnst.is_supported_on_chain(1337));
        assert_eq!(vnst.contract_address(1337), Some(custom_addr));
    }

    #[test]
    fn test_vnst_for_vietnam_market() {
        let vnst = VnstToken::new();
        let metadata = vnst.metadata();

        // VNST is specifically designed for Vietnam market
        // Clone the description so it's not consumed
        assert!(metadata
            .description
            .clone()
            .unwrap()
            .contains("Vietnamese Dong"));
        assert!(metadata.description.is_some());
    }
}
