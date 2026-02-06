//! RampOS Core - Business logic and orchestration
//!
//! This crate contains the core business logic for RampOS:
//! - Configuration management
//! - Repository implementations (PostgreSQL)
//! - Service layer (Payin, Payout, Trade, Ledger, Webhook)
//! - State machine definitions
//! - Event publishing (NATS)
//! - Workflow definitions and Temporal worker
//! - Multi-stablecoin support (USDT, USDC, DAI, VNST)
//! - Yield integrations (Aave V3, Compound V3)
//! - Cross-chain bridge support (Stargate, Across)
//! - Price oracles (Chainlink, CoinGecko)
//! - DEX aggregator swap engine (1inch, ParaSwap)
//! - Chain abstraction layer (EVM, Solana, TON)
//! - Cross-chain intent execution

pub mod bridge;
pub mod chain;
pub mod config;
pub mod crosschain;
pub mod event;
pub mod jobs;
pub mod oracle;
pub mod repository;
pub mod service;
pub mod stablecoin;
pub mod state_machine;
pub mod swap;
pub mod temporal_worker;
pub mod workflows;
pub mod r#yield;

pub mod test_utils;

pub use config::Config;
pub use stablecoin::{Stablecoin, StablecoinRegistry, TenantTokenConfig};
pub use temporal_worker::{TemporalWorker, TemporalWorkerConfig, WorkflowClient};
pub use r#yield::{YieldProtocol, YieldService, ProtocolRegistry, ProtocolId};
pub use bridge::{
    CrossChainBridge, BridgeRegistry, BridgeConfig, BridgeQuote, BridgeStatus,
    BridgeToken, BridgeTransfer, SupportedChain, StargateBridge, AcrossBridge,
};
pub use oracle::{
    ChainlinkOracle, CoinGeckoFallback, DepegAlert, DepegConfig, DepegLevel,
    OracleRegistry, Price, PriceOracle, PriceSource,
};
pub use swap::{
    DexAggregator, SwapQuote, SwapRouter, SwapService, Token as SwapToken,
    AggregatorRegistry, OneInchAggregator, ParaSwapAggregator,
};
pub use chain::{
    Chain, ChainId, ChainType, ChainRegistry, ChainInfo, ChainError,
    Balance, TokenBalance, Transaction, TxHash, TxStatus, TxState,
    FeeEstimate, FeeOption, UnifiedAddress,
    EvmChain, SolanaChain, TonChain,
};
pub use crosschain::{
    CrossChainIntent, IntentType, IntentStatus, IntentExecution,
    ExecutionStep, StepType, StepStatus, GasEstimate,
    IntentExecutor, ExecutionConfig, ExecutionResult, CrossChainExecutor,
    CrossChainRelayer, RelayerConfig, MessageStatus, CrossChainMessage,
};
