//! Passkey signer module for WebAuthn P256 signature encoding
//!
//! Encodes passkey signatures for on-chain verification by the
//! RampOSAccount smart contract. Handles formatting P256 ECDSA
//! signatures as ERC-4337 UserOperation signature bytes.

pub mod signer;

pub use signer::PasskeySigner;
