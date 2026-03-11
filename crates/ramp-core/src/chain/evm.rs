//! EVM Chain Adapter
//!
//! Implementation of Chain trait for EVM-compatible chains:
//! - Ethereum Mainnet/Testnets
//! - Arbitrum
//! - Base
//! - Optimism
//! - Polygon
//! - BSC
//! - Avalanche

use alloy::primitives::{Address, Bytes, B256, U256};
use alloy::providers::Provider;
use async_trait::async_trait;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, info, warn};

use super::{
    Balance, Chain, ChainError, ChainId, ChainType, FeeEstimate, FeeOption, Result, TokenBalance,
    Transaction, TxHash, TxState, TxStatus, UnifiedAddress,
};

/// EVM Chain configuration
#[derive(Debug, Clone)]
pub struct EvmChainConfig {
    pub chain_id: ChainId,
    pub name: String,
    pub rpc_url: String,
    pub native_symbol: String,
    pub is_testnet: bool,
    pub explorer_url: String,
    /// Whether to use EIP-1559 transactions
    pub eip1559: bool,
    /// Block time in seconds (for fee estimation)
    pub block_time_secs: u64,
}

impl EvmChainConfig {
    /// Create config for Ethereum mainnet
    pub fn ethereum(rpc_url: &str) -> Self {
        Self {
            chain_id: ChainId::ETHEREUM,
            name: "Ethereum".to_string(),
            rpc_url: rpc_url.to_string(),
            native_symbol: "ETH".to_string(),
            is_testnet: false,
            explorer_url: "https://etherscan.io".to_string(),
            eip1559: true,
            block_time_secs: 12,
        }
    }

    /// Create config for Arbitrum
    pub fn arbitrum(rpc_url: &str) -> Self {
        Self {
            chain_id: ChainId::ARBITRUM,
            name: "Arbitrum One".to_string(),
            rpc_url: rpc_url.to_string(),
            native_symbol: "ETH".to_string(),
            is_testnet: false,
            explorer_url: "https://arbiscan.io".to_string(),
            eip1559: true,
            block_time_secs: 1,
        }
    }

    /// Create config for Base
    pub fn base(rpc_url: &str) -> Self {
        Self {
            chain_id: ChainId::BASE,
            name: "Base".to_string(),
            rpc_url: rpc_url.to_string(),
            native_symbol: "ETH".to_string(),
            is_testnet: false,
            explorer_url: "https://basescan.org".to_string(),
            eip1559: true,
            block_time_secs: 2,
        }
    }

    /// Create config for Optimism
    pub fn optimism(rpc_url: &str) -> Self {
        Self {
            chain_id: ChainId::OPTIMISM,
            name: "Optimism".to_string(),
            rpc_url: rpc_url.to_string(),
            native_symbol: "ETH".to_string(),
            is_testnet: false,
            explorer_url: "https://optimistic.etherscan.io".to_string(),
            eip1559: true,
            block_time_secs: 2,
        }
    }

    /// Create config for Polygon
    pub fn polygon(rpc_url: &str) -> Self {
        Self {
            chain_id: ChainId::POLYGON,
            name: "Polygon".to_string(),
            rpc_url: rpc_url.to_string(),
            native_symbol: "MATIC".to_string(),
            is_testnet: false,
            explorer_url: "https://polygonscan.com".to_string(),
            eip1559: true,
            block_time_secs: 2,
        }
    }

    /// Create config for Polygon zkEVM
    pub fn polygon_zkevm(rpc_url: &str) -> Self {
        Self {
            chain_id: ChainId::POLYGON_ZKEVM,
            name: "Polygon zkEVM".to_string(),
            rpc_url: rpc_url.to_string(),
            native_symbol: "ETH".to_string(),
            is_testnet: false,
            explorer_url: "https://zkevm.polygonscan.com".to_string(),
            eip1559: true,
            block_time_secs: 5,
        }
    }

    /// Create config for BSC
    pub fn bsc(rpc_url: &str) -> Self {
        Self {
            chain_id: ChainId::BSC,
            name: "BNB Smart Chain".to_string(),
            rpc_url: rpc_url.to_string(),
            native_symbol: "BNB".to_string(),
            is_testnet: false,
            explorer_url: "https://bscscan.com".to_string(),
            eip1559: false,
            block_time_secs: 3,
        }
    }

    /// Create config for Sepolia testnet
    pub fn sepolia(rpc_url: &str) -> Self {
        Self {
            chain_id: ChainId::SEPOLIA,
            name: "Sepolia".to_string(),
            rpc_url: rpc_url.to_string(),
            native_symbol: "ETH".to_string(),
            is_testnet: true,
            explorer_url: "https://sepolia.etherscan.io".to_string(),
            eip1559: true,
            block_time_secs: 12,
        }
    }
}

