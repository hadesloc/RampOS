//! API Versioning Module - Stripe-style date-based API versioning
//!
//! This module provides:
//! - `ApiVersion`: A date-based version identifier (e.g., `2026-02-01`)
//! - `VersionTransformer`: A trait for transforming payloads between versions
//! - `TransformerRegistry`: A pipeline that chains transformers for multi-version jumps
//!
//! # How it works
//!
//! 1. The client sends a request with the `RampOS-Version` header (e.g., `2026-02-01`).
//! 2. If no header is present, the tenant's pinned version is used.
//! 3. If the tenant has no pinned version, the default version is used.
//! 4. The version negotiation middleware injects the resolved `ApiVersion` into
//!    request extensions so handlers can read it.
//! 5. The response transformation layer can downgrade responses to match the
//!    client's expected format using the `TransformerRegistry`.

pub mod transformers;
pub mod version;

pub use transformers::{TransformError, TransformerRegistry, VersionTransformer};
pub use version::{ApiVersion, ApiVersionError};
