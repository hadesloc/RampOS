//! Stablecoin Yield Integration Module
//!
//! This module provides integrations with DeFi yield protocols for stablecoin deposits:
//! - Aave V3 for lending/borrowing
//! - Compound V3 for supply/borrow
//!
//! The yield service automatically selects the best APY protocol for deposits.
//! Strategy automation provides different risk profiles for yield management.

pub mod aave;
pub mod compound;
mod service;
pub mod strategy;
mod types;

pub use aave::AaveV3Protocol;
pub use compound::CompoundV3Protocol;
pub use service::{recommended_treasury_buffer_percent, YieldService};
pub use strategy::{
    AggressiveStrategy, BalancedStrategy, ConservativeStrategy, RiskLevel, StrategyConfig,
    StrategyId, StrategyManager, StrategyPerformance, YieldStrategy,
};
pub use types::*;

use alloy::primitives::{Address, B256, U256};
use async_trait::async_trait;
use ramp_common::Result;

/// Yield protocol trait for DeFi integrations
#[async_trait]
pub trait YieldProtocol: Send + Sync {
    /// Get the protocol name
    fn name(&self) -> &str;

    /// Get the protocol identifier
    fn protocol_id(&self) -> ProtocolId;

    /// Get current APY for a token (as percentage, e.g., 5.5 = 5.5%)
    async fn current_apy(&self, token: Address) -> Result<f64>;

    /// Deposit tokens into the yield protocol
    async fn deposit(&self, token: Address, amount: U256) -> Result<B256>;

    /// Withdraw tokens from the yield protocol
    async fn withdraw(&self, token: Address, amount: U256) -> Result<B256>;

    /// Get current balance in the protocol (including accrued yield)
    async fn balance(&self, token: Address) -> Result<U256>;

    /// Get accrued yield that hasn't been withdrawn
    async fn accrued_yield(&self, token: Address) -> Result<U256>;

    /// Claim any pending rewards (e.g., COMP, AAVE tokens)
    async fn claim_rewards(&self) -> Result<Option<B256>>;

    /// Check if the protocol supports a specific token
    fn supports_token(&self, token: Address) -> bool;

    /// Get the receipt token address (aToken, cToken) for a given token
    fn receipt_token(&self, token: Address) -> Option<Address>;

    /// Get protocol health factor (for safety monitoring)
    async fn health_factor(&self) -> Result<f64>;
}

/// Protocol registry for managing multiple yield protocols
pub struct ProtocolRegistry {
    protocols: Vec<Box<dyn YieldProtocol>>,
}

impl ProtocolRegistry {
    pub fn new() -> Self {
        Self {
            protocols: Vec::new(),
        }
    }

    pub fn register(&mut self, protocol: Box<dyn YieldProtocol>) {
        self.protocols.push(protocol);
    }

    pub fn get(&self, id: ProtocolId) -> Option<&dyn YieldProtocol> {
        self.protocols
            .iter()
            .find(|p| p.protocol_id() == id)
            .map(|p| p.as_ref())
    }

    pub fn all(&self) -> &[Box<dyn YieldProtocol>] {
        &self.protocols
    }

    /// Find the best protocol for a token based on APY
    pub async fn best_for_token(&self, token: Address) -> Result<Option<&dyn YieldProtocol>> {
        let mut best: Option<(&dyn YieldProtocol, f64)> = None;

        for protocol in &self.protocols {
            if !protocol.supports_token(token) {
                continue;
            }

            match protocol.current_apy(token).await {
                Ok(apy) => {
                    if best.is_none() || apy > best.as_ref().map(|(_, a)| *a).unwrap_or(0.0) {
                        best = Some((protocol.as_ref(), apy));
                    }
                }
                Err(_) => continue,
            }
        }

        Ok(best.map(|(p, _)| p))
    }
}

impl Default for ProtocolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_registry_new() {
        let registry = ProtocolRegistry::new();
        assert!(registry.all().is_empty());
    }
}