/// EVM Chain implementation
pub struct EvmChain {
    config: EvmChainConfig,
    provider: alloy::providers::RootProvider<alloy::transports::http::Http<reqwest::Client>>,
}

impl EvmChain {
    /// Create a new EVM chain instance
    pub fn new(config: EvmChainConfig) -> Result<Self> {
        let url: reqwest::Url = config
            .rpc_url
            .parse()
            .map_err(|e| ChainError::RpcError(format!("Invalid RPC URL: {}", e)))?;
        let provider = alloy::providers::ProviderBuilder::new().on_http(url);

        Ok(Self { config, provider })
    }

    /// Get the provider
    pub fn provider(
        &self,
    ) -> &alloy::providers::RootProvider<alloy::transports::http::Http<reqwest::Client>> {
        &self.provider
    }

    /// ABI-encode an address as a 32-byte word (left-padded with zeros).
    fn abi_encode_address(addr: Address) -> [u8; 32] {
        let mut word = [0u8; 32];
        word[12..32].copy_from_slice(addr.as_slice());
        word
    }

    /// Parse EVM address
    fn parse_address(address: &str) -> Result<Address> {
        address
            .parse::<Address>()
            .map_err(|e| ChainError::InvalidAddress(e.to_string()))
    }

    /// Parse U256 from string
    fn parse_u256(value: &str) -> Result<U256> {
        value
            .parse::<U256>()
            .map_err(|e| ChainError::Internal(e.to_string()))
    }
}

#[async_trait]
impl Chain for EvmChain {
    fn chain_id(&self) -> ChainId {
        self.config.chain_id
    }

    fn name(&self) -> &str {
        &self.config.name
    }

    fn chain_type(&self) -> ChainType {
        ChainType::Evm
    }

    fn is_testnet(&self) -> bool {
        self.config.is_testnet
    }

    fn native_symbol(&self) -> &str {
        &self.config.native_symbol
    }

    fn explorer_url(&self) -> &str {
        &self.config.explorer_url
    }

    fn validate_address(&self, address: &str) -> Result<UnifiedAddress> {
        // Validate it's a proper EVM address
        let _ = Self::parse_address(address)?;
        UnifiedAddress::new(ChainType::Evm, address)
    }

    async fn get_balance(&self, address: &str) -> Result<Balance> {
        let addr = Self::parse_address(address)?;

        let balance = self
            .provider
            .get_balance(addr)
            .await
            .map_err(|e| ChainError::RpcError(e.to_string()))?;

        Ok(Balance {
            native: balance.to_string(),
            native_symbol: self.config.native_symbol.clone(),
            tokens: HashMap::new(),
        })
    }

    async fn get_token_balance(&self, address: &str, token_address: &str) -> Result<TokenBalance> {
        let addr = Self::parse_address(address)?;
        let token_addr = Self::parse_address(token_address)?;

        // ERC20 balanceOf(address) selector
        let mut calldata = Vec::with_capacity(36);
        calldata.extend_from_slice(&hex::decode("70a08231").unwrap()); // balanceOf selector
        calldata.extend_from_slice(&Self::abi_encode_address(addr));

        let call = alloy::rpc::types::TransactionRequest::default()
            .to(token_addr)
            .input(alloy::rpc::types::TransactionInput::new(Bytes::from(
                calldata,
            )));

        let result = self
            .provider
            .call(&call)
            .await
            .map_err(|e| ChainError::RpcError(e.to_string()))?;

        let result_bytes: &[u8] = result.as_ref();
        let balance = if result_bytes.len() >= 32 {
            U256::from_be_slice(&result_bytes[0..32])
        } else {
            U256::ZERO
        };

        // Get decimals
        let decimals_calldata = hex::decode("313ce567").unwrap(); // decimals selector
        let decimals_call = alloy::rpc::types::TransactionRequest::default()
            .to(token_addr)
            .input(alloy::rpc::types::TransactionInput::new(Bytes::from(
                decimals_calldata,
            )));

        let decimals_result = self
            .provider
            .call(&decimals_call)
            .await
            .map_err(|e| ChainError::RpcError(e.to_string()))?;

        let decimals_bytes: &[u8] = decimals_result.as_ref();
        let decimals = if decimals_bytes.len() >= 32 {
            decimals_bytes[31]
        } else {
            18 // Default to 18 decimals
        };

        // Get symbol
        let symbol_calldata = hex::decode("95d89b41").unwrap(); // symbol selector
        let symbol_call = alloy::rpc::types::TransactionRequest::default()
            .to(token_addr)
            .input(alloy::rpc::types::TransactionInput::new(Bytes::from(
                symbol_calldata,
            )));

        let symbol_result = self.provider.call(&symbol_call).await.unwrap_or_default();

        let symbol_bytes: &[u8] = symbol_result.as_ref();
        let symbol = if symbol_bytes.len() > 64 {
            // ABI encoded string
            let offset = U256::from_be_slice(&symbol_bytes[0..32]);
            let offset_usize: usize = offset.try_into().unwrap_or(0);
            if offset_usize < symbol_bytes.len() {
                let len = U256::from_be_slice(&symbol_bytes[offset_usize..offset_usize + 32]);
                let len_usize: usize = len.try_into().unwrap_or(0);
                let start = offset_usize + 32;
                if start + len_usize <= symbol_bytes.len() {
                    String::from_utf8_lossy(&symbol_bytes[start..start + len_usize])
                        .trim()
                        .to_string()
                } else {
                    "UNKNOWN".to_string()
                }
            } else {
                "UNKNOWN".to_string()
            }
        } else {
            "UNKNOWN".to_string()
        };

        Ok(TokenBalance {
            balance: balance.to_string(),
            symbol,
            decimals,
            contract_address: token_address.to_string(),
        })
    }

