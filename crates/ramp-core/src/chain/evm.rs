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

use alloy::primitives::{keccak256, Address, Bytes, B256, U256};
use alloy::providers::Provider;
use alloy::rpc::types::Filter;
use async_trait::async_trait;
use ramp_common::onchain_gate::{
    experimental_onchain_execution_disabled_message, experimental_onchain_execution_enabled,
};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, warn};

use super::{
    Balance, Chain, ChainError, ChainId, ChainType, FeeEstimate, FeeOption, InboundTransferQuery,
    NativeInboundTransferQuery, ObservedInboundTransfer, Result, TokenBalance, Transaction, TxHash,
    TxState, TxStatus, UnifiedAddress,
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

    /// Create config for Avalanche C-Chain
    pub fn avalanche(rpc_url: &str) -> Self {
        Self {
            chain_id: ChainId::AVALANCHE,
            name: "Avalanche C-Chain".to_string(),
            rpc_url: rpc_url.to_string(),
            native_symbol: "AVAX".to_string(),
            is_testnet: false,
            explorer_url: "https://snowtrace.io".to_string(),
            eip1559: true,
            block_time_secs: 2,
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
    const ERC20_TRANSFER_EVENT: &'static str = "Transfer(address,address,uint256)";

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

    fn parse_transfer_log(log: alloy::rpc::types::Log) -> Result<ObservedInboundTransfer> {
        let topics = log.topics();
        if topics.len() < 3 {
            return Err(ChainError::Internal(
                "ERC20 transfer log missing indexed topics".to_string(),
            ));
        }

        let tx_hash = log.transaction_hash.ok_or_else(|| {
            ChainError::Internal("Transfer log missing transaction hash".to_string())
        })?;
        let block_number = log
            .block_number
            .ok_or_else(|| ChainError::Internal("Transfer log missing block number".to_string()))?;
        let amount = log.data().data.as_ref();
        if amount.len() < 32 {
            return Err(ChainError::Internal(
                "ERC20 transfer log missing amount payload".to_string(),
            ));
        }

        Ok(ObservedInboundTransfer {
            tx_hash: TxHash(format!("{:#x}", tx_hash)),
            from_address: format!("{:#x}", Self::topic_to_address(&topics[1])?),
            to_address: format!("{:#x}", Self::topic_to_address(&topics[2])?),
            amount: U256::from_be_slice(&amount[..32]).to_string(),
            block_number,
        })
    }

    fn topic_to_address(topic: &B256) -> Result<Address> {
        let bytes = topic.as_slice();
        if bytes.len() != 32 {
            return Err(ChainError::Internal(
                "Invalid ERC20 indexed topic width".to_string(),
            ));
        }
        Ok(Address::from_slice(&bytes[12..]))
    }

    fn parse_native_transaction(tx: &alloy::rpc::types::Transaction) -> ObservedInboundTransfer {
        ObservedInboundTransfer {
            tx_hash: TxHash(format!("{:#x}", tx.hash)),
            from_address: format!("{:#x}", tx.from),
            to_address: tx
                .to
                .map(|address| format!("{:#x}", address))
                .unwrap_or_else(|| "0x0000000000000000000000000000000000000000".to_string()),
            amount: tx.value.to_string(),
            block_number: tx.block_number.unwrap_or_default(),
        }
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

        // EXPERIMENTAL — not launch scope (D-03). Fail-closed: raw EVM transaction
        // submission is not implemented and never submits an empty placeholder payload.
        if !experimental_onchain_execution_enabled() {
            return Err(ChainError::NotSupported(
                experimental_onchain_execution_disabled_message(
                    "EVM on-chain transaction submission",
                ),
            ));
        }

        Err(ChainError::NotSupported(
            "EVM on-chain transaction submission is experimental and not implemented — signed raw transaction submission required (D-03)".to_string(),
        ))
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

    async fn find_inbound_transfers(
        &self,
        query: &InboundTransferQuery,
    ) -> Result<Vec<ObservedInboundTransfer>> {
        let token_addr = Self::parse_address(&query.token_address)?;
        let to_addr = Self::parse_address(&query.to_address)?;
        let transfer_signature = keccak256(Self::ERC20_TRANSFER_EVENT.as_bytes());
        let recipient_topic = B256::from(Self::abi_encode_address(to_addr));
        let filter = Filter::new()
            .address(token_addr)
            .event_signature(transfer_signature)
            .topic2(recipient_topic)
            .from_block(query.from_block)
            .to_block(query.to_block);

        let logs = self
            .provider
            .get_logs(&filter)
            .await
            .map_err(|e| ChainError::RpcError(e.to_string()))?;

        logs.into_iter()
            .map(Self::parse_transfer_log)
            .collect::<Result<Vec<_>>>()
    }

    async fn find_native_inbound_transfers(
        &self,
        query: &NativeInboundTransferQuery,
    ) -> Result<Vec<ObservedInboundTransfer>> {
        let to_addr = Self::parse_address(&query.to_address)?;
        let mut transfers = Vec::new();

        for block_number in query.from_block..=query.to_block {
            let Some(block) = self
                .provider
                .get_block_by_number(block_number.into(), true)
                .await
                .map_err(|e| ChainError::RpcError(e.to_string()))?
            else {
                continue;
            };

            for tx in block.transactions.txns() {
                if tx.to != Some(to_addr) || tx.value.is_zero() {
                    continue;
                }
                transfers.push(Self::parse_native_transaction(tx));
            }
        }

        Ok(transfers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ramp_common::onchain_gate::{test_env_lock, EXPERIMENTAL_ONCHAIN_EXECUTION_ENV};

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

        let avalanche = EvmChainConfig::avalanche("https://avax.example.com");
        assert_eq!(avalanche.chain_id, ChainId::AVALANCHE);
        assert_eq!(avalanche.native_symbol, "AVAX");
    }

    #[test]
    fn test_parse_address() {
        let valid = EvmChain::parse_address("0x1234567890123456789012345678901234567890");
        assert!(valid.is_ok());

        let invalid = EvmChain::parse_address("invalid");
        assert!(invalid.is_err());
    }

    #[test]
    fn test_topic_to_address() {
        let address =
            Address::parse_checksummed("0x1234567890123456789012345678901234567890", None).unwrap();
        let topic = B256::from(EvmChain::abi_encode_address(address));
        let parsed = EvmChain::topic_to_address(&topic).unwrap();
        assert_eq!(parsed, address);
    }

    #[tokio::test]
    async fn test_send_transaction_fails_closed_when_gate_off() {
        let _guard = test_env_lock();
        std::env::remove_var(EXPERIMENTAL_ONCHAIN_EXECUTION_ENV);

        let chain = EvmChain::new(EvmChainConfig::ethereum("https://eth.example.com")).unwrap();
        let tx = Transaction {
            from: "0x1111111111111111111111111111111111111111".to_string(),
            to: "0x2222222222222222222222222222222222222222".to_string(),
            value: "1".to_string(),
            data: None,
            gas_limit: None,
            gas_price: None,
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            nonce: None,
        };

        let err = chain
            .send_transaction(tx)
            .await
            .expect_err("EVM send_transaction should fail closed by default");
        assert!(err
            .to_string()
            .contains("EVM on-chain transaction submission"));
        assert!(err.to_string().contains("experimental and disabled"));
    }

    #[tokio::test]
    async fn test_send_transaction_errors_even_with_gate_enabled() {
        let _guard = test_env_lock();
        std::env::set_var(EXPERIMENTAL_ONCHAIN_EXECUTION_ENV, "true");

        let chain = EvmChain::new(EvmChainConfig::ethereum("https://eth.example.com")).unwrap();
        let tx = Transaction {
            from: "0x1111111111111111111111111111111111111111".to_string(),
            to: "0x2222222222222222222222222222222222222222".to_string(),
            value: "1".to_string(),
            data: None,
            gas_limit: None,
            gas_price: None,
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            nonce: None,
        };

        let err = chain
            .send_transaction(tx)
            .await
            .expect_err("EVM send_transaction must not submit placeholder payloads");
        assert!(err.to_string().contains("not implemented"));
        assert!(err
            .to_string()
            .contains("signed raw transaction submission required"));

        std::env::remove_var(EXPERIMENTAL_ONCHAIN_EXECUTION_ENV);
    }

    #[test]
    fn test_parse_transfer_log() {
        let from =
            Address::parse_checksummed("0x1111111111111111111111111111111111111111", None).unwrap();
        let to =
            Address::parse_checksummed("0x2222222222222222222222222222222222222222", None).unwrap();
        let transfer_signature = keccak256(EvmChain::ERC20_TRANSFER_EVENT.as_bytes());
        let log = alloy::rpc::types::Log {
            inner: alloy::primitives::Log::new_unchecked(
                Address::ZERO,
                vec![
                    transfer_signature,
                    B256::from(EvmChain::abi_encode_address(from)),
                    B256::from(EvmChain::abi_encode_address(to)),
                ],
                Bytes::from(U256::from(100_000_000u64).to_be_bytes_vec()),
            ),
            block_hash: None,
            block_number: Some(12345),
            block_timestamp: None,
            transaction_hash: Some(B256::repeat_byte(0xabu8)),
            transaction_index: None,
            log_index: None,
            removed: false,
        };

        let parsed = EvmChain::parse_transfer_log(log).unwrap();
        assert_eq!(
            parsed.tx_hash.0,
            format!("{:#x}", B256::repeat_byte(0xabu8))
        );
        assert_eq!(parsed.from_address, format!("{:#x}", from));
        assert_eq!(parsed.to_address, format!("{:#x}", to));
        assert_eq!(parsed.amount, "100000000");
        assert_eq!(parsed.block_number, 12345);
    }

    #[test]
    fn test_parse_native_transaction() {
        let tx = alloy::rpc::types::Transaction {
            hash: B256::repeat_byte(0xcdu8),
            nonce: 7,
            block_hash: Some(B256::repeat_byte(0x11)),
            block_number: Some(456),
            transaction_index: Some(0),
            from: Address::parse_checksummed("0x1111111111111111111111111111111111111111", None)
                .unwrap(),
            to: Some(
                Address::parse_checksummed("0x2222222222222222222222222222222222222222", None)
                    .unwrap(),
            ),
            value: U256::from(1_500_000_000_000_000_000u128),
            gas_price: Some(1),
            gas: 21_000,
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            max_fee_per_blob_gas: None,
            input: Bytes::new(),
            signature: None,
            chain_id: Some(1),
            blob_versioned_hashes: None,
            access_list: None,
            transaction_type: Some(0),
            other: Default::default(),
        };

        let parsed = EvmChain::parse_native_transaction(&tx);
        assert_eq!(parsed.tx_hash.0, format!("{:#x}", tx.hash));
        assert_eq!(parsed.from_address, format!("{:#x}", tx.from));
        assert_eq!(parsed.to_address, format!("{:#x}", tx.to.unwrap()));
        assert_eq!(parsed.amount, "1500000000000000000");
        assert_eq!(parsed.block_number, 456);
    }
}
