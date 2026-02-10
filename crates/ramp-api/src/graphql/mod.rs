//! GraphQL API module for RampOS
//!
//! Provides a complete GraphQL API with:
//! - Query resolvers for intents, users, ledger entries, and dashboard stats
//! - Mutation resolvers for pay-in, pay-out operations
//! - Subscription resolvers for real-time intent status updates
//! - Cursor-based pagination (Relay Connection pattern)
//! - DataLoader for N+1 prevention
//!
//! # Usage
//!
//! ```rust,ignore
//! use ramp_api::graphql;
//!
//! let router = graphql::graphql_router(app_state);
//! // Mount at /graphql in your main router
//! ```

pub mod loaders;
pub mod mutation;
pub mod pagination;
pub mod query;
pub mod subscription;
pub mod types;

#[cfg(test)]
mod tests;

use async_graphql::http::GraphiQLSource;
use async_graphql::Schema;
use axum::{
    extract::State,
    response::{Html, IntoResponse},
    routing::get,
    Json, Router,
};
use std::sync::Arc;

use crate::router::AppState;
use mutation::MutationRoot;
use query::QueryRoot;
use subscription::{create_intent_event_channel, SubscriptionRoot};

/// The complete GraphQL schema type
pub type RampOsSchema = Schema<QueryRoot, MutationRoot, SubscriptionRoot>;

/// Build the GraphQL schema with all resolvers and context data from AppState.
pub fn build_schema(state: &AppState) -> RampOsSchema {
    let intent_event_sender = Arc::new(create_intent_event_channel());

    Schema::build(QueryRoot, MutationRoot, SubscriptionRoot)
        .data(state.intent_repo.clone())
        .data(state.payin_service.clone())
        .data(state.payout_service.clone())
        .data(state.user_service.clone())
        .data(intent_event_sender)
        .finish()
}

/// Handler for GraphQL requests (POST /graphql)
async fn graphql_handler(
    State(schema): State<RampOsSchema>,
    Json(request): Json<async_graphql::Request>,
) -> Json<async_graphql::Response> {
    Json(schema.execute(request).await)
}

/// Handler for GraphQL Playground (GET /graphql)
async fn graphql_playground() -> impl IntoResponse {
    Html(
        GraphiQLSource::build()
            .endpoint("/graphql")
            .subscription_endpoint("/graphql/ws")
            .finish(),
    )
}

/// Create a standalone GraphQL router that can be mounted on the main app.
///
/// This router provides:
/// - `POST /` - GraphQL query/mutation endpoint
/// - `GET /` - GraphiQL Playground UI
/// - `GET /playground` - GraphiQL Playground UI (alias)
///
/// # Example
///
/// ```rust,ignore
/// let gql_router = graphql::graphql_router(app_state);
/// let app = Router::new().nest("/graphql", gql_router);
/// ```
pub fn graphql_router(state: AppState) -> Router {
    let schema = build_schema(&state);
    graphql_router_from_schema(schema)
}

/// Create a GraphQL router from an already-built schema.
/// Useful for testing or when you need custom schema configuration.
pub fn graphql_router_from_schema(schema: RampOsSchema) -> Router {
    Router::new()
        .route("/", get(graphql_playground).post(graphql_handler))
        .route("/playground", get(graphql_playground))
        .with_state(schema)
}