    async fn send_transaction(&self, tx: Transaction) -> Result<TxHash> {
        let from = Self::parse_address(&tx.from)?;
        let to = Self::parse_address(&tx.to)?;
        let value = Self::parse_u256(&tx.value)?;

        debug!(
            chain = %self.name(),
            from = %tx.from,
            to = %tx.to,
            value = %tx.value,
            "Preparing transaction"
        );

        let mut tx_request = alloy::rpc::types::TransactionRequest::default()
            .from(from)
            .to(to)
            .value(value);

        if let Some(data) = &tx.data {
            tx_request.input =
                alloy::rpc::types::TransactionInput::new(Bytes::copy_from_slice(data));
        }

        if let Some(gas_limit) = tx.gas_limit {
            tx_request.gas = Some(gas_limit as u128);
        }

        if let Some(nonce) = tx.nonce {
            tx_request.nonce = Some(nonce);
        }

        if self.config.eip1559 {
            if let Some(max_fee) = &tx.max_fee_per_gas {
                let max_fee_u256 = Self::parse_u256(max_fee)?;
                let max_fee_u128: u128 = max_fee_u256.try_into().unwrap_or(u128::MAX);
                tx_request.max_fee_per_gas = Some(max_fee_u128);
            }
        } else if let Some(gas_price) = &tx.gas_price {
            let gas_price_u256 = Self::parse_u256(gas_price)?;
            let gas_price_u128: u128 = gas_price_u256.try_into().unwrap_or(u128::MAX);
            tx_request.gas_price = Some(gas_price_u128);
        }

        // Note: In production, this would require a wallet/signer
        // For now, we send the raw transaction request via the provider
        let pending = self
            .provider
            .send_raw_transaction(&[]) // Placeholder - requires signed tx in production
            .await
            .map_err(|e| ChainError::TransactionFailed(e.to_string()))?;

        let hash = *pending.tx_hash();
        info!(
            chain = %self.name(),
            hash = %hash,
            "Transaction submitted"
        );

        Ok(TxHash(format!("{:?}", hash)))
    }

