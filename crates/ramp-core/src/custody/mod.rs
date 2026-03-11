//! MPC-TSS Custody Solution
//!
//! Provides a 2-of-3 threshold MPC custody system with:
//! - Simulated Shamir Secret Sharing for key generation
//! - Threshold signing sessions with approval workflow
//! - Policy engine for transaction authorization
//!
//! NOTE: This uses simulated cryptography for service architecture validation.
//! A production deployment would integrate a real MPC-TSS library (e.g., multi-party-eddsa).

pub mod mpc_key;
pub mod mpc_signing;
pub mod policy;

pub use mpc_key::{KeyShare, MpcKeyGenResult, MpcKeyService};
pub use mpc_signing::{MpcSigningService, SigningRequest, SigningSession, SigningSessionStatus};
pub use policy::{CustodyPolicy, PolicyDecision, PolicyEngine, TimeRestriction};
