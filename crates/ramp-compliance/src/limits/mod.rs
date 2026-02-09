//! Transaction Limits Module
//!
//! Provides configurable transaction limits per KYC tier for VND transactions
//! following Vietnam SBV regulations.

mod vnd_limits;

pub use vnd_limits::{
    VndLimitChecker, VndLimitConfig, VndLimitDataProvider, VndLimitResult, VndTierLimits,
    VndUserLimitStatus,
};

pub use vnd_limits::MockVndLimitDataProvider;
