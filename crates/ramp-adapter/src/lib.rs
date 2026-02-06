//! RampOS Adapter SDK - Bank/PSP Integration
//!
//! This crate provides adapters for integrating with Vietnam's banking infrastructure:
//!
//! ## Adapters
//!
//! - **VietQR** - QR code payment integration following VietQR/EMVCo standard
//! - **Napas** - National payment switch for bank transfers
//! - **Mock** - Testing adapter with configurable behavior
//!
//! ## Usage
//!
//! ```rust,ignore
//! use ramp_adapter::{AdapterFactory, create_adapters_from_env};
//!
//! // Create adapters from environment configuration
//! let adapters = create_adapters_from_env()?;
//!
//! // Or use the factory for custom configuration
//! let factory = AdapterFactory::new()?;
//! let adapter = factory.create_from_json("vietqr", config)?;
//! ```
//!
//! ## Implementing Custom Adapters
//!
//! Implement the `RailsAdapter` trait to add support for new banks/PSPs:
//!
//! ```rust,ignore
//! use ramp_adapter::{RailsAdapter, CreatePayinInstructionRequest, PayinInstruction};
//!
//! struct MyBankAdapter { /* ... */ }
//!
//! #[async_trait]
//! impl RailsAdapter for MyBankAdapter {
//!     // Implement required methods...
//! }
//! ```

pub mod adapters;
pub mod factory;
pub mod traits;
pub mod types;

// Re-export main types for convenience
pub use factory::{create_adapters_from_env, create_test_adapters, AdapterFactory};
pub use traits::{InstantTransferAdapter, QrCodeAdapter, RailsAdapter, VirtualAccountAdapter};
pub use types::*;

// Re-export adapter implementations
pub use adapters::{MockAdapter, NapasAdapter, VietQRAdapter};
