//! Example adapter implementations

pub mod mock;
pub mod vietqr;
pub mod napas;

pub use mock::MockAdapter;
pub use vietqr::VietQRAdapter;
pub use napas::NapasAdapter;
