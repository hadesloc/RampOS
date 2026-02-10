//! RampOS API Server

pub mod dto;
pub mod error;
pub mod extract;
pub mod graphql;
pub mod handlers;
pub mod middleware;
pub mod openapi;
pub mod providers;
pub mod router;
pub mod versioning;

pub use error::ApiError;
pub use router::{create_router, AppState};
