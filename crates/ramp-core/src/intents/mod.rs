//! Chain Abstraction Protocol - Intent-based execution system
//!
//! This module provides a high-level intent-based system for executing
//! operations across multiple blockchains. Users express *what* they want
//! to achieve, and the system determines *how* to do it optimally.
//!
//! ## Architecture
//!
//! - **spec**: Intent DSL - declarative specification of user intents
//! - **solver**: Route finding and optimization
//! - **unified_balance**: Cross-chain balance aggregation
//! - **execution**: Step-by-step execution engine with state machine
//! - **backends**: Integration stubs for swap (1inch/ParaSwap) and bridge (Stargate/Across)
//!
//! ## Usage
//!
//! ```rust,ignore
//! use ramp_core::intents::{IntentSpec, IntentAction, AssetId, LocalSolver, ExecutionEngine};
//!
//! // 1. Create an intent
//! let spec = IntentSpec::new(
//!     IntentAction::Swap,
//!     AssetId::usdc(1),       // USDC on Ethereum
//!     AssetId::usdt(42161),   // USDT on Arbitrum
//!     "1000000",              // 1 USDC (6 decimals)
//! );
//!
//! // 2. Create engine with solver
//! let solver = Arc::new(LocalSolver::new());
//! let engine = ExecutionEngine::new(solver);
//!
//! // 3. Submit intent (solver finds optimal path)
//! let execution = engine.submit(spec).await?;
//!
//! // 4. Execute the plan
//! let result = engine.execute(&execution.id).await?;
//! ```

pub mod backends;
pub mod execution;
pub mod solver;
pub mod spec;
pub mod unified_balance;

// Re-export primary types for convenient access
pub use backends::{
    AcrossBackend, BackendRegistry, BridgeBackend, BridgeBackendQuote, BridgeTransferStatus,
    OneInchBackend, ParaSwapBackend, StargateBackend, SwapBackend, SwapBackendQuote,
};
pub use execution::{
    ExecutionEngine, ExecutionState, IntentExecution, ProgressEvent, ProgressEventType,
    StepExecutionStatus, StepRecord,
};
pub use solver::{IntentSolver, LocalSolver, RouteOption};
pub use spec::{
    AssetId, ExecutionPlan, ExecutionStepKind, IntentAction, IntentConstraints, IntentSpec,
    PlanStep, StepEstimate,
};
pub use unified_balance::{ChainBalance, ChainConfig, UnifiedBalance, UnifiedBalanceService};
