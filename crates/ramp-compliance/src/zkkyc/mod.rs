//! Zero-Knowledge KYC Module
//!
//! Provides privacy-preserving KYC verification using commitment schemes.
//! Users prove KYC status without revealing personal data.

pub mod credential;
pub mod service;

pub use credential::{ZkCredential, ZkCredentialIssuer};
pub use service::{
    VerificationResult, ZkKycProof, ZkKycProofRequest, ZkKycService,
};
