//! Portal API Handlers
//!
//! User-facing endpoints for the portal application including:
//! - Authentication (WebAuthn, Magic Link)
//! - KYC management
//! - Wallet operations
//! - Transaction history
//! - Deposit/Withdrawal intents

pub mod auth;
pub mod intents;
pub mod kyc;
pub mod settings;
pub mod transactions;
pub mod wallet;

use axum::Router;

use crate::router::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .nest("/auth", auth::router())
        .nest("/kyc", kyc::router())
        .nest("/wallet", wallet::router())
        .nest("/transactions", transactions::router())
        .nest("/intents", intents::router())
        .nest("/settings", settings::router())
}