    async fn get_transaction(&self, hash: &str) -> Result<TxStatus> {
        let tx_hash = hash
            .parse::<B256>()
            .map_err(|e| ChainError::InvalidAddress(e.to_string()))?;

        let receipt = self
            .provider
            .get_transaction_receipt(tx_hash)
            .await
            .map_err(|e| ChainError::RpcError(e.to_string()))?;

        let current_block = self.get_block_number().await?;

        match receipt {
            Some(receipt) => {
                let status = if receipt.status() {
                    TxState::Confirmed
                } else {
                    TxState::Failed
                };

                let block_number = receipt.block_number;
                let confirmations = block_number
                    .map(|b| current_block.saturating_sub(b))
                    .unwrap_or(0);

                Ok(TxStatus {
                    hash: TxHash(hash.to_string()),
                    status,
                    block_number,
                    block_hash: receipt.block_hash.map(|h| format!("{:?}", h)),
                    confirmations,
                    gas_used: Some(receipt.gas_used.to_string()),
                    effective_gas_price: Some(receipt.effective_gas_price.to_string()),
                    error_message: None,
                })
            }
            None => {
                // Check if transaction is pending
                let tx = self
                    .provider
                    .get_transaction_by_hash(tx_hash)
                    .await
                    .map_err(|e| ChainError::RpcError(e.to_string()))?;

                if tx.is_some() {
                    Ok(TxStatus {
                        hash: TxHash(hash.to_string()),
                        status: TxState::Pending,
                        block_number: None,
                        block_hash: None,
                        confirmations: 0,
                        gas_used: None,
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
        }
    }

    async fn wait_for_confirmation(
        &self,
        hash: &TxHash,
        confirmations: u64,
        timeout_secs: u64,
    ) -> Result<TxStatus> {
        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(timeout_secs);
        let poll_interval = Duration::from_secs(self.config.block_time_secs.max(1));

        loop {
            if start.elapsed() > timeout {
                return Err(ChainError::Timeout);
            }

            let status = self.get_transaction(&hash.0).await?;

            match status.status {
                TxState::Confirmed if status.confirmations >= confirmations => {
                    return Ok(status);
                }
                TxState::Failed => {
                    return Ok(status);
                }
                TxState::NotFound => {
                    warn!(hash = %hash, "Transaction not found, may have been dropped");
                }
                _ => {
                    debug!(
                        hash = %hash,
                        confirmations = status.confirmations,
                        required = confirmations,
                        "Waiting for confirmations"
                    );
                }
            }

            tokio::time::sleep(poll_interval).await;
        }
    }

    async fn estimate_fee(&self, tx: &Transaction) -> Result<FeeEstimate> {
        let from = Self::parse_address(&tx.from)?;
        let to = Self::parse_address(&tx.to)?;
        let value = Self::parse_u256(&tx.value)?;

        let mut tx_request = alloy::rpc::types::TransactionRequest::default()
            .from(from)
            .to(to)
            .value(value);

        if let Some(data) = &tx.data {
            tx_request.input =
                alloy::rpc::types::TransactionInput::new(Bytes::copy_from_slice(data));
        }

        // Estimate gas
        let gas_estimate = self
            .provider
            .estimate_gas(&tx_request)
            .await
            .map_err(|e| ChainError::RpcError(e.to_string()))?;

        let gas_units: u64 = gas_estimate.try_into().unwrap_or(u64::MAX);

        // Get current gas price
        let gas_price = self
            .provider
            .get_gas_price()
            .await
            .map_err(|e| ChainError::RpcError(e.to_string()))?;

        // Calculate fee options (slow: 0.8x, standard: 1x, fast: 1.5x)
        let slow_price = gas_price * 80 / 100;
        let standard_price = gas_price;
        let fast_price = gas_price * 150 / 100;

        let slow_total = slow_price * gas_estimate as u128;
        let standard_total = standard_price * gas_estimate as u128;
        let fast_total = fast_price * gas_estimate as u128;

        Ok(FeeEstimate {
            gas_units,
            slow: FeeOption {
                price: slow_price.to_string(),
                max_fee: None,
                priority_fee: None,
                total_cost: slow_total.to_string(),
                estimated_time_seconds: self.config.block_time_secs * 3,
            },
            standard: FeeOption {
                price: standard_price.to_string(),
                max_fee: None,
                priority_fee: None,
                total_cost: standard_total.to_string(),
                estimated_time_seconds: self.config.block_time_secs * 2,
            },
            fast: FeeOption {
                price: fast_price.to_string(),
                max_fee: None,
                priority_fee: None,
                total_cost: fast_total.to_string(),
                estimated_time_seconds: self.config.block_time_secs,
            },
        })
    }

    async fn get_block_number(&self) -> Result<u64> {
        let block = self
            .provider
            .get_block_number()
            .await
            .map_err(|e| ChainError::RpcError(e.to_string()))?;

        Ok(block)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evm_chain_config() {
        let config = EvmChainConfig::ethereum("https://eth.example.com");
        assert_eq!(config.chain_id, ChainId::ETHEREUM);
        assert_eq!(config.native_symbol, "ETH");
        assert!(!config.is_testnet);
        assert!(config.eip1559);
    }

    #[test]
    fn test_chain_configs() {
        let arbitrum = EvmChainConfig::arbitrum("https://arb.example.com");
        assert_eq!(arbitrum.chain_id, ChainId::ARBITRUM);

        let base = EvmChainConfig::base("https://base.example.com");
        assert_eq!(base.chain_id, ChainId::BASE);

        let polygon = EvmChainConfig::polygon("https://polygon.example.com");
        assert_eq!(polygon.native_symbol, "MATIC");
    }

    #[test]
    fn test_parse_address() {
        let valid = EvmChain::parse_address("0x1234567890123456789012345678901234567890");
        assert!(valid.is_ok());

        let invalid = EvmChain::parse_address("invalid");
        assert!(invalid.is_err());
    }
}
