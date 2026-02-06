//! Example adapter implementations

pub mod ekyc;
pub mod mock;
pub mod napas;
pub mod vietqr;

pub use mock::MockAdapter;
pub use napas::NapasAdapter;
pub use vietqr::VietQRAdapter;

// eKYC provider exports
pub use ekyc::{
    EkycProvider, EkycProviderConfig, FptAiEkycProvider, MockEkycProvider, VnpayEkycProvider,
};
