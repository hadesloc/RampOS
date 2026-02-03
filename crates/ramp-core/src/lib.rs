//! RampOS Core - Business logic and orchestration
//!
//! This crate contains the core business logic for RampOS:
//! - Configuration management
//! - Repository implementations (PostgreSQL)
//! - Service layer (Payin, Payout, Trade, Ledger, Webhook)
//! - State machine definitions
//! - Event publishing (NATS)
//! - Workflow definitions and Temporal worker

pub mod config;
pub mod event;
pub mod jobs;
pub mod repository;
pub mod service;
pub mod state_machine;
pub mod temporal_worker;
pub mod workflows;

pub mod test_utils;

pub use config::Config;
pub use temporal_worker::{TemporalWorker, TemporalWorkerConfig, WorkflowClient};
