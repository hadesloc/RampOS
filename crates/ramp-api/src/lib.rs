//! RampOS API Server

pub mod handlers;
pub mod middleware;
pub mod dto;
pub mod error;
pub mod router;
pub mod extract;
pub mod openapi;

pub use router::{create_router, AppState};
pub use error::ApiError;
