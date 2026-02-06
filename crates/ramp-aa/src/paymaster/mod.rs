//! Paymaster module - Gas sponsorship and payment abstraction
//!
//! This module provides:
//! - Base paymaster functionality (ERC-4337 compatible)
//! - Multi-token gas payment (USDT, USDC, DAI)
//! - Cross-chain gas sponsorship

mod base;
pub mod cross_chain;
pub mod multi_token;

pub use base::{Paymaster, PaymasterService, SponsorshipPolicy};
pub use cross_chain::{
    CrossChainGasQuote, CrossChainPaymaster, CrossChainPaymasterConfig,
    CrossChainPaymentInstruction, CrossChainRoute, LiquidityPool, LiquidityProvider,
    MockLiquidityProvider, SupportedChain,
};
pub use multi_token::{
    GasQuote, GasToken, MockPriceOracle, MultiTokenPaymaster, MultiTokenPaymasterConfig,
    PriceOracle, TenantGasLimits, TenantGasUsage, TokenApprovalStatus, TokenConfig,
};
