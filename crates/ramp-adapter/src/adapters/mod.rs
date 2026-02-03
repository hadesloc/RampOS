//! Example adapter implementations

pub mod mock;
pub mod napas;
pub mod vietqr;

pub use mock::MockAdapter;
pub use napas::NapasAdapter;
pub use vietqr::VietQRAdapter;
