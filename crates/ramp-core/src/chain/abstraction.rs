//! Chain Abstraction Layer
//!
//! Provides a high-level service for interacting with multiple blockchains
//! through a unified interface.
//!
//! Features:
//! - Multi-chain support (EVM, Solana, TON)
//! - Unified address format
//! - Unified transaction model
//! - Automatic fee estimation
//! - Transaction tracking
//! - Balance normalization

use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn};

use super::{
    Chain, ChainError, ChainId, ChainRegistry, ChainType, EvmChain, EvmChainConfig,
    SolanaChain, SolanaChainConfig, TonChain, TonChainConfig, Transaction, TxHash,
    TxStatus, UnifiedAddress, Balance, TokenBalance, FeeEstimate, Result
};

/// Service for unified chain interactions
pub struct ChainAbstractionLayer {
    registry: ChainRegistry,
}

impl ChainAbstractionLayer {
    /// Create a new chain abstraction layer
    pub fn new() -> Self {
        Self {
            registry: ChainRegistry::new(),
        }
    }

    /// Register a chain
    pub fn register(&mut self, chain: Arc<dyn Chain>) {
        self.registry.register(chain);
    }

    /// Initialize with default chains
    pub fn with_defaults(rpc_map: HashMap<ChainId, String>) -> Result<Self> {
        let mut cal = Self::new();

        // Ethereum Mainnet
        if let Some(rpc) = rpc_map.get(&ChainId::ETHEREUM) {
            let config = EvmChainConfig::ethereum(rpc);
            let chain = EvmChain::new(config)?;
            cal.register(Arc::new(chain));
        }

        // Arbitrum
        if let Some(rpc) = rpc_map.get(&ChainId::ARBITRUM) {
            let config = EvmChainConfig::arbitrum(rpc);
            let chain = EvmChain::new(config)?;
            cal.register(Arc::new(chain));
        }

        // Base
        if let Some(rpc) = rpc_map.get(&ChainId::BASE) {
            let config = EvmChainConfig::base(rpc);
            let chain = EvmChain::new(config)?;
            cal.register(Arc::new(chain));
        }

        // Optimism
        if let Some(rpc) = rpc_map.get(&ChainId::OPTIMISM) {
            let config = EvmChainConfig::optimism(rpc);
            let chain = EvmChain::new(config)?;
            cal.register(Arc::new(chain));
        }

        // Polygon
        if let Some(rpc) = rpc_map.get(&ChainId::POLYGON) {
            let config = EvmChainConfig::polygon(rpc);
            let chain = EvmChain::new(config)?;
            cal.register(Arc::new(chain));
        }

        // Solana
        if let Some(rpc) = rpc_map.get(&ChainId::SOLANA_MAINNET) {
            let config = SolanaChainConfig::mainnet(rpc);
            let chain = SolanaChain::new(config)?;
            cal.register(Arc::new(chain));
        }

        // TON
        if let Some(rpc) = rpc_map.get(&ChainId::TON_MAINNET) {
            let config = TonChainConfig::mainnet(rpc);
            let chain = TonChain::new(config)?;
            cal.register(Arc::new(chain));
        }

        Ok(cal)
    }

    /// Get a chain by ID
    pub fn get_chain(&self, chain_id: ChainId) -> Result<Arc<dyn Chain>> {
        self.registry.get_or_error(chain_id)
    }

    /// Validate an address on a specific chain
    pub fn validate_address(&self, chain_id: ChainId, address: &str) -> Result<UnifiedAddress> {
        let chain = self.get_chain(chain_id)?;
        chain.validate_address(address)
    }

    /// Get balance on a specific chain
    pub fn get_balance<'a>(&'a self, chain_id: ChainId, address: &'a str) -> Result<impl std::future::Future<Output = Result<Balance>> + 'a> {
        let chain = self.get_chain(chain_id)?;
        Ok(async move { chain.get_balance(address).await })
    }

    // Helper to run async call immediately if needed or return future
    // Since the trait returns futures, we just propagate them

    /// Get token balance
    pub async fn get_token_balance(
        &self,
        chain_id: ChainId,
        address: &str,
        token_address: &str,
    ) -> Result<TokenBalance> {
        let chain = self.get_chain(chain_id)?;
        chain.get_token_balance(address, token_address).await
    }

    /// Send a transaction
    pub async fn send_transaction(&self, chain_id: ChainId, tx: Transaction) -> Result<TxHash> {
        let chain = self.get_chain(chain_id)?;
        chain.send_transaction(tx).await
    }

    /// Get transaction status
    pub async fn get_transaction_status(&self, chain_id: ChainId, hash: &str) -> Result<TxStatus> {
        let chain = self.get_chain(chain_id)?;
        chain.get_transaction(hash).await
    }

    /// Estimate transaction fee
    pub async fn estimate_fee(&self, chain_id: ChainId, tx: &Transaction) -> Result<FeeEstimate> {
        let chain = self.get_chain(chain_id)?;
        chain.estimate_fee(tx).await
    }

    /// Get all supported chains
    pub fn supported_chains(&self) -> Vec<Arc<dyn Chain>> {
        self.registry.list()
    }
}
