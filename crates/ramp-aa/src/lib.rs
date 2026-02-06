//! RampOS AA Kit - Account Abstraction (ERC-4337) Support
//!
//! Also includes EIP-7702 for EOA smart account delegation.

pub mod bundler;
pub mod eip7702;
pub mod gas;
pub mod paymaster;
pub mod policy;
pub mod smart_account;
pub mod types;
pub mod user_operation;

pub use bundler::BundlerClient;
pub use eip7702::{
    Authorization, AuthorizationList, Delegation, DelegationManager, Eip7702Config,
    Eip7702Transaction, Eip7702TxBuilder, SessionDelegation, SignedAuthorization,
};
pub use gas::{GasEstimate, GasEstimator};
pub use paymaster::{
    CrossChainGasQuote, CrossChainPaymaster, CrossChainPaymasterConfig, GasQuote, GasToken,
    MultiTokenPaymaster, MultiTokenPaymasterConfig, Paymaster, PaymasterService, PriceOracle,
    SponsorshipPolicy, SupportedChain, TenantGasLimits,
};
pub use smart_account::SmartAccountService;
pub use types::*;
pub use user_operation::UserOperation;
