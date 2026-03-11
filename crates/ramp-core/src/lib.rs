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
//! - Chain abstraction protocol (intent-based)

pub mod billing;
pub mod bridge;
pub mod chain;
pub mod config;
pub mod crosschain;
pub mod custody;
pub mod domain;
pub mod event;
pub mod intents;
pub mod jobs;
pub mod oracle;
pub mod repository;
pub mod service;
pub mod sso;
pub mod stablecoin;
pub mod state_machine;
pub mod swap;
pub mod temporal_worker;
pub mod workflow_engine;
pub mod workflows;
pub mod r#yield;

pub mod test_utils;

pub use bridge::{
    AcrossBridge, BridgeConfig, BridgeQuote, BridgeRegistry, BridgeStatus, BridgeToken,
    BridgeTransfer, CrossChainBridge, StargateBridge, SupportedChain,
};
pub use chain::{
    Balance, Chain, ChainError, ChainId, ChainInfo, ChainRegistry, ChainType, EvmChain,
    FeeEstimate, FeeOption, SolanaChain, TokenBalance, TonChain, Transaction, TxHash, TxState,
    TxStatus, UnifiedAddress,
};
pub use config::Config;
pub use crosschain::{
    CrossChainExecutor, CrossChainIntent, CrossChainRelayer, ExecutionConfig, ExecutionResult,
    ExecutionStep, GasEstimate, IntentExecution, IntentExecutor, IntentStatus, IntentType,
    MessageStatus, RelayerConfig, StepStatus, StepType,
};
pub use oracle::{
    ChainlinkOracle, CoinGeckoFallback, DepegAlert, DepegConfig, DepegLevel, OracleRegistry, Price,
    PriceOracle, PriceSource,
};
pub use r#yield::{ProtocolId, ProtocolRegistry, YieldProtocol, YieldService};
pub use stablecoin::{Stablecoin, StablecoinRegistry, TenantTokenConfig};
pub use swap::{
    AggregatorRegistry, DexAggregator, OneInchAggregator, ParaSwapAggregator, SwapQuote,
    SwapRouter, SwapService, Token as SwapToken,
};
pub use temporal_worker::{TemporalWorker, TemporalWorkerConfig, WorkflowClient};
pub use workflow_engine::{
    create_workflow_engine, InProcessEngine, TemporalEngine, WorkflowEngine, WorkflowState,
    WorkflowStateRepository,
};
