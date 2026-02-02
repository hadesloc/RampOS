//! RampOS API Server

pub mod dto;
pub mod error;
pub mod extract;
pub mod handlers;
pub mod middleware;
pub mod openapi;
pub mod router;

pub use error::ApiError;
pub use router::{create_router, AppState};
